# gh-issue-2831-quiet-hours-drain-concurrent-race

**Vector:** bug
**Score:** 3
**Source:** Issue #2831 (follow-up on PR #2826)
**Confidence:** high

## Hypothesis
`QuietHoursDrainWorker` starts unconditionally on every api-server process. `GranularNotificationRepository::get_notifications_to_release()` is a plain `SELECT` with no `FOR UPDATE SKIP LOCKED`, so with >1 replica each worker reads the same due rows before either persists its `delivered_channels` merge. The per-channel bookkeeping added in PR #2826 solves only the sequential-retry case; the concurrent-worker case still double-delivers held notifications and races the release/attempt bookkeeping. Smallest fix: switch the row selection to an atomic `UPDATE … WHERE id IN (SELECT … FOR UPDATE SKIP LOCKED)` claim inside a transaction, and fold `batch_limit` into the SQL `LIMIT`.

## Evidence
- `backend/servers/api-server/src/main.rs:709-715` — `QuietHoursDrainWorker::new(...)` followed by `worker.start()` is invoked on every replica, gated only by `QUIET_HOURS_DRAIN_ENABLED`; no leader election.
- `backend/crates/db/src/repositories/granular_notification.rs:366-379` — `get_notifications_to_release()` runs `SELECT * FROM held_notifications WHERE released_at IS NULL AND dead_lettered_at IS NULL AND release_at <= $1 ORDER BY release_at`, no `FOR UPDATE`, no `LIMIT`.
- Issue #2831 traces the sequence: two replicas race the same row → both read `delivered_channels = ∅` → both call `deliver_held` → user gets one duplicate per channel per extra replica.
- Sibling writes `mark_notification_released` / `record_held_attempt` / `mark_notification_dead_lettered` are also unguarded, so attempt increments and the release timestamp can lose/duplicate updates under contention.

## Files
- `backend/crates/db/src/repositories/granular_notification.rs:366`
- `backend/servers/api-server/src/services/quiet_hours_drain.rs`
- `backend/servers/api-server/src/services/notification_pipeline.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

Mode: cloud-ok

## Repro steps
1. Start two api-server replicas against one Postgres with `QUIET_HOURS_DRAIN_ENABLED=1` and a low `drain_interval`.
2. Insert one row into `held_notifications` with `release_at` in the past and non-empty channel set.
3. Both workers tick simultaneously; expected: `deliver_held` called once, one notification per channel; actual: `deliver_held` called ≥ 2×, user receives one extra notification per channel per extra replica.

## Suggested approach
1. In `granular_notification.rs`, rename `get_notifications_to_release` (or add a peer `claim_notifications_to_release(limit)`) that runs the atomic-claim CTE inside a transaction: `UPDATE held_notifications SET attempts = attempts WHERE id IN (SELECT id FROM held_notifications WHERE released_at IS NULL AND dead_lettered_at IS NULL AND release_at <= $1 ORDER BY release_at FOR UPDATE SKIP LOCKED LIMIT $2) RETURNING *`.
2. Push `batch_limit` from the Rust `.take()` in `quiet_hours_drain.rs` into the SQL `LIMIT $2` so the claim is bounded.
3. Update `quiet_hours_drain.rs` to call the new claim method; keep the merge-delivered/attempts/mark-released write path unchanged (each row now owned by exactly one worker for the tick, so no additional locking needed on the followup writes).
4. Add a concurrent-race integration test to the existing `quiet_hours_drain.rs` tests (or a new sibling test file) that spins up two workers against the same in-memory Postgres and asserts `deliver_held` fires exactly once for one due row.
5. Leave `notification_pipeline.rs` alone unless it calls the renamed method; if so, update the call site.

## Alternatives considered
- **Postgres advisory lock (`pg_try_advisory_lock`) around the whole drain loop** — rejected because it serialises all drain work across the fleet, wiping horizontal throughput; the row-level claim keeps parallelism while still eliminating the race.
- **In-Kubernetes leader election (leases / stateful set) selecting a single drain replica** — rejected because it moves the fix out of the app and requires deployment-topology guarantees the repo does not currently manage; the SQL claim keeps the fix contained to backend code and works under any deployment shape.

## Root-cause trace
1. Symptom: users receive duplicate held-notifications per channel per extra api-server replica after PR #2826 landed.
2. ← `deliver_held` called more than once per row at `backend/servers/api-server/src/services/quiet_hours_drain.rs` (per-tick loop over `get_notifications_to_release()`).
3. ← Two workers select the same row because `get_notifications_to_release()` at `backend/crates/db/src/repositories/granular_notification.rs:366-379` uses a plain `SELECT` with no `FOR UPDATE SKIP LOCKED` and no atomic claim.
4. Origin: PR #2729 introduced the drain worker; PR #2826 (issue #2823) closed only the sequential-retry duplicate case, leaving the concurrent case untouched (documented in issue #2831).

## Test plan
- [ ] `backend/servers/api-server/tests/suites/quiet_hours_drain_concurrent_test.rs` — spin up two workers, one due row, assert single delivery.
- [ ] Existing `quiet_hours_drain.rs` unit tests still pass (per-channel bookkeeping unchanged).
- [ ] `cd backend && cargo test -p api-server quiet_hours_drain`

## Out of scope
- Any refactor of `deliver_held` itself or the channel-fanout logic.
- Migrating from advisory locks / adding new columns to `held_notifications` (the claim uses `FOR UPDATE SKIP LOCKED` on existing columns; new columns would need a migration).
- Leader-election infra for other periodic workers (`push_fanout`, `scheduler`, `notification_digest`) — track separately if they exhibit the same pattern.

## After-merge
- Move this file to `plans/_archive/gh-issue-2831-quiet-hours-drain-concurrent-race.md`
- Mark the matching `backlog.json` row as `status: "done"`
