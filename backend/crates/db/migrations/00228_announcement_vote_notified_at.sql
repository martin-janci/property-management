-- Migration: 00228_announcement_vote_notified_at
-- Decouple "published/activated" from "notified" for the scheduler's
-- announcement / vote notification triggers (issue #2612, follow-up to PR #2608).
--
-- Background
-- ----------
-- `Scheduler::publish_scheduled_announcements` and the vote-lifecycle jobs did
-- two committed-but-decoupled things in one tick: (1) flip the row to
-- published/active/closed and COMMIT, then (2) resolve notification targets and
-- dispatch. After PR #2608 a failure in step (2) is logged, but because step (1)
-- already committed, the row is never re-selected — a single transient
-- target-resolution / dispatch blip permanently DROPS the notification
-- (fire-once).
--
-- Fix: add a nullable "notified" watermark to each notifiable transition so the
-- state change (published/active/closed) is decoupled from the notification.
-- The scheduler selects rows in the target state whose watermark IS NULL,
-- dispatches, and stamps the watermark ONLY on success — mirroring the
-- "stamp only after a successful run" pattern already used by the retention
-- prunes (`last_*_prune_at`) and `fire_due_report_schedules`. A transient error
-- simply leaves the watermark NULL and the next ~60s tick retries.
--
-- Semantics of the watermark (NULL = "needs notification"):
--   * The scheduler's automated transition (publish_scheduled / activate_scheduled
--     / close_expired) leaves the watermark NULL so the dispatch pass picks the
--     row up.
--   * Manual transitions (announcement publish, vote publish→active, vote close
--     from the manager route) stamp the watermark = NOW() so they are NOT picked
--     up by the scheduler dispatch pass — preserving the pre-existing behaviour
--     that manual publish/close did not fan out a scheduler notification.
--
-- Backfill: every row that has ALREADY transitioned past its notifiable moment
-- is stamped NOW() so this additive column does not cause a one-time mass
-- re-notification of the entire historical backlog on deploy. Rows still in the
-- pre-notification state (announcements/votes that are still `scheduled`, and
-- not-yet-closed votes) keep a NULL closed-watermark so their eventual scheduler
-- transition still notifies.

-- ---------------------------------------------------------------------------
-- Announcements: single "published" notification.
-- ---------------------------------------------------------------------------
ALTER TABLE announcements
    ADD COLUMN IF NOT EXISTS notified_at TIMESTAMPTZ;

COMMENT ON COLUMN announcements.notified_at IS
    'Scheduler notification watermark (issue #2612). NULL = the published-announcement notification still needs to be dispatched; stamped only after a successful dispatch so a transient failure is retried instead of dropped.';

-- Suppress a mass re-notification: everything that is not still `scheduled` has
-- already had (or will never get) its published notification.
UPDATE announcements
SET notified_at = COALESCE(published_at, updated_at, created_at)
WHERE status <> 'scheduled'
  AND notified_at IS NULL;

-- Cheap lookup for the dispatch pass (`published AND notified_at IS NULL`).
CREATE INDEX IF NOT EXISTS idx_announcements_pending_notification
    ON announcements (scheduled_at)
    WHERE status = 'published' AND notified_at IS NULL;

-- ---------------------------------------------------------------------------
-- Votes: two distinct notifiable transitions — vote-started (activation) and
-- vote-closed (result). Each needs its own watermark.
-- ---------------------------------------------------------------------------
ALTER TABLE votes
    ADD COLUMN IF NOT EXISTS started_notified_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS closed_notified_at TIMESTAMPTZ;

COMMENT ON COLUMN votes.started_notified_at IS
    'Scheduler notification watermark for the vote-started notification (issue #2612). NULL = still needs dispatch; stamped only after a successful dispatch.';
COMMENT ON COLUMN votes.closed_notified_at IS
    'Scheduler notification watermark for the closed-vote result notification (issue #2612). NULL = still needs dispatch; stamped only after a successful dispatch.';

-- Started watermark: any vote past `scheduled` has already had (or bypassed via
-- manual publish) its started moment.
UPDATE votes
SET started_notified_at = COALESCE(published_at, updated_at, created_at)
WHERE status <> 'scheduled'
  AND started_notified_at IS NULL;

-- Closed watermark: only already-closed votes have had their result moment.
UPDATE votes
SET closed_notified_at = COALESCE(results_calculated_at, updated_at, created_at)
WHERE status = 'closed'
  AND closed_notified_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_votes_pending_started_notification
    ON votes (start_at)
    WHERE status = 'active' AND started_notified_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_votes_pending_closed_notification
    ON votes (end_at)
    WHERE status = 'closed' AND closed_notified_at IS NULL;
