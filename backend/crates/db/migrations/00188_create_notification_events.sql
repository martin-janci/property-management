-- Migration: 00188_create_notification_events
-- Epic 2B, Story 2B-C.3 (gap-sweep #969 gap 4): notification delivery tracking
-- & analytics.
--
-- Background
-- ----------
-- The notification pipeline (`NotificationPipeline::dispatch`, Epic 2B) produces
-- `DeliveryRecord`s in memory only — they were logged and discarded, so there
-- was no way to compute delivery/failure rates or drive a >5% failure-rate
-- alert. This table is the append-only event log those records are persisted to
-- (asynchronously, off the dispatch hot path).
--
-- Access model
-- ------------
-- Rows are written by the pipeline through the service-role pool (no per-request
-- user GUC is set), and read by operators through the capability-gated admin
-- analytics endpoint (also the service-role pool). There is no per-tenant read
-- here: this is platform-operator observability, mirroring `audit_logs`. RLS is
-- therefore a service-role-or-super-admin allowance, matching the
-- `device_push_tokens` / `critical_notifications` service-role convention
-- (00157, 00177): the GUC is unset on the service pool, so the policy permits;
-- an authenticated end-user RLS connection (user GUC set) is denied.
--
-- No foreign keys: this is a best-effort analytics log written after dispatch
-- completes, so a notification or user that disappears between dispatch and the
-- async write must never make the insert fail (and the `notification_id` for
-- email/push has no stored row to reference).

CREATE TABLE notification_events (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    -- The notification this event belongs to (not an FK — see header).
    notification_id UUID        NOT NULL,
    -- Target user (not an FK — append-only analytics, see header).
    user_id         UUID        NOT NULL,
    -- Delivery channel. Matches `NotificationChannel::as_str()`.
    channel         TEXT        NOT NULL CHECK (channel IN ('push', 'email', 'in_app')),
    -- Delivery event. `sent`/`failed`/`skipped` are emitted by the pipeline at
    -- dispatch time (mirroring `DeliveryStatus`); `delivered`/`opened`/`clicked`
    -- are reserved for downstream receipt/engagement tracking (email pixel,
    -- click webhooks) added in a follow-up — the table is ready for them now.
    event           TEXT        NOT NULL CHECK (
                                    event IN ('pending', 'sent', 'delivered',
                                              'failed', 'skipped', 'opened',
                                              'clicked')
                                ),
    -- Human-readable error (set when event = 'failed').
    error_message   TEXT,
    -- When the underlying delivery attempt / event occurred.
    occurred_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Time-window aggregation is the primary query shape (rates over a window),
-- usually further filtered by channel.
CREATE INDEX notification_events_occurred_at_idx
    ON notification_events (occurred_at);
CREATE INDEX notification_events_channel_event_idx
    ON notification_events (channel, event);
-- Per-notification lookup (timeline for one notification across channels).
CREATE INDEX notification_events_notification_id_idx
    ON notification_events (notification_id);

-- RLS: service-role (no user GUC) and super-admins only. See header.
ALTER TABLE notification_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE notification_events FORCE ROW LEVEL SECURITY;

CREATE POLICY notification_events_service_policy ON notification_events
    FOR ALL
    USING (
        NULLIF(current_setting('app.current_user_id', TRUE), '') IS NULL
        OR is_super_admin()
    )
    WITH CHECK (
        NULLIF(current_setting('app.current_user_id', TRUE), '') IS NULL
        OR is_super_admin()
    );
