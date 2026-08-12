# code-review-api-core-quiet-hours-drain-untested

**Vector:** test-gap
**Score:** 3
**Source:** rotating-expert-review-pm-qa (2026-08-12)
**Confidence:** medium

## Hypothesis
`QuietHoursDrainWorker::drain_due()` in `backend/servers/api-server/src/services/quiet_hours_drain.rs` is only covered by pure-decision unit tests (the release-decision function). There is **no DB-integration test** exercising the real `held_notifications` table under mixed `PipelineResult` outcomes: partial-channel-failure must leave the row held for retry, full success must mark it released, and the `batch_limit` boundary must be respected across ticks. Without integration coverage a regression in the retry/release invariant would silently drop or infinitely re-hold user notifications in production. PR #2729 recently changed related quiet-hours logic ("don't release held notifications on failed quiet-hours drain") — regressing this path is high-risk today. Fix: add a `#[sqlx::test]` integration suite that seeds `held_notifications`, drives `drain_due()` under mocked `NotificationDispatcher::deliver` outcomes, and asserts row transitions.

## Evidence
- `backend/servers/api-server/src/services/quiet_hours_drain.rs` — `QuietHoursDrainWorker::drain_due()` reads `held_notifications`, calls `NotificationDispatcher::deliver`, and mutates `released_at` / `release_at`; no integration test file exists for it.
- PR #2729 (2026-08-10) `code-review-api-core-quiet-drain-drops-failed-delivery`: changed the release policy on failed drain — the invariant it added has no regression fence.
- PR #2730 (2026-08-10) `code-review-api-core-quiet-schedule-err-failopen`: closed a fail-open on schedule read error — same class of untested drain-path invariant.
- Search across `backend/servers/api-server/tests/suites/` finds no `quiet_hours_drain*.rs` test file.

## Files
- `backend/servers/api-server/src/services/quiet_hours_drain.rs`
- `backend/servers/api-server/tests/suites`
- `backend/servers/api-server/src/services`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (understanding the release-decision state machine before writing the test)
- [x] C2 — Seed data (needs a seeded `held_notifications` fixture)
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode:**

Mode: cloud-ok

## Repro steps
1. Seed two `held_notifications` rows for the same user, one whose channels include a failing endpoint, one whose channels all succeed.
2. Call `QuietHoursDrainWorker::drain_due(now_after_release)` with a mocked `NotificationDispatcher` that returns `PipelineResult::PartialFailure` for row 1 and `PipelineResult::Delivered` for row 2.
3. Expected: row 1 stays `released_at IS NULL` with `release_at` bumped for retry; row 2 has `released_at = NOW()` and is not returned by subsequent `drain_due` calls.
4. Actual today: no test asserts this. A silent regression in the release predicate (say, releasing on `PartialFailure`) would ship without any red signal.

## Suggested approach
1. Add `backend/servers/api-server/tests/suites/quiet_hours_drain_tests.rs`. Register the file in the suites index (`mod` declaration in `tests/main.rs` or equivalent — verify the crate's existing suite-registration pattern).
2. Use `#[sqlx::test(migrator = "backend::db::MIGRATOR")]` (or the crate's existing helper — mirror `oauth_integration_tests.rs`) so each test gets an isolated schema-applied DB.
3. Build a small `MockDispatcher` implementing the `NotificationDispatcher` trait; return injectable `PipelineResult`s per row.
4. Test cases (one function each): (a) all-success releases exactly the drained rows, (b) partial-failure keeps the row held with a bumped `release_at`, (c) full-failure keeps the row held, (d) `batch_limit` cap is respected — a 3-row seed drained with `batch_limit=2` leaves 1 row on the queue for the next tick, (e) rows outside the `release_at <= now` window are not drained.
5. Snapshot `held_notifications` state before and after each call; assert directly against `sqlx::query!` reads on the `released_at` / `release_at` columns.
6. Wire an assertion that fails against `origin/dev` today — proves the test is a real fence: run once before adding sanitising fixes to catch any incidental breakage.
7. `cargo test -p api-server --test quiet_hours_drain_tests` — must pass; then `cargo test -p api-server` full-workspace.

## Alternatives considered
- **Extend the existing unit tests with fake `Pool`** — rejected because the whole invariant lives in the SQL update predicate; mocking the pool loses coverage of the exact statement that governs release.
- **Property-based (proptest) fuzz over drain sequences** — rejected as too heavy for a first fence; add after the deterministic cases prove the machine.

## Root-cause trace
N/A — test-gap doesn't need backward tracing. The gap is the absence of a regression fence around a recently-modified invariant (PRs #2729, #2730).

## Test plan
- [ ] `backend/servers/api-server/tests/suites/quiet_hours_drain_tests.rs` — new file, 5 `#[sqlx::test]` cases above.
- [ ] Failing-on-main sanity: temporarily invert the release predicate; test must go red.
- [ ] Regression fence for PR #2729's "don't release on failed drain" invariant.
- [ ] Regression fence for `batch_limit` boundary.
- [ ] `cargo test -p api-server --test quiet_hours_drain_tests` — exact local command.
- [ ] `cargo test -p api-server` — full suite must stay green.

## Out of scope
- Refactoring `QuietHoursDrainWorker`'s dispatcher trait shape (this plan only adds tests).
- Adding tests for `QuietHoursScheduler` (separate module — has its own coverage plan).
- Frontend/mobile behaviour when a held notification is deferred.

## After-merge
- Move this file to `plans/_archive/code-review-api-core-quiet-hours-drain-untested.md`
- Mark the matching `backlog.json` row as `status: "done"`
