-- Migration: 00208_thread_participant_last_read_at
-- Per-participant read watermark for N-party group threads (#1773 finding-2, BIT-183).
--
-- Background
-- ----------
-- `messages.read_at` is a single per-message flag, so read state is shared
-- across every participant of a thread. In a 2-party direct thread that is fine
-- (the one other person reading it == "read"), but in a 3+-party group thread
-- (UC-05.8, BIT-183) the FIRST recipient to read a message sets `read_at`,
-- marking it read for EVERYONE — so the other participants' unread counts
-- under-report.
--
-- Fix: add a per-(thread, user) read watermark on `thread_participant_state`
-- (the per-participant table added in BIT-182, migration 00190). Unread is then
-- derived as "messages created after the caller's `last_read_at`", independent
-- per participant. A NULL watermark (no row, or a row that has never been
-- read-stamped) means every message from other senders is unread — the correct
-- default for a participant who has not opened the thread.
--
-- `messages.read_at` is left in place: it still drives the per-message read
-- field shown in the message list (a read receipt) and is unchanged for the
-- 2-party case. Only the unread *count* moves to the watermark.

ALTER TABLE thread_participant_state
    ADD COLUMN IF NOT EXISTS last_read_at TIMESTAMPTZ;

COMMENT ON COLUMN thread_participant_state.last_read_at IS
    'Per-user read watermark: messages created after this are unread for this user (#1773 / BIT-183)';
