-- Phase 2: Identity Unification — POPULATE users from portal_users.
--
-- This migration ONLY POPULATES the unified `users` table from `portal_users`.
-- It does NOT drop `portal_users`, does NOT rewrite FKs that point at
-- `portal_users(id)`, and does NOT alter `portal_sessions`. Physical removal of
-- `portal_users` is a future cleanup phase.
--
-- Behavior:
--   * Non-collision: `portal_users.email` not present in `users` → insert a
--     new `users` row with `principal_kind='public'`, `portal_origin_id` set
--     to the source portal_user id, status='active' (portal users were
--     authenticated).
--   * Collision: `portal_users.email` already present in `users` (any kind) →
--     write a row to `user_merge_collisions` with status='pending' for human
--     review. NO automatic merge.
--
-- Idempotent: re-running the migration is a no-op for rows already merged
-- (UNIQUE on portal_origin_id) and for collisions already queued.

-- ---------------------------------------------------------------------------
-- 1) Add the back-pointer column.
-- ---------------------------------------------------------------------------

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS portal_origin_id UUID NULL;

-- The constraint conditional add: only if portal_users still exists.
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = 'public' AND table_name = 'portal_users'
    ) THEN
        IF NOT EXISTS (
            SELECT 1 FROM information_schema.table_constraints
            WHERE table_name = 'users' AND constraint_name = 'users_portal_origin_id_fkey'
        ) THEN
            ALTER TABLE users
                ADD CONSTRAINT users_portal_origin_id_fkey
                FOREIGN KEY (portal_origin_id) REFERENCES portal_users(id) ON DELETE SET NULL;
        END IF;
    END IF;
END $$;

CREATE UNIQUE INDEX IF NOT EXISTS idx_users_portal_origin_id_unique
    ON users (portal_origin_id) WHERE portal_origin_id IS NOT NULL;

COMMENT ON COLUMN users.portal_origin_id IS
    'Phase 2: back-pointer to the originating portal_users row when the unified user came from a portal_users merge. NULL for non-portal-origin users.';

-- ---------------------------------------------------------------------------
-- 2) Run the merge — only if portal_users still exists.
-- ---------------------------------------------------------------------------

DO $$
DECLARE
    v_collisions INTEGER := 0;
    v_inserted   INTEGER := 0;
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = 'public' AND table_name = 'portal_users'
    ) THEN
        RAISE NOTICE 'portal_users table not present, skipping merge';
        RETURN;
    END IF;

    -- (a) Queue collisions: every portal_user whose email already exists in
    --     `users` (case-insensitive). Skip portal_users we have already merged
    --     (portal_origin_id round-trip) and skip collisions already queued
    --     for the same source row.
    EXECUTE $sql$
        INSERT INTO user_merge_collisions (
            source_table, source_id, target_email, payload, status
        )
        SELECT 'portal_users',
               pu.id,
               pu.email,
               jsonb_build_object(
                   'portal_user_id',     pu.id,
                   'portal_user_email',  pu.email,
                   'portal_user_name',   pu.name,
                   'portal_user_provider', pu.provider,
                   'portal_user_pm_user_id', pu.pm_user_id,
                   'portal_user_created_at', pu.created_at,
                   'reason',             'email_already_in_users_table'
               ),
               'pending'
          FROM portal_users pu
          JOIN users u ON LOWER(u.email) = LOWER(pu.email)
         WHERE NOT EXISTS (
                 SELECT 1 FROM users u2 WHERE u2.portal_origin_id = pu.id
           )
           AND NOT EXISTS (
                 SELECT 1 FROM user_merge_collisions c
                  WHERE c.source_table = 'portal_users'
                    AND c.source_id    = pu.id
                    AND c.status       = 'pending'
           )
    $sql$;
    GET DIAGNOSTICS v_collisions = ROW_COUNT;

    -- (b) Insert non-colliding portal_users into users with principal_kind='public'.
    --     SSO-only portal users (password_hash IS NULL) get a sentinel that
    --     cannot validate as Argon2id — they must continue to use SSO.
    EXECUTE $sql$
        INSERT INTO users (
            email, password_hash, name, locale, status, email_verified_at,
            principal_kind, portal_origin_id, created_at, updated_at
        )
        SELECT pu.email,
               COALESCE(pu.password_hash, '!sso-only-no-password'),
               pu.name,
               CASE WHEN pu.locale IN ('sk','cs','de','en') THEN pu.locale ELSE 'en' END,
               'active',
               CASE WHEN pu.email_verified THEN COALESCE(pu.updated_at, NOW()) ELSE NULL END,
               'public',
               pu.id,
               pu.created_at,
               pu.updated_at
          FROM portal_users pu
         WHERE NOT EXISTS (
                 SELECT 1 FROM users u WHERE LOWER(u.email) = LOWER(pu.email)
           )
           AND NOT EXISTS (
                 SELECT 1 FROM users u2 WHERE u2.portal_origin_id = pu.id
           )
    $sql$;
    GET DIAGNOSTICS v_inserted = ROW_COUNT;

    RAISE NOTICE 'portal_users merge: % inserted, % collisions queued', v_inserted, v_collisions;
END $$;
