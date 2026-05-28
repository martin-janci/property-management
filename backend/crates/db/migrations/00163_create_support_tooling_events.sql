-- Migration 00163: Support Tooling Analytics Events
--
-- Creates a dedicated append-only table for support-tooling page analytics,
-- separate from the generic audit_logs table that is gated by AuditRead.
--
-- Three event kinds are tracked (see #635):
--   support_data_viewed      — admin opened the Support Data page
--   support_user_searched    — admin executed a per-user lookup / search
--   support_sessions_revoked — admin revoked all sessions for a target user
--
-- The table is intentionally narrow. Rich per-event props are stored as
-- JSONB so future event kinds can carry arbitrary fields without schema
-- migrations. A DB-level trigger prevents UPDATE/DELETE so rows are
-- append-only (mirrors the pattern from migration 00138 for audit_logs).

CREATE TABLE support_tooling_events (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    event_kind    TEXT        NOT NULL
                  CHECK (event_kind IN (
                      'support_data_viewed',
                      'support_user_searched',
                      'support_sessions_revoked'
                  )),
    admin_user_id UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    props         JSONB       NOT NULL DEFAULT '{}',
    occurred_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Immutability trigger — prevent UPDATE/DELETE so the event log cannot be
-- tampered with after the fact (same defence as audit_logs, migration 00138).
CREATE OR REPLACE FUNCTION support_tooling_events_immutable()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'support_tooling_events rows are immutable';
END;
$$;

CREATE TRIGGER support_tooling_events_no_update
    BEFORE UPDATE ON support_tooling_events
    FOR EACH ROW EXECUTE FUNCTION support_tooling_events_immutable();

CREATE TRIGGER support_tooling_events_no_delete
    BEFORE DELETE ON support_tooling_events
    FOR EACH ROW EXECUTE FUNCTION support_tooling_events_immutable();

-- Index on admin_user_id for per-admin audit queries.
CREATE INDEX support_tooling_events_admin_user_id_idx
    ON support_tooling_events (admin_user_id);

-- Index on occurred_at for time-range queries.
CREATE INDEX support_tooling_events_occurred_at_idx
    ON support_tooling_events (occurred_at DESC);

-- Index on event_kind for filtering by event type.
CREATE INDEX support_tooling_events_kind_idx
    ON support_tooling_events (event_kind);
