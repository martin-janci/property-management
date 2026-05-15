-- Phase 2: Identity Unification — principal_kind discriminator on users.
--
-- Adds a `principal_kind` column to the unified `users` table so a single
-- identity row can be classified as one of:
--   * 'public'   — public portal user (formerly `portal_users`)
--   * 'staff'    — agency staff (the existing PM users)
--   * 'platform' — platform / super-admin principal
--
-- The column is the operational home for per-kind identity rules (password
-- policy, MFA, e-mail verification) and for authorization decisions made by
-- the new `RequestPrincipal` extractor.

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS principal_kind VARCHAR(20) NOT NULL DEFAULT 'staff'
        CHECK (principal_kind IN ('public', 'staff', 'platform'));

CREATE INDEX IF NOT EXISTS idx_users_principal_kind ON users(principal_kind);

-- Backfill: any existing user is presumed staff (default already covers this),
-- but if a `platform_admins` table exists, mark its users as platform.
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = 'public' AND table_name = 'platform_admins'
    ) THEN
        EXECUTE $sql$
            UPDATE users u
               SET principal_kind = 'platform'
              FROM platform_admins p
             WHERE p.user_id = u.id
        $sql$;
    END IF;
END $$;

COMMENT ON COLUMN users.principal_kind IS
    'Phase 2 identity discriminator: public (portal user) | staff (agency) | platform (super-admin). Never client-settable; mutated only via set_principal_kind() (see migration 00129).';
