-- Migration: 00190_thread_participant_state
-- Epic 6 / Gap-sweep gaps #4 (Archive UC-05.11) + #6 (Delete UC-05.7), BIT-182
--
-- Per-participant conversation state: archive + soft-delete.
--
-- Background
-- ----------
-- `message_threads` stores its participants in a `UUID[]` array
-- (`participant_ids`), so there is nowhere to record a *per-user* view of a
-- shared thread. Both "archive conversation" and "delete conversation for me"
-- must be per-participant: one user archiving or deleting their copy of a
-- thread MUST NOT change what the other participant sees. This table holds that
-- per-(thread, user) state.
--
-- A missing row means the default state (visible, not archived). `deleted_at`
-- soft-hides the thread from the owning user's list only; the row and the
-- other participant's view are untouched. `archived_at` moves the thread to the
-- owning user's "Archived" tab. Both are cleared (set back to NULL) by the
-- repository's unhide / unarchive paths (e.g. a new inbound message unhides a
-- previously deleted thread).

CREATE TABLE IF NOT EXISTS thread_participant_state (
    thread_id UUID NOT NULL REFERENCES message_threads(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- Per-user soft delete: hides the thread from this user's list only.
    deleted_at TIMESTAMPTZ,

    -- Per-user archive: moves the thread to this user's "Archived" tab.
    archived_at TIMESTAMPTZ,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (thread_id, user_id)
);

-- Lookups are always keyed by the owning user (list filtering joins on
-- user_id), so index that leading column for the per-user list query.
CREATE INDEX IF NOT EXISTS idx_thread_participant_state_user
    ON thread_participant_state(user_id);

COMMENT ON TABLE thread_participant_state IS
    'Per-participant view state (archive + soft-delete) for message_threads (BIT-182)';
COMMENT ON COLUMN thread_participant_state.deleted_at IS
    'When set, the thread is hidden from this user''s list only (per-user soft delete)';
COMMENT ON COLUMN thread_participant_state.archived_at IS
    'When set, the thread appears in this user''s archived tab only';

-- Keep updated_at fresh on every change (matches the convention used by the
-- other messaging tables in 00017).
CREATE TRIGGER trigger_thread_participant_state_updated_at
    BEFORE UPDATE ON thread_participant_state
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- ROW LEVEL SECURITY
-- ============================================================================
--
-- Mirror the user-scoped messaging convention: a user may only see/manage their
-- own per-participant state rows. ENABLE + FORCE so the owning DB role cannot
-- bypass the policy (matches 00171 for the other messaging tables), with the
-- standard `is_super_admin()` bypass used across the schema. The GUC is
-- `app.current_user_id`, set by `set_request_context()` / `RlsConnection`.

ALTER TABLE thread_participant_state ENABLE ROW LEVEL SECURITY;
ALTER TABLE thread_participant_state FORCE ROW LEVEL SECURITY;

CREATE POLICY thread_participant_state_owner_isolation ON thread_participant_state
    FOR ALL
    USING (
        user_id = NULLIF(current_setting('app.current_user_id', TRUE), '')::UUID
        OR is_super_admin()
    )
    WITH CHECK (
        user_id = NULLIF(current_setting('app.current_user_id', TRUE), '')::UUID
        OR is_super_admin()
    );
