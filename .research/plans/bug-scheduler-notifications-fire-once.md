# bug-scheduler-notifications-fire-once

**Vector:** bug
**Score:** 3
**Source:** Issue #2612
**Confidence:** high

## Hypothesis
The scheduler's `publish_scheduled_announcements` and its vote-lifecycle siblings commit the terminal state (announcement published, vote started/closed) *before* dispatching notifications. PR #2608 upgraded the dispatch-error paths from silent `unwrap_or_default()` to a logged `Vec::new()` fallback, so a failed dispatch is now *observable* — but the entry is never re-selected on the next tick, so a single transient blip (DB error resolving targets, notification-service RPC failure) permanently drops the notification. The smallest change that fixes it is to decouple "state committed" from "notified": add a nullable `notified_at` timestamp per row, and have a scheduler pass select `<terminal-state> AND notified_at IS NULL`, dispatch, and stamp `notified_at` only on success — the same pattern already used by the retention prunes (`last_*_prune_at`) and `fire_due_report_schedules`.

## Evidence
- `backend/servers/api-server/src/services/scheduler/mod.rs:361-434` — `publish_scheduled_announcements` flips announcements to published at line 365, then per-announcement resolves targets at line 387 and dispatches at line 404; both are best-effort with logged fallback but no retry (also called out in PR #2608 commit body).
- `backend/servers/api-server/src/services/scheduler/votes.rs` — activation / close paths follow the same shape (single-tick dispatch of vote-started and vote-result notifications; DB failures logged, activation stays committed).
- Issue #2612 — post-merge review write-up of PR #2608 explicitly documenting the durability gap the PR left out of scope.
- `backend/servers/api-server/src/services/scheduler/retention.rs` — reference pattern: retention prunes stamp `last_*_prune_at` only after a successful run, so a transient failure is retried on the next tick.
- PR #2608 body: "This follow-up covers the durability gap that #2608 intentionally left out of scope" — the observability fix landed, the durability fix is queued for this plan.

## Files
- `backend/servers/api-server/src/services/scheduler/mod.rs`
- `backend/servers/api-server/src/services/scheduler/votes.rs`
- `backend/crates/db/src/repositories/announcement.rs`
- `backend/crates/db/src/repositories/vote.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug — trace fire-once → find publish-before-notify split → introduce `notified_at`)
- [x] C2 — Seed data (need at least one scheduled announcement + one active vote in the test DB to exercise the retry pass)
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion (`cargo test -p api-server --test suite_5` after adding the retry-loop regression test)
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):**
`Mode: cloud-ok` (no C4/C5)

## Repro steps
1. Seed one scheduled announcement whose `scheduled_at` is in the past and whose `target_type` resolves via a query the test can force to fail (e.g. inject a repository stub that errors on the first call to `get_announcement_target_users` and succeeds on the second).
2. Tick the scheduler once — assert the announcement flipped to `published` and dispatch was skipped due to the injected error (log line at `scheduler/mod.rs:390-396`).
3. Tick the scheduler again with the repository behaving normally — expected: the announcement is re-selected and the notification is dispatched. Actual today: the announcement is not re-selected (it is already `published`), so the notification is permanently lost.

## Suggested approach
1. Add an additive migration under `backend/migrations/` introducing `notified_at TIMESTAMPTZ NULL` on both `announcements` and `votes` (one migration per table is fine; batch is also fine). No backfill needed — historical rows implicitly have `notified_at = NULL` and will be replayed once, so consider gating by `published_at > <migration-time>` if replay would be undesirable for archival rows (call it out in the migration comment).
2. In `announcement_repo.publish_scheduled()` / equivalent vote-lifecycle repo calls: leave `notified_at` untouched (NULL) — do not stamp it here.
3. Introduce a sibling scheduler pass, e.g. `dispatch_pending_announcement_notifications`, that: `SELECT ... WHERE status = 'published' AND notified_at IS NULL LIMIT N` → resolve targets → dispatch → on success `UPDATE ... SET notified_at = now()`. On failure log and leave `notified_at` NULL for the next tick.
4. Wire the new pass into `Scheduler::tick` right after `publish_scheduled_announcements` (or fold into a single unified pass that first publishes, then dispatches). Same shape for votes in `scheduler/votes.rs`.
5. Add a max-retry guard (`notify_attempts` column or a bounded backoff on `updated_at`) so a permanent target-resolution failure cannot spin forever — cap at e.g. 5 attempts, then log `notify_permanent_failure` and stamp `notified_at = now()` with a companion `notify_failed = true` flag.
6. Update the existing scheduler unit tests in `scheduler/mod.rs` (module `tests` around line 1437+) to exercise the retry path; add a new integration test under `backend/servers/api-server/tests/suites/` that seeds a failing-then-recovering repo stub and asserts eventual dispatch.
7. Do NOT change PR #2608's logged-fallback behavior — that's the observability half; this plan is the durability half.

## Alternatives considered
- **Wrap publish + dispatch in a single DB transaction so publish rolls back on dispatch failure.** Rejected because the notification service is an external side-effect (Redis fanout / push service) whose success cannot be rolled back — a transaction would cause double-notification on the retry after a network flake between "notify sent" and "commit succeeded".
- **Add an in-memory retry loop inside the current tick.** Rejected because the process can be killed between publish (already committed to DB) and the in-memory retry — the on-disk `notified_at` sentinel survives process restarts, an in-memory queue does not.

## Root-cause trace
1. Symptom: an announcement is published and the operator confirms it via `GET /announcements/:id` (status=published), but no user receives the corresponding push/in-app notification — server logs show the `"Failed to resolve announcement notification targets; skipping dispatch"` warning at `scheduler/mod.rs:390-396`.
2. ← Immediate cause at `scheduler/mod.rs:387-399`: the `match get_announcement_target_users(...)` arm coerces the error into an empty `Vec::new()`, so the subsequent `if !target_user_ids.is_empty()` at line 401 short-circuits the dispatch.
3. ← Upstream cause at `scheduler/mod.rs:365`: `announcement_repo.publish_scheduled().await?` commits the "published" state *before* dispatch is attempted. Because `announcement_repo::publish_scheduled` marks the row `status = 'published'` and returns, any subsequent selector (`SELECT ... WHERE status = 'scheduled'`) will never re-emit this announcement.
4. Origin: the publish-then-notify split predates PR #2608 — PR #2608 (`code-review-api-core-scheduler-rs-silent-target-err`) added the *observation* half (surface the error) but explicitly left the *durability* half out of scope (see PR #2608 commit body). The fire-once shape has therefore existed since the scheduler's initial announcement pass.

## Test plan
- [ ] `backend/servers/api-server/src/services/scheduler/mod.rs` — expand the existing `#[cfg(test)]` module (around line 1437) with a two-tick test that injects a target-resolution failure on tick 1 and asserts dispatch on tick 2 after `notified_at` is stamped.
- [ ] New integration test at `backend/servers/api-server/tests/suites/scheduler_notification_retry_tests.rs` (mounted in `backend/servers/api-server/tests/suite_5.rs`) that uses a real Postgres fixture (`sqlx::test`) to seed a scheduled announcement, run the tick twice with the notification service instrumented to fail on the first call, and assert the announcement's `notified_at` is populated only after the second tick.
- [ ] Regression scenario: seed 3 concurrently-scheduled announcements, force targets-resolution to fail for one specific announcement, tick — assert the other two dispatch normally and the failing one stays `notified_at IS NULL`; tick again with the failure cleared and assert the third dispatches on the retry.
- [ ] Command to run locally: `cargo test -p api-server --test suite_5 scheduler_notification_retry` (or `--all-targets` for the wider net).

## Out of scope
- Refactoring the notification-service transport layer (Redis fanout / push) — this plan operates entirely at the scheduler + repository layer.
- Retroactive "notify all historically-published announcements" backfill — the migration is additive with `notified_at NULL`, but a `now()` cutoff can be set at migration time if replay of archival rows is undesirable (call it out in the migration comment).
- Vote-result notification content changes — the plan only fixes the fire-once shape, not the payload of the notification.
- Any change to PR #2608's logged-fallback observability behavior — keep those log lines as-is.

## After-merge
- Move this file to `plans/_archive/bug-scheduler-notifications-fire-once.md`
- Mark the matching `backlog.json` row as `status: "done"`
