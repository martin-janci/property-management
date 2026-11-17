-- Terminal "abandoned" state for notification digests (Story 2B.3 follow-up).
--
-- Background
-- ----------
-- The inactivity email-digest worker (`notification_digest.rs`) persists a
-- digest row first and only marks `email_sent_at` once the email actually
-- sends, retrying transient `EmailService` failures in place. A row whose send
-- keeps failing eventually ages past the resend window and drops out of the
-- retry query (`get_unsent_digests`). At that point it is permanently given up
-- on — never retried again.
--
-- Until now that terminal state existed only in the worker's logs: each newly
-- aged-out row was logged once (at error level) and then left in the table
-- indistinguishable from a still-pending unsent row (`email_sent_at IS NULL`).
-- An operator could not query "which digests were abandoned?" — the only
-- record was a log line that scrolled away. That makes follow-up (manual
-- resend, incident triage, metrics) impossible after the fact.
--
-- Change
-- ------
-- Add a nullable `abandoned_at` timestamp. The worker sets it when a row
-- crosses out of the retry window, turning "abandoned" into a durable,
-- queryable terminal state alongside the existing `email_sent_at` /
-- `push_sent_at` delivery-status columns:
--
--   * `email_sent_at IS NOT NULL`                       -> delivered
--   * `email_sent_at IS NULL AND abandoned_at IS NULL`  -> pending / retrying
--   * `email_sent_at IS NULL AND abandoned_at IS NOT NULL` -> abandoned (terminal)
--
-- Marking is idempotent: the worker only marks rows where `abandoned_at IS NULL`,
-- so each abandoned digest is recorded (and logged) exactly once.
--
-- Backfill
-- --------
-- Existing unsent-and-aged rows are intentionally left with `abandoned_at NULL`.
-- The worker marks them on its next sweeps (bounded by its per-tick batch
-- limit), so no historical row is retroactively invented as "abandoned" by this
-- migration; abandonment is only ever recorded by the worker that observes it.

ALTER TABLE notification_digests
    ADD COLUMN abandoned_at TIMESTAMPTZ;

-- Supports listing/counting abandoned digests ("which digests were given up
-- on?"). Partial so it only indexes the terminal rows, which are the rare case.
CREATE INDEX idx_notification_digests_abandoned
    ON notification_digests (abandoned_at)
    WHERE abandoned_at IS NOT NULL;
