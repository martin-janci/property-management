# bug-coderev-api-core-scheduler-dup-exec

**Vector:** bug
**Score:** 3
**Source:** static-review (Phase 1.5 api-core segment, routine 2026-07-17)
**Confidence:** high

## Hypothesis
`ReportScheduler::fire_due_report_schedules` records a `report_executions` row and then advances `next_run_at` in two separate, non-transactional statements (backend/servers/api-server/src/services/scheduler.rs:1451-1484). If `record_execution` succeeds but `advance_after_run` errors (transient RLS/DB blip, deadlock, or the same failure-mode class that motivated PR #2318), `next_run_at` stays unchanged and the schedule remains due — so the next scheduler tick re-fires the same schedule and inserts another `pending` execution row. As long as the advance keeps erroring, executions duplicate unbounded. Wrap the two writes in a single `sqlx::Transaction` so record + advance either both land or neither does; on failure the schedule stays due but no phantom execution row exists.

## Evidence
- backend/servers/api-server/src/services/scheduler.rs:1451-1484 — the two writes are chained by early-`continue` on error, not by a transaction.
- backend/crates/db/src/repositories/report_schedule.rs — both `record_execution` and `advance_after_run` accept `&mut PgConnection`, so they already fit inside a single tx.
- Story 81.2 / PR #2318 (the RLS-context fix for the scheduler worker) established that transient RLS mismatches in this exact worker path do occur — the same class of failure would trip this dup-exec bug today.
- Same failure shape as `portal_webhook_events` needed a dedup net (PR #2376 / issue #2358) — this is the mirror image on the scheduler side.

## Files
- `backend/servers/api-server/src/services/scheduler.rs:1451`
- `backend/crates/db/src/repositories/report_schedule.rs`
- `backend/crates/db/tests/report_schedule_scheduler_rls_tests.rs`

## Dependencies
_(none — self-contained fix)_

## Required capabilities
- [x] C1 — Systematic debugging (bug vector)
- [ ] C2 — Seed data
- [x] C3 — Dev instance running (needs `stack up pm-local` to exercise the scheduler tick loop against Postgres)
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode:** `Mode: cloud-ok` (C4/C5 not ticked; C3 available via `ppt-bridge` MCP)

## Repro steps
1. Insert a schedule with `next_run_at <= now` and a valid `cadence` (e.g. daily 06:00 UTC).
2. In the scheduler tick, force `advance_after_run` to fail once (e.g. temporarily stub it to `Err(sqlx::Error::PoolTimedOut)`; or point the schedule at a canonical cadence that returns `None` and simultaneously break the parking write).
3. Expected: exactly one `report_executions` row and `next_run_at` recomputed (or NULL-parked).
   Actual: after N ticks where advance keeps erroring, there are N `pending` `report_executions` rows for the same schedule — every tick inserts another.

## Suggested approach
1. In `fire_due_report_schedules` (scheduler.rs:~1450), begin a `let mut tx = conn.begin().await?;` before the `record_execution` call and pass `&mut *tx` into both `record_execution` and `advance_after_run`.
2. Replace the two `continue` branches with `tx.rollback().await` on record error, and `tx.rollback().await` on advance error (retaining the log). On the happy path, `tx.commit().await` after `advance_after_run` succeeds.
3. Preserve today's semantics: on failure the schedule stays due (next_run_at unchanged) — the only change is that no phantom execution row is left behind.
4. Extend `backend/crates/db/tests/report_schedule_scheduler_rls_tests.rs` with a regression test: mock/stub `advance_after_run` to fail once, tick the scheduler twice, assert exactly zero `report_executions` rows exist after both ticks and the schedule is still due.
5. Verify the surrounding `fired += 1;` accounting still matches reality (increment only inside the committed branch).
6. Run `cargo test -p api-server report_schedule` and `cargo test -p db report_schedule` locally.

## Alternatives considered
- **Idempotency key on `report_executions` (unique index on `(schedule_id, tick_bucket)`)** — rejected because it hides the underlying two-write race behind a DB constraint that then silently rejects the second insert; downstream code still assumes each `record_execution` corresponds to a real fire, so a rejected row would break the `fired` counter and metrics without fixing the schedule-not-advanced side of the problem.
- **Retry `advance_after_run` in a loop after a failed record** — rejected because a persistently-failing advance would spin the worker tick and starve every other schedule; a transaction cleanly rolls the whole tick back and lets the next tick try afresh.

## Root-cause trace
1. Symptom: duplicate `pending` `report_executions` rows for one schedule after any tick where `advance_after_run` errors.
2. ← `advance_after_run` fails and returns Err at scheduler.rs:1476-1483.
3. ← `next_run_at` is unchanged (no write), but `record_execution` already wrote a row at scheduler.rs:1451-1460.
4. ← The two writes are separate DB round-trips inside a single `while let Some(schedule) = due.pop()` iteration, not a transaction.
5. Origin: introduced when the two-step "record then advance" pattern was added for Story 81.2 (report-scheduler history tracking). PR # for the original scheduler flow is the same one that later needed the RLS-context fix (#2318).

## Test plan
- [ ] Regression test in `backend/crates/db/tests/report_schedule_scheduler_rls_tests.rs`: stub `advance_after_run` to fail once, tick twice, assert `report_executions.len() == 0` and schedule still due.
- [ ] Happy-path test still green: normal fire → 1 execution row + `next_run_at` advanced.
- [ ] Command: `cargo test -p api-server --test report_schedule_scheduler_rls_tests` and `cargo test -p db report_schedule`.

## Out of scope
- Adding a unique index on `report_executions` (see Alternatives — separate design decision, not needed for this fix).
- Reworking the cadence-parking (`next.is_none()` → NULL) path — this plan only fixes the record/advance atomicity.
- Extending the same pattern to other scheduler worker paths (announcements, push fanout) — file follow-ups instead if the same shape exists there.

## After-merge
- Move this file to `plans/_archive/bug-coderev-api-core-scheduler-dup-exec.md`
- Mark the matching `backlog.json` row as `status: "done"`
