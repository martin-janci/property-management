-- Issue #2318 (findings 1 + 4, follow-up to PR #2313): give the report-schedule
-- due-work consumer a working RLS context + a due-work index.
--
-- Background
-- ----------
-- `report_schedules` / `report_executions` (00162) carry FORCE ROW LEVEL
-- SECURITY with a `is_super_admin() OR organization_id = get_current_org_id()`
-- policy, and the production api-server connects as the table OWNER — bound by
-- FORCE, no BYPASSRLS (00179). The background scheduler
-- (`api-server::services::scheduler::fire_due_report_schedules`, PR #2313) ran
-- its due-work SELECT on the bare pool with no GUC set, so
-- `get_current_org_id()` returned NULL, the policy evaluated false, and the
-- query **silently returned 0 rows**: scheduled reports never fired in prod.
--
-- Fix (mirrors 00136_listings_global_read_policy.sql)
-- ---------------------------------------------------
-- Add an `is_global_read_context()` SELECT-only leg so the context-less
-- worker can opt into the 4th RLS context (`set_global_read_context(TRUE)`)
-- for the cross-org due-work read. Policies for the same command are OR'd,
-- so a separate FOR SELECT policy extends the existing FOR ALL policy without
-- touching it.
--
-- Writes stay org-scoped: FOR SELECT policies have no WITH CHECK, so the
-- existing tenant-isolation policy remains the only INSERT/UPDATE/DELETE
-- gate. The scheduler performs its per-schedule writes (`record_execution`,
-- `advance_after_run`) under `set_tenant_context(schedule.organization_id)`.

-- ============================================================================
-- report_schedules: global-read SELECT leg for the scheduler worker.
-- ============================================================================
DROP POLICY IF EXISTS report_schedules_global_read ON report_schedules;

CREATE POLICY report_schedules_global_read ON report_schedules
    FOR SELECT
    USING (is_global_read_context());

COMMENT ON POLICY report_schedules_global_read ON report_schedules IS
    'Issue #2318: cross-org SELECT for context-less background workers '
    '(app.global_read = on). Read-only — writes remain gated by the '
    'org-scoped report_schedules_tenant_isolation policy.';

-- ============================================================================
-- report_executions: same leg, so the worker (and any future generator
-- worker consuming pending executions) can read execution rows cross-org.
-- ============================================================================
DROP POLICY IF EXISTS report_executions_global_read ON report_executions;

CREATE POLICY report_executions_global_read ON report_executions
    FOR SELECT
    USING (is_global_read_context());

COMMENT ON POLICY report_executions_global_read ON report_executions IS
    'Issue #2318: cross-org SELECT for context-less background workers '
    '(app.global_read = on). Read-only — writes remain gated by the '
    'org-scoped report_executions_tenant_isolation policy.';

-- ============================================================================
-- Due-work partial index (issue #2318, finding 4).
--
-- `get_due_schedules` filters
--   `WHERE is_active AND next_run_at IS NOT NULL AND next_run_at <= NOW()`
-- every 60s, but 00162 only indexed organization_id and status — a full-table
-- scan per tick. Mirrors `idx_workflow_schedules_next` (00047) and
-- `idx_automation_rules_next_run` (00064).
-- ============================================================================
CREATE INDEX IF NOT EXISTS idx_report_schedules_next
    ON report_schedules(next_run_at)
    WHERE is_active = TRUE AND next_run_at IS NOT NULL;
