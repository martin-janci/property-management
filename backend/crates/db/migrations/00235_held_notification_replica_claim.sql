-- Migration: 00235_held_notification_replica_claim
-- At-most-once quiet-hours drain across multiple api-server replicas
-- (issue #2831, follow-up to PR #2826 / #2823).
--
-- Background
-- ----------
-- The quiet-hours drain worker runs inside every api-server process. Under a
-- single replica the read-then-deliver-then-mark-released loop is safe, but
-- under >1 replica it double-delivers:
--
--   * `get_notifications_to_release` was a plain SELECT of every due, not-yet
--     -released, not-dead-lettered row. It claimed nothing.
--   * Replica A and replica B poll on the same cadence, both SELECT the SAME
--     due rows, both call `deliver_held`, and both `mark_notification_released`.
--     The user receives every held notification twice (once per replica) — and
--     N times under N replicas.
--
-- The per-channel bookkeeping added in #2823 does NOT help here: it dedups
-- across sequential retry *ticks* of one worker (it reads `delivered_channels`,
-- delivers the gap, writes it back), but two replicas racing the same tick both
-- read the same empty `delivered_channels`, both send every channel, and only
-- then race to write — a classic lost-update. Correctness needs an atomic claim
-- at row-selection time, not a read-modify-write after delivery.
--
-- Fix (additive schema): a claim lease.
--   * claimed_at TIMESTAMPTZ — when a drain worker last claimed this row for
--     delivery. The worker now SELECTs due rows and stamps `claimed_at` in a
--     single atomic `UPDATE ... WHERE id IN (SELECT ... FOR UPDATE SKIP LOCKED)
--     RETURNING *`. Only the replica whose UPDATE wins the row gets it back to
--     deliver; concurrent claimers SKIP LOCKED past it and, once committed, see
--     a fresh `claimed_at` that excludes the row for the lease window. A held
--     notification is therefore delivered by at most one replica.
--   * The lease (claimed_at older than QUIET_HOURS_DRAIN_CLAIM_LEASE_SECS is
--     treated as unclaimed) makes the claim self-healing: a worker that crashes
--     mid-delivery leaves a stale claim that another replica re-claims once the
--     lease expires, rather than the row being stranded forever. On a handled
--     partial-failure retry the worker clears `claimed_at` so the next tick can
--     re-claim promptly instead of waiting out the lease.

ALTER TABLE held_notifications
    ADD COLUMN IF NOT EXISTS claimed_at TIMESTAMPTZ;

COMMENT ON COLUMN held_notifications.claimed_at IS
    'When a drain worker last claimed this row for delivery (issue #2831). The drain claims due rows with an atomic UPDATE ... FOR UPDATE SKIP LOCKED, stamping claimed_at, so a held notification is delivered by at most one api-server replica. Treated as a lease: a claim older than QUIET_HOURS_DRAIN_CLAIM_LEASE_SECS is re-claimable, so a crashed worker''s row is not stranded. Cleared on a partial-failure retry so the next tick can re-claim promptly.';

-- The claim subquery filters on the same (released_at IS NULL AND
-- dead_lettered_at IS NULL) predicate ordered by release_at, so the existing
-- partial index from 00234 (idx_held_notifications_release) still serves it.
-- claimed_at is only an equality/age check on the already-narrowed candidate
-- set, so no additional index is required.
