-- Reality Portal: listing reports (UC-23).
--
-- Allows portal users to report problematic listings to the moderation team.

CREATE TABLE IF NOT EXISTS listing_reports (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    listing_id          UUID        NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
    reporter_user_id    UUID        REFERENCES portal_users(id) ON DELETE SET NULL,
    problem_type        TEXT        NOT NULL,
    description         TEXT        NOT NULL,
    -- JSON array of attachment URLs.
    attachments         JSONB,
    reporter_email      TEXT,
    reporter_phone      TEXT,
    -- status: received | in_review | resolved | rejected
    status              TEXT        NOT NULL DEFAULT 'received',
    resolution_notes    TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Lookup by listing (moderation queue).
CREATE INDEX IF NOT EXISTS idx_listing_reports_listing
    ON listing_reports(listing_id);

-- Lookup by reporter (user's own report history).
CREATE INDEX IF NOT EXISTS idx_listing_reports_reporter
    ON listing_reports(reporter_user_id);

-- Filter by status for moderation workflow.
CREATE INDEX IF NOT EXISTS idx_listing_reports_status
    ON listing_reports(status);

-- Auto-update updated_at.
CREATE OR REPLACE FUNCTION update_listing_reports_updated_at()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$;

CREATE TRIGGER trg_listing_reports_updated_at
    BEFORE UPDATE ON listing_reports
    FOR EACH ROW EXECUTE FUNCTION update_listing_reports_updated_at();
