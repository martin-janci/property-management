-- Epic 2A, Story 2A.2: Row-Level Security Policies
-- Enables RLS on tenant-scoped tables and creates policies

-- Enable RLS on organization_members (users can only see their own memberships)
ALTER TABLE organization_members ENABLE ROW LEVEL SECURITY;
-- Force RLS even for table owner (needed for tests running as postgres user)
ALTER TABLE organization_members FORCE ROW LEVEL SECURITY;

-- Policy: Users can see their own memberships
CREATE POLICY organization_members_select_own ON organization_members
    FOR SELECT
    USING (
        user_id = NULLIF(current_setting('app.current_user_id', TRUE), '')::UUID
        OR is_super_admin()
    );

-- Policy: Only org admins can insert members
CREATE POLICY organization_members_insert ON organization_members
    FOR INSERT
    WITH CHECK (
        organization_id = get_current_org_id()
        OR is_super_admin()
    );

-- Policy: Only org admins can update members in their org
CREATE POLICY organization_members_update ON organization_members
    FOR UPDATE
    USING (
        organization_id = get_current_org_id()
        OR is_super_admin()
    );

-- Policy: Only org admins can delete members in their org
CREATE POLICY organization_members_delete ON organization_members
    FOR DELETE
    USING (
        organization_id = get_current_org_id()
        OR is_super_admin()
    );

-- Enable RLS on roles (org-scoped)
ALTER TABLE roles ENABLE ROW LEVEL SECURITY;
-- Force RLS even for table owner (needed for tests running as postgres user)
ALTER TABLE roles FORCE ROW LEVEL SECURITY;

-- Policy: Users can see roles in their org (requires membership verification)
CREATE POLICY roles_select ON roles
    FOR SELECT
    USING (
        (
            organization_id = get_current_org_id()
            AND EXISTS (
                SELECT 1 FROM organization_members om
                WHERE om.organization_id = roles.organization_id
                  AND om.user_id = NULLIF(current_setting('app.current_user_id', TRUE), '')::UUID
            )
        )
        OR is_super_admin()
    );

-- Policy: Only org admins can manage roles
CREATE POLICY roles_insert ON roles
    FOR INSERT
    WITH CHECK (
        organization_id = get_current_org_id()
        OR is_super_admin()
    );

CREATE POLICY roles_update ON roles
    FOR UPDATE
    USING (
        organization_id = get_current_org_id()
        OR is_super_admin()
    )
    WITH CHECK (
        -- Cannot modify system roles
        NOT is_system
        OR is_super_admin()
    );

CREATE POLICY roles_delete ON roles
    FOR DELETE
    USING (
        (organization_id = get_current_org_id() AND NOT is_system)
        OR is_super_admin()
    );

-- Function to set user context for RLS
CREATE OR REPLACE FUNCTION set_user_context(user_id UUID)
RETURNS void AS $$
BEGIN
    PERFORM set_config('app.current_user_id', user_id::TEXT, FALSE);
END;
$$ LANGUAGE plpgsql;

-- Function to set super admin flag
CREATE OR REPLACE FUNCTION set_super_admin_context(is_admin BOOLEAN)
RETURNS void AS $$
BEGIN
    PERFORM set_config('app.is_super_admin', is_admin::TEXT, FALSE);
END;
$$ LANGUAGE plpgsql;

-- Combined function to set all context at once (called by middleware)
-- Note: set_config's third parameter controls LOCAL vs SESSION scope
-- FALSE = session-scoped (persists across transactions in the same connection)
-- TRUE = transaction-scoped (only valid within current transaction)
-- We use FALSE to ensure settings persist across queries in the same session
CREATE OR REPLACE FUNCTION set_request_context(
    p_org_id UUID,
    p_user_id UUID,
    p_is_super_admin BOOLEAN DEFAULT FALSE
)
RETURNS void AS $$
BEGIN
    PERFORM set_config('app.current_org_id', COALESCE(p_org_id::TEXT, ''), FALSE);
    PERFORM set_config('app.current_user_id', COALESCE(p_user_id::TEXT, ''), FALSE);
    PERFORM set_config('app.is_super_admin', COALESCE(p_is_super_admin::TEXT, 'false'), FALSE);
END;
$$ LANGUAGE plpgsql;

-- Function to clear request context (called after request)
CREATE OR REPLACE FUNCTION clear_request_context()
RETURNS void AS $$
BEGIN
    PERFORM set_config('app.current_org_id', '', FALSE);
    PERFORM set_config('app.current_user_id', '', FALSE);
    PERFORM set_config('app.is_super_admin', 'false', FALSE);
END;
$$ LANGUAGE plpgsql;

-- Note: The organizations table itself doesn't have RLS enabled
-- because it's not tenant-scoped (it IS the tenant).
-- Access control for organizations is done at the application level.

-- Create a view for checking RLS coverage (Story 2A.3)
CREATE OR REPLACE VIEW rls_coverage AS
SELECT
    schemaname,
    tablename,
    CASE WHEN rowsecurity THEN 'enabled' ELSE 'disabled' END as rls_status,
    CASE
        WHEN tablename IN ('organizations', 'users', 'email_verification_tokens',
                          'refresh_tokens', 'password_reset_tokens')
        THEN 'exempt'
        ELSE 'tenant_scoped'
    END as table_type
FROM pg_tables
WHERE schemaname = 'public'
  AND tablename NOT LIKE 'pg_%'
  AND tablename NOT LIKE '_sqlx%';

-- Create a function to validate all tenant-scoped tables have RLS
CREATE OR REPLACE FUNCTION validate_rls_coverage()
RETURNS TABLE(tablename NAME, has_rls BOOLEAN, needs_rls BOOLEAN) AS $$
BEGIN
    RETURN QUERY
    SELECT
        t.tablename,
        t.rowsecurity as has_rls,
        CASE
            -- app_feedback: user-scoped general feedback / bug reports, mirrors
            -- user_article_feedback; organization_id is an optional FK-less triage
            -- hint, not an access-control boundary — intentionally non-tenant / no RLS
            -- (migration 00246). Keep in sync with EXEMPT_TABLES in check-rls-coverage.sh.
            WHEN t.tablename IN ('organizations', 'users', 'email_verification_tokens',
                                'refresh_tokens', 'password_reset_tokens', 'roles',
                                'organization_members', 'app_feedback')
            THEN FALSE
            ELSE TRUE
        END as needs_rls
    FROM pg_tables t
    WHERE t.schemaname = 'public'
      AND t.tablename NOT LIKE 'pg_%'
      AND t.tablename NOT LIKE '_sqlx%'
      AND (
          -- Show tables that need RLS but don't have it
          (CASE WHEN t.tablename NOT IN ('organizations', 'users', 'email_verification_tokens',
                                         'refresh_tokens', 'password_reset_tokens', 'app_feedback')
           THEN TRUE ELSE FALSE END) = TRUE
          AND t.rowsecurity = FALSE
      );
END;
$$ LANGUAGE plpgsql;
