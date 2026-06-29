-- Follow-up to PR #1746 (#1784): DB-level defense-in-depth for listing enums.
--
-- `listings.status` / `property_type` / `transaction_type` / `currency` are
-- plain VARCHAR columns whose domains were documented only as inline comments
-- in migration 00049 and enforced solely in the reality-server route handler
-- (`ALLOWED_*` allow-lists in routes/portal_listings.rs). `portal_update_listing`
-- (migration 00186) is SECURITY DEFINER and does `status = COALESCE(p_status,
-- status)` with no DB guard, so any future second caller could persist an
-- out-of-domain value. These CHECK constraints make the database reject
-- out-of-domain values regardless of caller; the route allow-list stays the
-- friendly `400` and keeps the owner-vs-moderation distinction.
--
-- Domains are taken verbatim from migration 00049's inline comments and the
-- Rust allow-lists. NOTE: the status domain includes 'active' (the publicly
-- visible / moderation state) even though the owner route forbids owners from
-- setting it directly — the moderation path legitimately writes it.
--
-- Plain ALTER ... ADD CONSTRAINT (no IF NOT EXISTS, which Postgres rejects for
-- ADD CONSTRAINT) so a re-run fails loudly, matching repo style. Additive only:
-- no backfill — the sole existing writers (the guarded route + moderation) only
-- ever persist in-domain values.

ALTER TABLE listings ADD CONSTRAINT listings_status_check
    CHECK (status IN ('draft', 'active', 'paused', 'sold', 'rented', 'archived'));

ALTER TABLE listings ADD CONSTRAINT listings_property_type_check
    CHECK (property_type IN ('apartment', 'house', 'commercial', 'land', 'parking', 'storage', 'other'));

ALTER TABLE listings ADD CONSTRAINT listings_transaction_type_check
    CHECK (transaction_type IN ('sale', 'rent'));

ALTER TABLE listings ADD CONSTRAINT listings_currency_check
    CHECK (currency IN ('EUR', 'CZK'));
