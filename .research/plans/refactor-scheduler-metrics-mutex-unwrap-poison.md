# refactor-scheduler-metrics-mutex-unwrap-poison

**Vector:** refactor
**Score:** 3
**Source:** code-review api-core 2026-07-17 (dispatcher-tier1d dev-review); backend/servers/api-server/src/services/scheduler.rs:168,314,377,497,549,832,858,878,1033,1163,1256,1323,1502,1511
**Confidence:** high

## Hypothesis
`Scheduler` stores its metrics in `Arc<std::sync::Mutex<SchedulerMetrics>>` and reaches into it with `.lock().unwrap()` on every scheduled tick (14 call sites). If any single lock holder panics while holding the guard, the mutex is poisoned and every subsequent `.unwrap()` panics — silently killing all scheduled work (announcement publish/unpin, vote activation/closing, vote reminders, overdue transitions, report schedule fires). Replacing `.unwrap()` with `.unwrap_or_else(|e| e.into_inner())` (or the `parking_lot::Mutex` primitive, which does not poison) is a mechanical, one-file refactor that eliminates the class of failure.

## Evidence
- `backend/servers/api-server/src/services/scheduler.rs:168,314,377,497,549,832,858,878,1033,1163,1256,1323,1502,1511` — 14 `.lock().unwrap()` sites; confirmed via `grep -c 'metrics\.lock()\.unwrap()'` → 14.
- `backend/servers/api-server/src/services/scheduler.rs:167-180` — `get_metrics()` panics on poison; called by admin/observability surface, so a poison there also takes out metrics readout.
- `std::sync::Mutex::lock` returns `Err(PoisonError)` after any prior panic under the guard; `PoisonError::into_inner()` retrieves the guard regardless of poison (standard recovery idiom for metrics-shaped state where a stale-but-live counter beats a dead scheduler).
- Dispatcher action-list entry `code-review-api-core-scheduler-mutex-poison` (`.research/management/action-list.json`) — surfaced by tier1d review 2026-07-17T18:17:03Z at Low severity.

## Files
- `backend/servers/api-server/src/services/scheduler.rs`

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
1. In a unit test, construct a `Scheduler` with the shared `Arc<Mutex<SchedulerMetrics>>` accessible.
2. Spawn a task that acquires the metrics guard and panics while holding it (`std::panic::catch_unwind(|| { let _g = m.lock().unwrap(); panic!("simulated"); })`).
3. Then call any scheduled tick function that reaches `metrics.lock().unwrap()` (e.g. `get_metrics()`).
4. Expected (after fix): the tick recovers via `into_inner()` and returns a valid `SchedulerMetrics` snapshot. Actual (bug, pre-fix): the tick panics again with `PoisonError`.

## Suggested approach
1. Introduce a private helper on `Scheduler` (or a free function in the same module): `fn lock_metrics<'a>(&'a self) -> MutexGuard<'a, SchedulerMetrics> { self.metrics.lock().unwrap_or_else(|e| e.into_inner()) }`.
2. Replace every `self.metrics.lock().unwrap()` and any inline `metrics.lock().unwrap()` (14 sites, enumerated in evidence) with a call to the new helper. Grep `metrics\.lock\(\)\.unwrap\(\)` in `scheduler.rs` to make sure none are missed.
3. Consider — decide during implementation, don't pre-commit — swapping `std::sync::Mutex` for `parking_lot::Mutex` since `parking_lot` doesn't poison at all. Only do this if `parking_lot` is already in the workspace dependency graph (`cargo tree -p api-server | grep parking_lot`); otherwise, adding a dependency for one mutex isn't worth it and the `into_inner` shim is enough.
4. Add a `#[cfg(test)]` unit test in `scheduler.rs` (or `tests/scheduler_metrics_poison_test.rs`) that reproduces the panic-under-guard scenario and asserts the next lock still yields a live snapshot.
5. `cargo fmt && cargo clippy -p api-server -- -D warnings && cargo test -p api-server`.
6. Verify no `metrics.lock().unwrap()` remains: `cargo clippy` won't catch it, so add a bespoke `grep` step in the PR description ("audited — 0 remaining sites").

## Alternatives considered
- **Wrap `SchedulerMetrics` in `Arc<AtomicU64>` fields** — rejected because the struct has ~15 counters and it would balloon the diff; the point is a minimal safety patch, not a re-architecture.
- **Swap directly to `tokio::sync::Mutex`** — rejected because the metrics writes are strictly synchronous and don't need async locking; using `tokio::Mutex` would leak `.await` into every tick handler (14 sites) for no functional gain.

## Root-cause trace
N/A — refactor doesn't need backward tracing. The poison-panic scenario is a general Rust `std::sync::Mutex` failure mode, not a regression of a specific commit.

## Test plan
- [ ] `backend/servers/api-server/tests/scheduler_metrics_poison_test.rs::poisoned_lock_still_yields_metrics` — panic while holding the guard in a background task, then assert subsequent locks succeed.
- [ ] Existing scheduler tick tests continue to pass (`cargo test -p api-server scheduler` — no regressions in `announcement`, `vote`, `overdue` paths).
- [ ] `cargo test -p api-server`

## Out of scope
- Broader mutex-poison audit across `api-server` — the plan is scoped to `scheduler.rs` because it's the module surfaced by dev-review and its long-running nature makes poison lethal in a way ad-hoc handler mutexes are not.
- Introducing a metrics-collection framework (Prometheus, OpenTelemetry) — that's an epic-sized refactor tracked elsewhere.

## After-merge
- Move this file to `plans/_archive/refactor-scheduler-metrics-mutex-unwrap-poison.md`
- Mark the matching `backlog.json` row as `status: "done"`
