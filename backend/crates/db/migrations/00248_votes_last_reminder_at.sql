-- Persistent vote-reminder dedup (vote-reminder spam fix).
--
-- The vote-reminder scheduler (`send_vote_reminders`) runs on a ~60s tick and
-- re-selected every active vote ending inside the reminder window on every
-- tick, re-notifying all owners who had not yet voted each time. There was no
-- guard at all, so non-voters were spammed with a reminder every minute for the
-- entire reminder window.
--
-- Add a persistent per-vote stamp (mirroring invoices.last_payment_reminder_at
-- (#1790) and signature reminders' last_reminder_at) so the scheduler can skip
-- votes already reminded within the ~5-minute dedup window.
ALTER TABLE votes
    ADD COLUMN IF NOT EXISTS last_reminder_at TIMESTAMPTZ;
