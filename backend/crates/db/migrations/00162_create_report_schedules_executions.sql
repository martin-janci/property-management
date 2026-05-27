-- Epic 81: Report Schedule Management & Execution History (Story 81.1, 81.2)
--
-- Creates:
--   report_schedules  — configuration for recurring report delivery
--   report_executions — per-run audit log with status and download link
--
-- RLS: tenant-scoped via organization_id on report_schedules; child table
-- report_executions inherits isolation through the schedule FK.

-- ============================================================================
-- report_schedules
-- ============================================================================

CREATE TABLE IF NOT EXISTS report_schedules (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    report_id       UUID        NOT NULL,
    organization_id UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name            TEXT        NOT NULL,
    -- 'daily' | 'weekly' | 'monthly'
    frequency       TEXT        NOT NULL CHECK (frequency IN ('daily', 'weekly', 'monthly')),
    -- 0=Sunday … 6=Saturday, used when frequency = 'weekly'
    day_of_week     INT,
    -- 1-31, used when frequency = 'monthly'
    day_of_month    INT,
    -- HH:MM in the given timezone
    time            TEXT        NOT NULL DEFAULT '08:00',
    timezone        TEXT        NOT NULL DEFAULT 'UTC',
    -- 'pdf' | 'excel' | 'csv'
    format          TEXT        NOT NULL DEFAULT 'pdf',
    -- JSON array of recipient email addresses
    recipients      JSONB       NOT NULL DEFAULT '[]',
    is_active       BOOLEAN     NOT NULL DEFAULT TRUE,
    -- 'active' | 'paused' | 'inactive'
    status          TEXT        NOT NULL DEFAULT 'active'
                                CHECK (status IN ('active', 'paused', 'inactive')),
    last_run_at     TIMESTAMPTZ,
    next_run_at     TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Lookup all schedules for a tenant.
CREATE INDEX IF NOT EXISTS idx_report_schedules_org
    ON report_schedules(organization_id);

-- Quick filter on active/paused schedules.
CREATE INDEX IF NOT EXISTS idx_report_schedules_status
    ON report_schedules(status);

-- Auto-update updated_at.
CREATE OR REPLACE FUNCTION update_report_schedules_updated_at()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$;

CREATE TRIGGER trg_report_schedules_updated_at
    BEFORE UPDATE ON report_schedules
    FOR EACH ROW EXECUTE FUNCTION update_report_schedules_updated_at();

-- RLS
ALTER TABLE report_schedules ENABLE ROW LEVEL SECURITY;
ALTER TABLE report_schedules FORCE ROW LEVEL SECURITY;

CREATE POLICY report_schedules_tenant_isolation ON report_schedules
    FOR ALL
    USING (
        is_super_admin()
        OR organization_id = get_current_org_id()
    )
    WITH CHECK (
        is_super_admin()
        OR organization_id = get_current_org_id()
    );

-- ============================================================================
-- report_executions
-- ============================================================================

CREATE TABLE IF NOT EXISTS report_executions (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    schedule_id     UUID        NOT NULL REFERENCES report_schedules(id) ON DELETE CASCADE,
    -- 'pending' | 'running' | 'completed' | 'failed' | 'cancelled' | 'skipped'
    status          TEXT        NOT NULL DEFAULT 'pending'
                                CHECK (status IN (
                                    'pending', 'running', 'completed',
                                    'failed', 'cancelled', 'skipped'
                                )),
    started_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at    TIMESTAMPTZ,
    -- Wall-clock duration in milliseconds.
    duration_ms     BIGINT,
    -- S3 object key for the generated report file (NULL until completed).
    file_key        TEXT,
    file_name       TEXT,
    file_size       BIGINT,
    -- Error details (only set when status = 'failed').
    error_code      TEXT,
    error_message   TEXT,
    error_details   TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Primary access pattern: all executions for a given schedule, newest first.
CREATE INDEX IF NOT EXISTS idx_report_executions_schedule
    ON report_executions(schedule_id, started_at DESC);

-- Filter by status (e.g. show only failed ones).
CREATE INDEX IF NOT EXISTS idx_report_executions_status
    ON report_executions(status);

-- RLS: inherit tenant isolation through the parent schedule.
ALTER TABLE report_executions ENABLE ROW LEVEL SECURITY;
ALTER TABLE report_executions FORCE ROW LEVEL SECURITY;

CREATE POLICY report_executions_tenant_isolation ON report_executions
    FOR ALL
    USING (
        is_super_admin()
        OR EXISTS (
            SELECT 1 FROM report_schedules s
            WHERE s.id = report_executions.schedule_id
              AND s.organization_id = get_current_org_id()
        )
    )
    WITH CHECK (
        is_super_admin()
        OR EXISTS (
            SELECT 1 FROM report_schedules s
            WHERE s.id = report_executions.schedule_id
              AND s.organization_id = get_current_org_id()
        )
    );
