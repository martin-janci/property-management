-- General in-app feedback & bug reports submitted from the mobile client
-- (#3544, follow-up to PR #3543).
--
-- Background
-- ----------
-- The mobile `FeedbackManager` (Epic 50, Story 50.4) POSTs a general feedback
-- submission to `POST /api/v1/feedback`, but no backend server mounted that
-- route — only `/api/v1/help/articles/{slug}/feedback` existed, which is the
-- narrow per-article helpful/not-helpful vote, not a general bug/feature
-- channel. Every mobile feedback submission therefore 404'd (#3544). This
-- table is the durable store the newly-mounted `POST /api/v1/feedback` handler
-- writes into.
--
-- Scope
-- -----
-- This is general product feedback (bug / feature / improvement / question /
-- other) addressed to the product team, not tenant-scoped business data. It
-- mirrors the existing `user_article_feedback` table (00034): user-scoped,
-- keyed on `user_id`, and intentionally NOT row-level-security protected —
-- it is not part of any organization's data plane. `organization_id` is kept
-- as an OPTIONAL, FK-less context column (the submitter's active building/org
-- as reported in the client `appContext`) purely so the product team can
-- triage by tenant; it is not an access-control boundary.

CREATE TABLE IF NOT EXISTS app_feedback (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- bug | feature | improvement | question | other (validated app-side; stored
    -- as free text so a new client enum value never bounces off a CHECK here).
    feedback_type VARCHAR(32) NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    -- Optional reply-to address the user opted to share.
    email TEXT,
    -- Optional screenshot payload (data URI / object key) the user attached.
    screenshot TEXT,
    -- Optional diagnostic blobs the client sends when the user opts in:
    -- `deviceInfo` (platform/os/app version/locale) and `appContext`
    -- (current screen, building, session duration). Stored verbatim as JSONB.
    device_info JSONB,
    app_context JSONB,
    -- Submitter's active organization/building context, if the client reported
    -- one. NO foreign key on purpose: this is a triage hint, not a relation, and
    -- feedback must survive the org/building being deleted.
    organization_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Product-team triage: list a user's submissions newest-first, and sweep by
-- type across the whole table.
CREATE INDEX IF NOT EXISTS idx_app_feedback_user ON app_feedback (user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_app_feedback_type ON app_feedback (feedback_type);

COMMENT ON TABLE app_feedback IS 'General in-app feedback / bug reports from the mobile client (#3544)';
