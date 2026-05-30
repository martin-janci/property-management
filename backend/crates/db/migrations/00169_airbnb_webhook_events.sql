-- Gap 83-1 follow-up (#711): Airbnb inbound webhook deduplication.
--
-- Airbnb guarantees at-least-once delivery. Without a persistent dedup
-- store, rapid re-delivery of the same event_id will:
--   * For ReservationCreated/Updated — enqueue duplicate SYNC_EXTERNAL jobs
--     that race to upsert the same reservation.
--   * For ReservationCancelled — trip the booking-status guard but still
--     run the DB lookup and emit a spurious "duplicate delivery" warning.
--
-- This table is the canonical idempotency ledger for Airbnb webhook
-- deliveries. The handler upserts the event_id with ON CONFLICT DO NOTHING;
-- if no row is inserted, the delivery is a duplicate and the handler
-- returns 200 immediately (idempotent acknowledgement) without running any
-- side effects.
--
-- The table is intentionally tenant-agnostic: event_id is globally unique
-- across Airbnb and the row carries no PII (no listing_id, no
-- confirmation_code, no reservation data). No RLS is required.
--
-- Retention: rows can be pruned after ~7 days (Airbnb's documented retry
-- window) by a future maintenance job; we keep them indefinitely for now
-- so the table also serves as a basic delivery audit trail.

CREATE TABLE IF NOT EXISTS airbnb_webhook_events (
    event_id    TEXT        PRIMARY KEY,
    event_type  TEXT,
    received_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Sweep index for the future retention job — cheap, sees lots of writes,
-- few reads.
CREATE INDEX IF NOT EXISTS idx_airbnb_webhook_events_received_at
    ON airbnb_webhook_events (received_at);

COMMENT ON TABLE airbnb_webhook_events IS
    'Idempotency ledger for inbound Airbnb webhook deliveries (issue #711). '
    'Handler INSERTs event_id ON CONFLICT DO NOTHING; conflict = duplicate, '
    'return 200 without re-processing.';
