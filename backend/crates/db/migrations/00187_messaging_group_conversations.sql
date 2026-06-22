-- Migration: 00187_messaging_group_conversations
-- Epic 6 / Story UC-05.8 ([BIT-183]): Group conversations (N-party threads).
--
-- Generalizes `message_threads` from exactly-2 participants to N (>= 2):
--   1. relaxes the participant-count CHECK from "= 2" to ">= 2", and
--   2. ensures the (organization_id, participant_ids) UNIQUE constraint that
--      `get_or_create_thread_rls`'s `ON CONFLICT ON CONSTRAINT
--      message_threads_organization_id_participant_ids_key` clause already
--      references actually exists. Threads are stored with participant_ids
--      sorted + de-duped, so this enforces a single canonical thread per
--      (org, participant-set) for any N.
--
-- Idempotent / safe to re-run: every change is guarded.

-- 1. Relax the participant-count check from "= 2" to ">= 2".
ALTER TABLE message_threads
    DROP CONSTRAINT IF EXISTS message_threads_two_participants;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'message_threads_min_participants'
          AND conrelid = 'message_threads'::regclass
    ) THEN
        ALTER TABLE message_threads
            ADD CONSTRAINT message_threads_min_participants
            CHECK (array_length(participant_ids, 1) >= 2);
    END IF;
END $$;

-- 2. Ensure the UNIQUE constraint referenced by get_or_create_thread_rls exists.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'message_threads_organization_id_participant_ids_key'
          AND conrelid = 'message_threads'::regclass
    ) THEN
        ALTER TABLE message_threads
            ADD CONSTRAINT message_threads_organization_id_participant_ids_key
            UNIQUE (organization_id, participant_ids);
    END IF;
END $$;

COMMENT ON COLUMN message_threads.participant_ids IS
    'Sorted, de-duped array of >= 2 participant user UUIDs (UC-05.8: N-party group conversations)';
