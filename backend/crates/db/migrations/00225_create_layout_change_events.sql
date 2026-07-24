-- Migration 00225: Layout Publish/Webhook Analytics Events (sink + retention)
--
-- Creates the append-only sink for the layout publish → outbound webhook →
-- reality-web ISR revalidation pipeline, closing gap C of
-- docs/data/layout-publish-event-tracking.md (the events were defined there but
-- nothing persisted them). Producers:
--   layout_change_published   — api-server routes/layout/admin.rs (one per
--                               successful publish/rollback/kill/unkill)
--   layout_webhook_dispatched — api-server routes/layout/webhook.rs (outbound
--                               delivery outcome, incl. the unconfigured no-op)
--   layout_revalidate_received— reality-web /api/layout-revalidate (receiver)
--
-- Mirrors the support_tooling_events pattern (migration 00163 + 00165 + 00222):
-- a deliberately narrow table with rich per-event props in JSONB, DB-level
-- triggers that reject UPDATE/DELETE so the trail is append-only / tamper-
-- evident, and a DB-native retention prune whose sanctioned DELETE path is
-- gated by the app.retention_prune GUC.
--
-- NO RLS: like layout_configs / layout_config_versions / layout_kill_flags
-- (migration 00221) these are platform-admin-owned GLOBAL rows, not tenant-
-- scoped. Access is gated at the application layer (superadmin routes). Because
-- the table carries no FORCE-RLS policy, the retention prune does NOT need to
-- assert app.is_super_admin (unlike cleanup_old_support_tooling_events); it only
-- opens the immutability-trigger's sanctioned DELETE path.
--
-- Retention: 730 days (24 months) — the same sanctioned lifetime as
-- support_tooling_events (00222), long enough for security / accountability
-- investigations, bounded enough to honour GDPR storage limitation
-- (Art. 5(1)(e)). Defined UP FRONT (privacy finding on issue #2532) rather than
-- deferred. The period is a function parameter (DEFAULT 730) so operators can
-- override per-invocation without a schema change. Wiring the prune into the
-- api-server background scheduler is a separate follow-up (same posture as
-- support_tooling_events); this migration + repository method establish the
-- retention definition itself.

CREATE TABLE layout_change_events (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    event_kind    TEXT        NOT NULL
                  CHECK (event_kind IN (
                      'layout_change_published',
                      'layout_webhook_dispatched',
                      'layout_revalidate_received'
                  )),
    -- Layout screen id, e.g. 'reality/listing-detail'. Nullable: the receiver
    -- records an event even on the invalid_body path where no screen parsed.
    screen        TEXT,
    -- Correlation id shared across the three events for one mutation
    -- (gap D). Nullable so a bare/legacy delivery can still be recorded.
    delivery_id   UUID,
    -- Acting platform SuperAdmin for layout_change_published. FK with
    -- ON DELETE SET NULL so the audit trail survives (anonymised) account
    -- deletion — matches the privacy posture in the events doc §7. NULL for
    -- operational events (webhook_dispatched / revalidate_received) which carry
    -- no admin identity.
    published_by  UUID        REFERENCES users(id) ON DELETE SET NULL,
    -- Rich per-event dimensions (change_kind, layout_version, target_tenant,
    -- dispatch_outcome, revalidate_outcome, http_status, tags, …). JSONB so new
    -- event kinds can add fields without a schema migration. Never carries the
    -- webhook signature/secret or the layout config payload (events doc §7).
    props         JSONB       NOT NULL DEFAULT '{}',
    occurred_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Immutability trigger — UPDATE is always rejected; DELETE is rejected unless
-- the caller is inside the sanctioned retention path (app.retention_prune =
-- 'on', set transaction-locally by cleanup_old_layout_change_events). Any other
-- UPDATE/DELETE is tampering and is refused, preserving append-only semantics.
-- Same defence as support_tooling_events (migration 00163 + 00222).
CREATE OR REPLACE FUNCTION layout_change_events_immutable()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' THEN
        RAISE EXCEPTION 'layout_change_events rows are immutable';
    END IF;

    -- TG_OP = 'DELETE': allow only inside the sanctioned retention path.
    IF current_setting('app.retention_prune', true) IS DISTINCT FROM 'on' THEN
        RAISE EXCEPTION 'layout_change_events rows are immutable';
    END IF;

    RETURN OLD;
END;
$$;

CREATE TRIGGER layout_change_events_no_update
    BEFORE UPDATE ON layout_change_events
    FOR EACH ROW EXECUTE FUNCTION layout_change_events_immutable();

CREATE TRIGGER layout_change_events_no_delete
    BEFORE DELETE ON layout_change_events
    FOR EACH ROW EXECUTE FUNCTION layout_change_events_immutable();

-- DB-native retention prune. Mirrors cleanup_old_support_tooling_events (00222):
-- DELETE aged rows by timestamp inside the sanctioned path, return the deleted
-- count. The GUC is set transaction-locally (is_local = true) and reset before
-- return, so delete rights never bleed onto the pooled connection or any
-- subsequent statement in the same transaction. No app.is_super_admin needed —
-- the table has no RLS policy.
CREATE OR REPLACE FUNCTION cleanup_old_layout_change_events(retention_days INTEGER DEFAULT 730)
RETURNS BIGINT AS $$
DECLARE
    deleted_count BIGINT;
BEGIN
    PERFORM set_config('app.retention_prune', 'on', true);

    DELETE FROM layout_change_events
    WHERE occurred_at < NOW() - (retention_days || ' days')::INTERVAL;

    GET DIAGNOSTICS deleted_count = ROW_COUNT;

    PERFORM set_config('app.retention_prune', 'off', true);

    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION cleanup_old_layout_change_events IS
    'Retention prune for layout_change_events: deletes layout publish/webhook/'
    'revalidate analytics events older than the retention period (default 730 '
    'days / 24 months, GDPR Art. 5(1)(e) storage limitation). Deletes are gated '
    'by the app.retention_prune GUC so the append-only immutability trigger '
    'still blocks every other UPDATE/DELETE.';

-- Time-range queries (dashboards, prune scans).
CREATE INDEX layout_change_events_occurred_at_idx
    ON layout_change_events (occurred_at DESC);

-- Filter by event type.
CREATE INDEX layout_change_events_kind_idx
    ON layout_change_events (event_kind);

-- Join the three events of one mutation on the shared correlation id.
CREATE INDEX layout_change_events_delivery_id_idx
    ON layout_change_events (delivery_id)
    WHERE delivery_id IS NOT NULL;
