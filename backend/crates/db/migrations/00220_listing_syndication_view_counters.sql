-- Give the portal syndication view/inquiry counters a real, observable home (#2360).
--
-- Background
-- ----------
-- `increment_syndication_stats` (backend/crates/db/src/repositories/listing.rs)
-- was a no-op for counts: it discarded `views_delta`/`inquiries_delta` and only
-- bumped `updated_at`, while every read path (`get_syndication_status`,
-- `get_organization_syndication_stats`) hardcoded `views: 0` / `inquiries: 0`.
-- That made the #2360 dedup gate in `process_view_webhook` (advance the counter
-- only on a freshly-`Recorded` delivery, skip it on a `Duplicate` replay/retry)
-- observably meaningless -- gating a no-op on a dedup signal changes no stored
-- number.
--
-- This migration lands the counter the gate protects: two monotonic BIGINT
-- columns on `listing_syndications`, incremented by `increment_syndication_stats`
-- and surfaced by the read paths. The dedup *anchor* that makes the gate's
-- `Duplicate` branch reachable (the partial unique index on
-- `portal_webhook_events (portal, event_type, external_id)`) is created by the
-- sibling change for #2358 in migration 00219; once that lands, a replayed
-- delivery is suppressed and these counters no longer inflate.
--
-- RLS: `listing_syndications` is already FORCE ROW LEVEL SECURITY (00110), and
-- these are plain additive columns on the existing row, so no policy change is
-- needed.

ALTER TABLE listing_syndications
    ADD COLUMN IF NOT EXISTS total_views BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS total_inquiries BIGINT NOT NULL DEFAULT 0;

-- Guard against negative deltas ever driving a counter below zero.
ALTER TABLE listing_syndications
    ADD CONSTRAINT listing_syndications_total_views_nonneg CHECK (total_views >= 0),
    ADD CONSTRAINT listing_syndications_total_inquiries_nonneg CHECK (total_inquiries >= 0);
