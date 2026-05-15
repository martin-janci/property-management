-- Phase 0: Retire the orphaned users.is_super_admin column.
--
-- The real super-admin mechanism is the JWT role (TenantRole::SuperAdmin /
-- PlatformAdmin) projected into the `app.is_super_admin` session GUC by the
-- RLS context plumbing. The `users.is_super_admin` table column (added in
-- 00071) was never wired into that flow and is the single remaining DB-fallback
-- path. This migration removes it.
--
-- Ordering matters: is_current_user_super_admin() must be rewritten to drop its
-- DB-fallback reference to users.is_super_admin BEFORE the column is dropped,
-- otherwise the DROP COLUMN would fail on the dependent function body.

-- 1. Rewrite is_current_user_super_admin() to be session-GUC-only.
--    The function NAME is preserved so the 9 existing RLS policy references
--    (00072_create_background_jobs.sql, 00079_create_distributed_tracing.sql,
--    00080_create_health_alert_rules.sql) keep working untouched. After this
--    rewrite it is functionally identical to is_super_admin().
CREATE OR REPLACE FUNCTION is_current_user_super_admin()
RETURNS BOOLEAN AS $$
BEGIN
    RETURN COALESCE(current_setting('app.is_super_admin', TRUE)::BOOLEAN, FALSE);
EXCEPTION
    WHEN OTHERS THEN
        RETURN FALSE;
END;
$$ LANGUAGE plpgsql STABLE;

COMMENT ON FUNCTION is_current_user_super_admin() IS 'Check if the current session user is a super admin. Reads only the app.is_super_admin session GUC (set from the JWT role); no database fallback.';

-- 2. Drop the index that depended on the column.
DROP INDEX IF EXISTS idx_users_is_super_admin;

-- 3. Drop the orphaned column itself.
ALTER TABLE users DROP COLUMN IF EXISTS is_super_admin;
