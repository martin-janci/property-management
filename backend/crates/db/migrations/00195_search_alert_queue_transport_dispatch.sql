-- BIT-139 / Epic 16: email + push transport drainer dispatch tracking.
--
-- `search_alert_queue.status` is overloaded with the *in-app read* state
-- (`pending` → `sent` when the portal user reads the alert via the pull API:
-- `POST /api/v1/saved-searches/alerts/{id}/read`). The out-of-band email/push
-- drainer needs to track its own delivery *independently* so it neither
-- re-sends on every poll nor clobbers the in-app unread badge by flipping
-- `status` before the user has seen the alert.
--
-- `notified_at`     — set once the drainer has dispatched (email/push) the alert.
-- `notify_attempts` — incremented on each failed dispatch; the drainer stops
--                     retrying a row once it crosses its attempt budget so one
--                     permanently-undeliverable alert can't wedge the queue.
ALTER TABLE search_alert_queue
    ADD COLUMN IF NOT EXISTS notified_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS notify_attempts INTEGER NOT NULL DEFAULT 0;

-- The drainer scans for rows it has not yet delivered; a partial index keeps
-- that poll cheap as the queue grows (delivered rows drop out of the index).
CREATE INDEX IF NOT EXISTS idx_search_alert_queue_undelivered
    ON search_alert_queue (created_at)
    WHERE notified_at IS NULL;
