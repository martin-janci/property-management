-- Migration: 00234_held_notification_delivery_tracking
-- Per-channel delivery bookkeeping + bounded retry for the quiet-hours drain
-- (issue #2823, follow-up to PR #2729).
--
-- Background
-- ----------
-- PR #2729 made `deliver_held` return a `PipelineResult` and had the drain
-- worker keep a held row for retry (`released_at` stays NULL) whenever any
-- channel reported a transient `failed`. That correctly stops a transient
-- failure from permanently dropping the notification, but retry is at ROW
-- granularity while `deliver_held` re-attempts EVERY channel on the row each
-- tick. Two problems fall out of that:
--
--   1. Duplicate delivery on partial failure. Row channels [push, email] where
--      push succeeds but email hits a transient failure is (correctly) kept for
--      retry, but the next tick re-sends push as well -> the user gets a
--      duplicate push. There is no per-channel success bookkeeping, so every
--      retry re-fires the channels that already succeeded.
--   2. Unbounded retry / no dead-letter. A row that keeps returning failed > 0
--      (permanently-misconfigured channel, permanently-rejected push token) is
--      never released and re-attempted on every tick forever, re-delivering any
--      healthy channel and never making progress.
--
-- Fix (additive schema):
--   * delivered_channels TEXT[] — channels already delivered successfully on a
--     prior tick. `deliver_held` skips channels already in this set, so a retry
--     only re-attempts the channels that have NOT yet succeeded (kills the
--     duplicate delivery).
--   * attempts INT — number of drain attempts made against the row. The worker
--     bumps it on every retry and, once it reaches a bounded cap, dead-letters
--     the row instead of looping forever.
--   * dead_lettered_at TIMESTAMPTZ — when the row gave up after exhausting its
--     retry budget. A dead-lettered row is excluded from the release query the
--     same way a released row is, but is distinguishable from a clean release
--     for operators.

ALTER TABLE held_notifications
    ADD COLUMN IF NOT EXISTS delivered_channels TEXT[] NOT NULL DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS attempts INT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS dead_lettered_at TIMESTAMPTZ;

COMMENT ON COLUMN held_notifications.delivered_channels IS
    'Channels already delivered successfully on a prior drain tick (issue #2823). deliver_held skips these so a partial-failure retry never re-delivers a channel that already succeeded.';
COMMENT ON COLUMN held_notifications.attempts IS
    'Number of quiet-hours drain attempts made against this row (issue #2823). Bumped on each retry; once it reaches the worker''s max-attempts cap the row is dead-lettered instead of retried forever.';
COMMENT ON COLUMN held_notifications.dead_lettered_at IS
    'Set when the row exhausted its retry budget and delivery was given up (issue #2823). Excluded from the release query like released_at, but distinguishable from a clean release.';

-- The drain worker selects rows that are neither released nor dead-lettered.
-- Recreate the release index so its partial predicate matches that query.
DROP INDEX IF EXISTS idx_held_notifications_release;
CREATE INDEX idx_held_notifications_release
    ON held_notifications(release_at)
    WHERE released_at IS NULL AND dead_lettered_at IS NULL;
