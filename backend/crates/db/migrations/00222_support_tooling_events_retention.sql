-- Migration 00222: Support Tooling Events — Retention Policy (data-privacy)
--
-- Publishes a data-retention (TTL) policy for the append-only
-- support_tooling_events audit trail created in migration 00163 and hardened
-- in 00165. Until now the table had NO automated retention: rows persisted
-- indefinitely, which conflicts with GDPR storage-limitation (Art. 5(1)(e)).
-- See docs/data/support-data-retention-privacy.md for the full policy write-up.
--
-- Retention period: 730 days (24 months). This is the sanctioned lifetime for
-- admin support-tooling audit events — long enough to satisfy security /
-- accountability investigations, bounded enough to honour storage limitation.
-- The period is a function parameter (DEFAULT 730) so operators can override
-- per-invocation without a schema change, mirroring cleanup_old_traces
-- (migration 00079) and cleanup_old_health_check_results (migration 00074).
--
-- Mechanism: a DB-native `cleanup_old_support_tooling_events(retention_days)`
-- function, invoked from db::PlatformAdminRepository::cleanup_old_support_tooling_events
-- (the same repository-method + `SELECT cleanup_old_*()` pattern the tracing and
-- health-monitoring retention jobs already use). Wiring the call into the
-- api-server background scheduler is a separate, out-of-scope follow-up (owner
-- pm-backend) — this migration + repository method establish the retention
-- definition itself.
--
-- Immutability interaction: support_tooling_events carries triggers (migration
-- 00163) that reject ALL UPDATE/DELETE so the log is tamper-evident. A retention
-- prune must, by definition, DELETE aged rows. We therefore relax the trigger to
-- permit DELETE **only** inside the sanctioned retention path, which sets the
-- session-local `app.retention_prune` GUC (same `app.*` namespace as
-- app.org_id / app.is_super_admin). Every other UPDATE/DELETE — i.e. any
-- tampering attempt — is still rejected. UPDATE remains unconditionally
-- forbidden.

-- 1. Relax the immutability trigger: UPDATE always rejected; DELETE rejected
--    unless the caller is inside the sanctioned retention path.
CREATE OR REPLACE FUNCTION support_tooling_events_immutable()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' THEN
        RAISE EXCEPTION 'support_tooling_events rows are immutable';
    END IF;

    -- TG_OP = 'DELETE': allow only when the retention-prune GUC is set to 'on'
    -- (set transaction-locally by cleanup_old_support_tooling_events). Any other
    -- DELETE is tampering and is rejected, preserving append-only semantics.
    IF current_setting('app.retention_prune', true) IS DISTINCT FROM 'on' THEN
        RAISE EXCEPTION 'support_tooling_events rows are immutable';
    END IF;

    RETURN OLD;
END;
$$;

-- 2. DB-native retention prune. Mirrors cleanup_old_traces (00079) /
--    cleanup_old_health_check_results (00074): DELETE aged rows by timestamp,
--    return the deleted count. Wrapped so it (a) opens the sanctioned
--    retention path for the immutability trigger and (b) asserts the
--    super-admin RLS context so the FORCE-RLS policy (migration 00165) permits
--    the delete when run by the api-server (bound by FORCE RLS, no BYPASSRLS).
--    Both GUCs are set transaction-locally (is_local = true) and reset before
--    return, so context never bleeds onto the pooled connection.
CREATE OR REPLACE FUNCTION cleanup_old_support_tooling_events(retention_days INTEGER DEFAULT 730)
RETURNS BIGINT AS $$
DECLARE
    deleted_count BIGINT;
BEGIN
    PERFORM set_config('app.retention_prune', 'on', true);
    PERFORM set_config('app.is_super_admin', 'true', true);

    DELETE FROM support_tooling_events
    WHERE occurred_at < NOW() - (retention_days || ' days')::INTERVAL;

    GET DIAGNOSTICS deleted_count = ROW_COUNT;

    -- Close the sanctioned path immediately after the prune so no subsequent
    -- statement in the same transaction inherits delete rights.
    PERFORM set_config('app.retention_prune', 'off', true);
    PERFORM set_config('app.is_super_admin', 'false', true);

    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION cleanup_old_support_tooling_events IS
    'Retention prune for support_tooling_events: deletes admin support-tooling '
    'audit events older than the retention period (default 730 days / 24 months, '
    'GDPR Art. 5(1)(e) storage limitation). Deletes are gated by the '
    'app.retention_prune GUC so the append-only immutability trigger still blocks '
    'every other UPDATE/DELETE.';
