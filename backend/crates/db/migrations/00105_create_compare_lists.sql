-- Reality Portal: compare lists (UC-48).
--
-- Stores the per-user listing compare list (max 4 entries enforced in application layer).

CREATE TABLE IF NOT EXISTS compare_lists (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID        NOT NULL REFERENCES portal_users(id) ON DELETE CASCADE,
    listing_id  UUID        NOT NULL REFERENCES listings(id)     ON DELETE CASCADE,
    added_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT uq_compare_user_listing UNIQUE (user_id, listing_id)
);

CREATE INDEX IF NOT EXISTS idx_compare_lists_user
    ON compare_lists(user_id);

CREATE INDEX IF NOT EXISTS idx_compare_lists_listing
    ON compare_lists(listing_id);
