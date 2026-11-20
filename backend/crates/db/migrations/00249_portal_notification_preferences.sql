-- Reality Portal notification preferences (per portal user, per topic).
--
-- The shared `notification_preferences` table is per *channel*
-- (email/push/in-app) for the property-management app. Portal users choose
-- per *topic* (new listings for saved searches, price drops on favorites,
-- inquiry replies, listing updates, marketing), which is what the mobile app
-- and reality-web settings expose. One row per user; a missing row means
-- "defaults" (everything on except marketing).
--
-- Access is keyed on `user_id` by reality-server's `PortalPrincipal`
-- (GH #1300); the row holds no tenant data, so no tenant RLS policy applies.
CREATE TABLE IF NOT EXISTS portal_notification_preferences (
    user_id           UUID        PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    new_listings      BOOLEAN     NOT NULL DEFAULT TRUE,
    price_drops       BOOLEAN     NOT NULL DEFAULT TRUE,
    inquiry_responses BOOLEAN     NOT NULL DEFAULT TRUE,
    listing_updates   BOOLEAN     NOT NULL DEFAULT TRUE,
    marketing         BOOLEAN     NOT NULL DEFAULT FALSE,
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
