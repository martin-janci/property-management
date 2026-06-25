-- Favorite alert delivery queue + status watermark (BIT-138 / #983 follow-up).
--
-- Background workers cannot read `portal_favorites` / `listing_price_history`
-- without an explicit tenant context because both tables are under FORCE RLS.
-- This migration adds:
--   1. `portal_favorites.last_seen_listing_status` so workers can detect a
--      listing returning to market without a blanket bypass.
--   2. `favorite_alert_queue` for durable in-app delivery of price / status
--      notifications scoped to the owning portal user.

ALTER TABLE portal_favorites
    ADD COLUMN IF NOT EXISTS last_seen_listing_status VARCHAR(50);

UPDATE portal_favorites pf
SET last_seen_listing_status = l.status
FROM listings l
WHERE l.id = pf.listing_id
  AND pf.last_seen_listing_status IS NULL;

ALTER TABLE portal_favorites
    ALTER COLUMN last_seen_listing_status SET DEFAULT 'active';

CREATE TABLE favorite_alert_queue (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    favorite_id UUID NOT NULL REFERENCES portal_favorites(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    listing_id UUID NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
    alert_type VARCHAR(32) NOT NULL CHECK (alert_type IN ('price_change', 'back_on_market')),
    old_price DECIMAL(15, 2),
    new_price DECIMAL(15, 2),
    currency VARCHAR(3),
    change_percentage DECIMAL(5, 2),
    previous_status VARCHAR(50),
    new_status VARCHAR(50),
    status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'sent', 'failed')),
    source_changed_at TIMESTAMPTZ,
    processed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_favorite_alert_queue_user
    ON favorite_alert_queue(user_id);

CREATE INDEX idx_favorite_alert_queue_status
    ON favorite_alert_queue(status)
    WHERE status = 'pending';

CREATE UNIQUE INDEX idx_favorite_alert_queue_price_dedupe
    ON favorite_alert_queue(favorite_id, alert_type, source_changed_at)
    WHERE alert_type = 'price_change';

CREATE UNIQUE INDEX idx_favorite_alert_queue_status_dedupe
    ON favorite_alert_queue(favorite_id, alert_type, previous_status, new_status)
    WHERE alert_type = 'back_on_market';

-- Tenant isolation: a favorite_alert_queue row belongs to the org that owns the
-- referenced listing. Mirrors portal_favorites_tenant_isolation (the parent
-- table this queue is derived from) so the org boundary is identical. The
-- per-org FavoriteAlertWorker sets app.current_org_id before reading/writing,
-- and the portal-user delivery path runs under the same org context as
-- portal_favorites reads.
ALTER TABLE favorite_alert_queue ENABLE ROW LEVEL SECURITY;
ALTER TABLE favorite_alert_queue FORCE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS favorite_alert_queue_tenant_isolation ON favorite_alert_queue;
CREATE POLICY favorite_alert_queue_tenant_isolation ON favorite_alert_queue
    FOR ALL
    USING (
        (
            is_super_admin()
            OR EXISTS (
                SELECT 1
                FROM listings p
                WHERE p.id = favorite_alert_queue.listing_id
                  AND p.organization_id = get_current_org_id()
            )
        )
        AND get_current_org_not_deleted()
    )
    WITH CHECK (
        (
            is_super_admin()
            OR EXISTS (
                SELECT 1
                FROM listings p
                WHERE p.id = favorite_alert_queue.listing_id
                  AND p.organization_id = get_current_org_id()
            )
        )
        AND get_current_org_not_deleted()
    );
