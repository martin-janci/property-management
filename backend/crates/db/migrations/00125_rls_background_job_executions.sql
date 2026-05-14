-- Phase 0 — RLS Foundation Hardening (follow-up)
--
-- background_job_executions is the append-only execution-history child of
-- background_jobs (FK job_id). The parent background_jobs is tenant-scoped via
-- its nullable org_id column and already has RLS; the child had none. This
-- table was missed by the initial gap recon and caught by check-rls-coverage.sh.
--
-- The policy intentionally mirrors background_jobs' own security model rather
-- than the generic Phase-0 child template: background workers INSERT execution
-- rows via complete_background_job() / fail_background_job() while running with
-- the `app.is_system` context (and no org context), so the worker write path
-- must be allowed. ENABLE only (no FORCE) — consistent with the parent table
-- and with that worker path.

ALTER TABLE background_job_executions ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS background_job_executions_tenant_isolation ON background_job_executions;
CREATE POLICY background_job_executions_tenant_isolation ON background_job_executions
    FOR ALL
    USING (
        is_super_admin()
        OR COALESCE(NULLIF(current_setting('app.is_system', true), ''), 'false') = 'true'
        OR EXISTS (
            SELECT 1 FROM background_jobs bj
            WHERE bj.id = background_job_executions.job_id
              AND (bj.org_id IS NULL OR bj.org_id = get_current_org_id())
        )
    )
    WITH CHECK (
        is_super_admin()
        OR COALESCE(NULLIF(current_setting('app.is_system', true), ''), 'false') = 'true'
    );
