-- backend/scripts/init-template-db.sql
-- =============================================================================
-- ppt_dev_template: Postgres template database for worktree DB cloning.
-- =============================================================================
--
-- Purpose:
--   This script (re)builds ppt_dev_template, the source template for:
--     1. The shared default worktree DB (Phase 1, schema-search-path mode), and
--     2. Per-worktree dedicated DBs in Phase 3, cloned via:
--          CREATE DATABASE ppt_wt_<alias> TEMPLATE ppt_dev_template;
--
-- Idempotent: safe to re-run; existing template is dropped and recreated.
--
-- Usage (as ppt user against the postgres maintenance DB):
--   docker exec -i ppt-postgres psql -U ppt -d postgres \
--     < backend/scripts/init-template-db.sql
--
-- Schema source:
--   /docker-entrypoint-initdb.d/01-init.sql is mounted into the postgres
--   container by docker-compose.dev.yml from backend/scripts/init-db.sql.
--   It installs extensions and RLS helper functions only.
--
--   The actual table schema is owned by SQLx migrations under
--   backend/crates/db/migrations/. The expected workflow is:
--     1. Run this script to create the template DB skeleton (extensions + helpers).
--     2. Apply SQLx migrations against ppt_dev_template (e.g. by pointing
--        api-server at it once, or running `sqlx migrate run` directly).
--     3. Re-run the seed block at the bottom of this script to populate
--        minimal demo data (or run only the SEED section if migrations
--        already applied).
--
--   To make this script tolerant of being run before migrations are applied,
--   the seed INSERTs are wrapped in a DO block that no-ops if the target
--   tables do not yet exist.
-- =============================================================================

\set ON_ERROR_STOP on

-- ----------------------------------------------------------------------------
-- 1. Recreate the template database from a clean base.
-- ----------------------------------------------------------------------------
-- DROP requires no other connections to the target DB.
-- TEMPLATE template0 ensures locale/encoding match the cluster default and
-- bypasses the "ppt" template that may carry user data.
DROP DATABASE IF EXISTS ppt_dev_template;
CREATE DATABASE ppt_dev_template TEMPLATE template0;

-- ----------------------------------------------------------------------------
-- 2. Connect to the new template and load the canonical init script.
-- ----------------------------------------------------------------------------
\c ppt_dev_template

-- Loads extensions (uuid-ossp, pgcrypto) and RLS helper functions
-- (current_tenant_id, current_user_id). Same path used by Postgres at
-- container startup, so this works inside the ppt-postgres container.
\i /docker-entrypoint-initdb.d/01-init.sql

-- ----------------------------------------------------------------------------
-- 3. Minimal demo seed.
-- ----------------------------------------------------------------------------
-- Goal: a freshly-cloned worktree DB has 1 organization, 1 manager, 1 owner,
-- and 1 building so the UI shows non-empty states out of the box.
--
-- Notes on schema alignment (verified against migrations 00001-00102):
--   * `organizations` (no `tenants` table). Required: name, slug, contact_email.
--   * `users`. Required: email, password_hash, name. Status defaults to
--     'pending'; we set 'active' + email_verified_at = NOW() so the demo
--     accounts can log in immediately. Argon2 hash here is a non-functional
--     placeholder — operators must reset passwords via the api-server CLI
--     (`cargo run -p api-server --bin ppt-seed`) before login is possible.
--   * `organization_members` is the user<->org junction with `role_type` and
--     `role_id`. The `create_default_roles` trigger fires on INSERT INTO
--     organizations and seeds 8 system roles (incl. 'Manager', 'Owner').
--   * `buildings` requires organization_id, street, city, postal_code (no
--     single `address` column). RLS is enforced via `is_super_admin()` OR
--     `organization_id = get_current_org_id()`, so seed must run with
--     super-admin context set via set_request_context(NULL, NULL, TRUE).
--
-- Tolerance: if the SQLx migrations have not yet been applied to this
-- template DB, the to_regclass() guard makes the seed a no-op. In that case
-- the operator should apply migrations and re-run only the SEED section.
-- ----------------------------------------------------------------------------

DO $$
DECLARE
    v_org_id        UUID := '00000000-0000-0000-0000-000000000001';
    v_manager_id    UUID := '11111111-1111-1111-1111-111111111111';
    v_owner_id      UUID := '22222222-2222-2222-2222-222222222222';
    v_building_id   UUID := '33333333-3333-3333-3333-333333333333';
    -- Non-functional Argon2id placeholder. Real password reset via CLI.
    v_pw_hash       TEXT := '$argon2id$v=19$m=19456,t=2,p=1$cmVwbGFjZW1lcmVwbGFjZW0$cmVwbGFjZW1lcmVwbGFjZW1lcmVwbGFjZW1lcmVwbGFjZW0';
    v_manager_role  UUID;
    v_owner_role    UUID;
BEGIN
    -- Bail out early if schema not yet applied.
    IF to_regclass('public.organizations') IS NULL
       OR to_regclass('public.users')         IS NULL
       OR to_regclass('public.buildings')     IS NULL
       OR to_regclass('public.organization_members') IS NULL THEN
        RAISE NOTICE 'init-template-db: schema tables missing; skipping seed. '
                     'Apply SQLx migrations against ppt_dev_template, then '
                     're-run the seed block.';
        RETURN;
    END IF;

    -- Bypass RLS for seeding (also tolerant if set_request_context not yet
    -- defined — fall back to direct set_config calls).
    BEGIN
        PERFORM set_request_context(NULL, NULL, TRUE);
    EXCEPTION
        WHEN undefined_function THEN
            PERFORM set_config('app.is_super_admin', 'true', FALSE);
    END;

    -- Organization (the create_default_roles trigger seeds 8 system roles).
    INSERT INTO organizations (id, name, slug, contact_email, status, settings)
    VALUES (
        v_org_id,
        'Demo Property Management',
        'demo',
        'contact@demo-property.test',
        'active',
        '{}'::jsonb
    )
    ON CONFLICT (id) DO NOTHING;

    -- Resolve the auto-created system role IDs for this org.
    SELECT id INTO v_manager_role
    FROM roles
    WHERE organization_id = v_org_id AND name = 'Manager';

    SELECT id INTO v_owner_role
    FROM roles
    WHERE organization_id = v_org_id AND name = 'Owner';

    -- Manager user (verified + active so demo login works after pw reset).
    INSERT INTO users (
        id, email, password_hash, name, status, email_verified_at, locale
    ) VALUES (
        v_manager_id,
        'demo-manager@example.test',
        v_pw_hash,
        'Demo Manager',
        'active',
        NOW(),
        'en'
    )
    ON CONFLICT (id) DO NOTHING;

    -- Owner user.
    INSERT INTO users (
        id, email, password_hash, name, status, email_verified_at, locale
    ) VALUES (
        v_owner_id,
        'demo-owner@example.test',
        v_pw_hash,
        'Demo Owner',
        'active',
        NOW(),
        'en'
    )
    ON CONFLICT (id) DO NOTHING;

    -- Membership: manager.
    INSERT INTO organization_members (
        organization_id, user_id, role_id, role_type, status, joined_at
    ) VALUES (
        v_org_id, v_manager_id, v_manager_role, 'Manager', 'active', NOW()
    )
    ON CONFLICT (organization_id, user_id) DO NOTHING;

    -- Membership: owner.
    INSERT INTO organization_members (
        organization_id, user_id, role_id, role_type, status, joined_at
    ) VALUES (
        v_org_id, v_owner_id, v_owner_role, 'Owner', 'active', NOW()
    )
    ON CONFLICT (organization_id, user_id) DO NOTHING;

    -- Building (RLS-scoped to the demo organization).
    INSERT INTO buildings (
        id, organization_id, street, city, postal_code, country,
        name, total_floors, total_entrances, status
    ) VALUES (
        v_building_id,
        v_org_id,
        'Hlavna 1',
        'Bratislava',
        '811 01',
        'Slovakia',
        'Demo Building',
        4,
        1,
        'active'
    )
    ON CONFLICT (id) DO NOTHING;

    -- Always clear the elevated context, even on success.
    BEGIN
        PERFORM clear_request_context();
    EXCEPTION
        WHEN undefined_function THEN
            PERFORM set_config('app.is_super_admin', 'false', FALSE);
    END;
END
$$;

-- ----------------------------------------------------------------------------
-- 4. Mark the template as ready (deploy server checks this comment).
-- ----------------------------------------------------------------------------
COMMENT ON DATABASE ppt_dev_template IS 'ppt-deploy template, ready';
