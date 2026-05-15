-- Phase 5.5 follow-up: performance_portfolios RLS policies (added in 00100)
-- predate the Phase 0 `is_super_admin()` convention. They cast
-- `current_setting('app.current_user_id', true)::UUID` directly without
-- guarding against the empty-string value that `clear_request_context()`
-- writes when no user is set, which crashes any super-admin INSERT
-- (the Phase 0 RLS smoke test surfaces this on a fresh DB).
--
-- Bring the policies into line with the Phase 0 convention:
--   * `is_super_admin()` bypass on every policy.
--   * Read the GUC via `NULLIF(..., '')` so an empty value yields NULL.
-- The semantics for non-super-admin users are unchanged: they can only see
-- portfolios for orgs whose `organization_members` row references them.

DROP POLICY IF EXISTS perf_portfolios_select ON performance_portfolios;
CREATE POLICY perf_portfolios_select ON performance_portfolios
    FOR SELECT USING (
        is_super_admin()
        OR organization_id IN (
            SELECT organization_id FROM organization_members
            WHERE user_id = NULLIF(current_setting('app.current_user_id', true), '')::UUID
        )
    );

DROP POLICY IF EXISTS perf_portfolios_insert ON performance_portfolios;
CREATE POLICY perf_portfolios_insert ON performance_portfolios
    FOR INSERT WITH CHECK (
        is_super_admin()
        OR organization_id IN (
            SELECT organization_id FROM organization_members
            WHERE user_id = NULLIF(current_setting('app.current_user_id', true), '')::UUID
        )
    );

DROP POLICY IF EXISTS perf_portfolios_update ON performance_portfolios;
CREATE POLICY perf_portfolios_update ON performance_portfolios
    FOR UPDATE USING (
        is_super_admin()
        OR organization_id IN (
            SELECT organization_id FROM organization_members
            WHERE user_id = NULLIF(current_setting('app.current_user_id', true), '')::UUID
        )
    ) WITH CHECK (
        is_super_admin()
        OR organization_id IN (
            SELECT organization_id FROM organization_members
            WHERE user_id = NULLIF(current_setting('app.current_user_id', true), '')::UUID
        )
    );

DROP POLICY IF EXISTS perf_portfolios_delete ON performance_portfolios;
CREATE POLICY perf_portfolios_delete ON performance_portfolios
    FOR DELETE USING (
        is_super_admin()
        OR organization_id IN (
            SELECT organization_id FROM organization_members
            WHERE user_id = NULLIF(current_setting('app.current_user_id', true), '')::UUID
        )
    );
