-- Migration: 00191_create_message_attachments
-- Epic 6 / UC-05.9 (gap-sweep #972.8 / BIT-184): Attach file to message.
--
-- Adds S3-backed attachments to direct messages, mirroring the existing
-- `announcement_attachments` shape (00015) and the messaging RLS conventions
-- (00017 / 00019 / 00171).
--
-- Storage flow: client requests a presigned PUT URL (scoped to the thread),
-- uploads the bytes to S3, sends the message, then links the uploaded object
-- to that message via this table. Only thread participants can list/download.

-- ============================================================================
-- MESSAGE ATTACHMENTS TABLE
-- ============================================================================

CREATE TABLE IF NOT EXISTS message_attachments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    message_id UUID NOT NULL REFERENCES messages(id) ON DELETE CASCADE,

    -- File info
    file_key VARCHAR(512) NOT NULL,   -- S3 object key
    file_name VARCHAR(255) NOT NULL,  -- Original filename (Content-Disposition)
    file_type VARCHAR(100) NOT NULL,  -- MIME type
    file_size BIGINT NOT NULL,        -- Size in bytes

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_message_attachments_message_id
    ON message_attachments(message_id);

COMMENT ON TABLE message_attachments IS 'S3-backed file attachments linked to direct messages (UC-05.9)';
COMMENT ON COLUMN message_attachments.file_key IS 'S3 object key; presigned URLs are minted on demand';

-- ============================================================================
-- ROW LEVEL SECURITY
-- ============================================================================
--
-- Mirrors the messages policy (00019): an attachment is visible only to
-- participants of the owning thread, scoped to the active org GUC. FORCE so the
-- policy is not bypassed when the app connects as the table owner (00171).

ALTER TABLE message_attachments ENABLE ROW LEVEL SECURITY;
ALTER TABLE message_attachments FORCE ROW LEVEL SECURITY;

CREATE POLICY message_attachments_participant_isolation ON message_attachments
    FOR ALL
    USING (
        message_id IN (
            SELECT m.id
            FROM messages m
            JOIN message_threads t ON t.id = m.thread_id
            WHERE current_setting('app.current_user_id', true)::UUID = ANY(t.participant_ids)
              AND t.organization_id = COALESCE(
                  current_setting('app.current_org_id', true)::UUID,
                  t.organization_id
              )
        )
    )
    WITH CHECK (
        message_id IN (
            SELECT m.id
            FROM messages m
            JOIN message_threads t ON t.id = m.thread_id
            WHERE current_setting('app.current_user_id', true)::UUID = ANY(t.participant_ids)
              AND t.organization_id = COALESCE(
                  current_setting('app.current_org_id', true)::UUID,
                  t.organization_id
              )
        )
    );
