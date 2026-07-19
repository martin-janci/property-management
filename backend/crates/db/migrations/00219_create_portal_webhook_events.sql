-- Create the missing `portal_webhook_events` table + its dedup net (#2358).
--
-- Background
-- ----------
-- `record_webhook_event` / `mark_webhook_event_processed` /
-- `get_unprocessed_webhook_events` (backend/crates/db/src/repositories/listing.rs)
-- and the portal-webhook receivers (api-server routes/portal_webhooks.rs) have
-- always read and written a `portal_webhook_events` table, but **no migration
-- ever created it** (grep for `portal_webhook_events` across backend/**/*.sql
-- returned zero hits before this file). The table was effectively relying on an
-- ambient/hand-created schema.
--
-- The immediate trigger is #2358 (follow-up to PR #2353 / #2342 finding 2): the
-- inquiry path now escalates an unmatched syndication to a retriable 500 so the
-- portal **re-delivers** the lead. Re-delivery is only safe if a duplicate
-- delivery is idempotent, but the idempotency net
-- (`WebhookPersistOutcome::Duplicate`, api-server portal_webhooks.rs) can only
-- fire when a duplicate INSERT is rejected — which requires a unique index that
-- did not exist. Without it, every retry / at-least-once redelivery inserted a
-- **duplicate lead row**. This migration lands the dedup net that PR #2353
-- deferred (its findings 1 + 4), so the retry escalation is actually safe.
--
-- RLS
-- ---
-- Intentionally NOT `ENABLE/FORCE ROW LEVEL SECURITY`. This is a portal-ingest
-- log written on the *unauthenticated* portal-webhook path
-- (`process_inquiry_webhook` / `process_view_webhook`), which runs with a plain
-- pool and no `app.current_organization_id` GUC. An org-scoped RLS policy would
-- deny-all those writes and re-break the very lead path #2358 is protecting.
-- Tenant scoping is enforced downstream via the FK to `listings` /
-- `listing_syndications` (both org-scoped, FORCE RLS) when the events are
-- aggregated by authenticated code paths.

CREATE TABLE IF NOT EXISTS portal_webhook_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    listing_id UUID NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
    syndication_id UUID NOT NULL REFERENCES listing_syndications(id) ON DELETE CASCADE,
    portal VARCHAR(50) NOT NULL,          -- reality_portal, sreality, bezrealitky, nehnutelnosti
    event_type VARCHAR(50) NOT NULL,      -- view, inquiry, price_alert, status_update, ...
    external_id VARCHAR(255),             -- ID on the external portal (nullable)
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    processed BOOLEAN NOT NULL DEFAULT false,
    processed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Dedup net (#2358 high finding): a redelivery of the same portal event (same
-- portal + event_type + external_id) must NOT insert a second row. Partial so
-- that events without an external_id (which have no stable dedup key) are not
-- collapsed together. Matched by the
-- `ON CONFLICT (portal, event_type, external_id) WHERE external_id IS NOT NULL
-- DO NOTHING` clause in `record_webhook_event`.
CREATE UNIQUE INDEX idx_portal_webhook_events_dedup
    ON portal_webhook_events (portal, event_type, external_id)
    WHERE external_id IS NOT NULL;

-- Supports `get_unprocessed_webhook_events` (WHERE portal = $1 AND processed =
-- false ORDER BY created_at).
CREATE INDEX idx_portal_webhook_events_unprocessed
    ON portal_webhook_events (portal, created_at)
    WHERE processed = false;

CREATE INDEX idx_portal_webhook_events_listing
    ON portal_webhook_events (listing_id);
