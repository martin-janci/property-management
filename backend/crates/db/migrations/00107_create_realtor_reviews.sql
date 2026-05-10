-- Reality Portal: realtor reviews (UC-49, UC-51).
--
-- One review per user per realtor. verified_buyer is set by the application
-- based on prior confirmed inquiries (responded_at IS NOT NULL).

CREATE TABLE IF NOT EXISTS realtor_reviews (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    -- References the realtor_profiles.id (UC-33).
    realtor_id          UUID        NOT NULL REFERENCES realtor_profiles(id) ON DELETE CASCADE,
    reviewer_user_id    UUID        NOT NULL REFERENCES portal_users(id) ON DELETE CASCADE,
    -- 1–5 star rating.
    rating              INTEGER     NOT NULL CHECK (rating BETWEEN 1 AND 5),
    body                TEXT,
    -- Set server-side: true when reviewer had a prior responded inquiry.
    verified_buyer      BOOLEAN     NOT NULL DEFAULT FALSE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- One review per user per realtor.
    CONSTRAINT uq_realtor_review_user UNIQUE (realtor_id, reviewer_user_id)
);

CREATE INDEX IF NOT EXISTS idx_realtor_reviews_realtor
    ON realtor_reviews(realtor_id);

CREATE INDEX IF NOT EXISTS idx_realtor_reviews_reviewer
    ON realtor_reviews(reviewer_user_id);

-- Auto-update updated_at.
CREATE OR REPLACE FUNCTION update_realtor_reviews_updated_at()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$;

CREATE TRIGGER trg_realtor_reviews_updated_at
    BEFORE UPDATE ON realtor_reviews
    FOR EACH ROW EXECUTE FUNCTION update_realtor_reviews_updated_at();
