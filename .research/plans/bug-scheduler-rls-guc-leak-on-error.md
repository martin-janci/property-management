# bug-scheduler-rls-guc-leak-on-error

**Vector:** bug
**Score:** 3
**Source:** code-review api-core 2026-07-17 (dispatcher-tier1d dev-review); backend/servers/api-server/src/services/scheduler.rs:1416
**Confidence:** high

## Hypothesis
`Scheduler::fire_due_report_schedules` sets the session-level `app.global_read` GUC to `true` for a cross-org SELECT, then calls `set_global_read_context(&mut *conn, false)` and immediately `?`-propagates the error. If that clear-flag call itself errors, the connection is returned to the sqlx pool with the global-read GUC still `true`. A subsequent borrower that runs a query without first setting its own tenant context will pass the SELECT-only `global_read` leg of the RLS policies added in migration 00218 and read cross-org rows. Clearing the flag with a `defer`-style guard (mirroring `workflow_executor.rs`'s `guard.release()`) closes the leak with a one-file change.

## Evidence
- `backend/servers/api-server/src/services/scheduler.rs:1400-1426` — `?` on `set_global_read_context(conn, false)` short-circuits before `clear_request_context`; the pooled connection is dropped back with `SET app.global_read = true` still active on the session.
- `backend/crates/db/src/tenant_context.rs` — `set_global_read_context` issues a session-level `SET`, not a `SET LOCAL`, so it persists past the tx/borrow boundary.
- `backend/migrations/00218_global_read_policies.sql` — the RLS SELECT-only `global_read` policy leg is the read path a leaky connection would satisfy without a tenant.
- `backend/servers/api-server/src/services/workflow_executor.rs:260,328,744,771,871,922,959` — reference pattern: RAII `guard.release().await` idiom already used across the workflow executor to ensure GUC teardown regardless of downstream errors.
- Dispatcher action-list entry `code-review-api-core-scheduler-rls-leak` (`.research/management/action-list.json`) — surfaced by tier1d review 2026-07-17T18:17:03Z.

## Files
- `backend/servers/api-server/src/services/scheduler.rs`
- `backend/crates/db/src/tenant_context.rs`
- `backend/servers/api-server/src/services/workflow_executor.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**

Mode: cloud-ok

## Repro steps
1. Instrument `db::tenant_context::set_global_read_context` (test-only build) so the second call (with `false`) returns `Err(...)` once.
2. Drive `Scheduler::fire_due_report_schedules` end-to-end via a test that stubs `report_schedule_repo.get_due_schedules` to return `Ok(vec![])` so the function reaches line 1416 and short-circuits on `?`.
3. Immediately re-acquire the same underlying connection from the pool and run a bare `SELECT current_setting('app.global_read', true)`.
4. Expected: setting is `off`/`NULL`. Actual (bug): setting is `on` — the flag survived the pool return.

## Suggested approach
1. Introduce a lightweight RAII helper (either in `db::tenant_context` or scoped to `scheduler.rs`) mirroring the shape of the workflow-executor `guard.release()` pattern: `set_global_read_context(&mut *conn, true).await?; let guard = GlobalReadGuard::new(); … ; guard.release(&mut *conn).await?;`.
2. Ensure `Drop` for the guard schedules a best-effort `SET app.global_read = off` on the connection when release wasn't called (fallback for panic paths). Because sqlx `Drop` cannot await, drop-time cleanup wraps a `blocking_send` on a maintenance channel *or* — simpler — leave `Drop` empty and rely on the explicit release-in-all-paths pattern used elsewhere; the acceptable choice is whichever matches `workflow_executor.rs`.
3. Refactor `fire_due_report_schedules` (`scheduler.rs:~1395-1430`) so the release runs before every early-return / `?` path; the current `let due = due?;` on line 1420 already sits after the clear, but the clear itself is the leak — move the clear into a helper that runs in a `defer`-shape (`Result::and_then` or explicit branch on `due`'s Ok/Err).
4. Audit the rest of `scheduler.rs` for other `set_global_read_context(true)` call sites and apply the same guard where the paired `false` sits after a `?`. Grep: `set_global_read_context.*true` in `services/scheduler.rs`.
5. Add a regression test in `backend/servers/api-server/tests/scheduler_rls_context_test.rs` that uses a fault-injecting mock (or a `sqlx` pool with `min_connections=1`, `max_connections=1` and a query that flips the setting) to assert the setting is `off` after both the happy path *and* the injected-error path.
6. Run `cargo sqlx prepare --workspace` if new queries are added; regenerate offline data.
7. `cargo fmt && cargo clippy -p api-server -- -D warnings && cargo test -p api-server scheduler_rls_context_test`.

## Alternatives considered
- **Use SET LOCAL inside an explicit transaction** — rejected because the surrounding function currently reads without a tx (cross-org SELECT then per-schedule writes each own their tenant context); wrapping just the SELECT in a tx changes the connection lifecycle and would need parallel changes to the per-schedule loop.
- **Retry the clear on failure inline** — rejected because a repeated `?` still leaves the same leak if the retry fails; the guard-release pattern gives us panic-safety and matches the workflow-executor precedent, so consistency wins.

## Root-cause trace
1. Symptom: pooled connection returned to sqlx still has session GUC `app.global_read = true`; subsequent tenant-less query passes the RLS SELECT `global_read` policy leg.
2. ← `Scheduler::fire_due_report_schedules` at `backend/servers/api-server/src/services/scheduler.rs:1416` uses `?` on the clear-flag call, aborting before any teardown.
3. ← `db::tenant_context::set_global_read_context` in `backend/crates/db/src/tenant_context.rs` issues session-level `SET`, so absent an explicit reset the value persists across pool borrows.
4. Origin: migration `backend/migrations/00218_global_read_policies.sql` introduced the SELECT-only `global_read` policy leg (2026-05-xx); the scheduler's cross-org read was retrofitted around it without a symmetric release-on-error path.

## Test plan
- [ ] `backend/servers/api-server/tests/scheduler_rls_context_test.rs::global_read_cleared_on_error` — inject failure in the paired-clear call, assert `SELECT current_setting('app.global_read', true)` on the returned connection is `off`/`NULL`.
- [ ] `backend/servers/api-server/tests/scheduler_rls_context_test.rs::global_read_cleared_on_happy_path` — regression to lock in that the fix doesn't accidentally re-enter the leak from the success branch.
- [ ] `cargo test -p api-server scheduler_rls_context_test`

## Out of scope
- Broader audit of every `SET` GUC in the codebase — the plan targets scheduler.rs; other services already use the guard pattern.
- Changing the migration 00218 policy shape — the fix is at the app layer, not the DB policy.
- Refactoring the per-schedule write loop that runs after the SELECT — that block already owns its tenant context per iteration.

## After-merge
- Move this file to `plans/_archive/bug-scheduler-rls-guc-leak-on-error.md`
- Mark the matching `backlog.json` row as `status: "done"`
