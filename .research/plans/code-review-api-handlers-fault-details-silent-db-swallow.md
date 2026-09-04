# code-review-api-handlers-fault-details-silent-db-swallow

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review 2026-09-04 (api-handlers segment)
**Confidence:** high

## Hypothesis
`get_fault_details` in `backend/servers/api-server/src/routes/faults.rs` fetches the parent fault row with proper 500-on-DB-error handling, then hydrates `timeline` and `attachments` via `.await.unwrap_or_default()`, which coalesces every sqlx error (connection drop, RLS/tenant-context misconfig, permission denied, timeout) into empty `Vec`s and emits no `tracing::error!`. Callers see an authoritative-looking 200 body with `timeline: []` and `attachments: []` — indistinguishable from "this fault genuinely has no events" — and operators have zero signal that the underlying repo failed. The same anti-pattern recurs in `listings.rs::get_photos`/`get_syndications` on the public listing-detail response and in `documents/core.rs` (including a `count().unwrap_or(0)` that silently shows 0 total on pagination). Fix: replace the fail-open `unwrap_or_default()`/`unwrap_or(0)` with the same `Err → tracing::error! + 500 INTERNAL_ERROR` block already used for the parent `find_by_id_with_details_for_org` call in each handler.

## Evidence
- `backend/servers/api-server/src/routes/faults.rs:722-732` — `fault_repo.list_timeline(id, is_manager).await.unwrap_or_default()` and `.list_attachments(id).await.unwrap_or_default()` after the parent lookup already maps DB errors to 500 (`.map_err(|e| { tracing::error!(...); (500, INTERNAL_ERROR) })`).
- `backend/servers/api-server/src/routes/listings.rs:338-343` — same fail-open pattern on `get_photos` and `get_syndications` in the public listing detail path.
- `backend/servers/api-server/src/routes/documents/core.rs:699-741` — same on document metadata hydration; `.count().unwrap_or(0)` at :741 silently returns `total = 0` on any pagination-count DB error.
- 2026-09-04 rotating-expert-review (api-handlers segment) — pattern flagged, cross-checked against backlog.json (no dup).

## Files
- `backend/servers/api-server/src/routes/faults.rs`
- `backend/servers/api-server/src/routes/listings.rs`
- `backend/servers/api-server/src/routes/documents/core.rs`

## Dependencies
_(none)_

## Required capabilities
- [x] C1 — Systematic debugging (bug vector, correctness)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived):** pure Rust backend edit, verifiable via `cargo test -p api-server` in the cloud runner.

Mode: cloud-ok

## Repro steps
1. Point the api-server at a Postgres pool where `fault_timeline` queries fail (e.g. drop `SELECT` grant on `fault_timeline`, or wire a poisoned pool that returns `sqlx::Error::WorkerCrashed`).
2. Ensure the parent `faults` row still loads normally.
3. `curl -H "authorization: bearer $TOKEN" http://localhost:8080/api/v1/faults/{existing_id}` (use whatever bearer-token header shape your local api-server accepts).
4. Expected: `500 INTERNAL_ERROR` with a `tracing::error!` line in the server log.
5. Actual today: `200 OK` with body `{ ..., "timeline": [], "attachments": [] }` and no error log.

## Suggested approach
1. In `backend/servers/api-server/src/routes/faults.rs::get_fault_details`, extract a small `internal_error(op: &str, e: sqlx::Error) -> (StatusCode, ErrorResponse)` helper (or reuse whatever helper the parent `find_by_id_with_details_for_org` already routes through) and replace both `.await.unwrap_or_default()` calls at :722 and :732 with `.await.map_err(|e| internal_error("list_timeline", e))?` / `.await.map_err(|e| internal_error("list_attachments", e))?`.
2. Repeat the substitution at `backend/servers/api-server/src/routes/listings.rs:338` (`get_photos`) and `:343` (`get_syndications`) — same helper; the site is inside the public listing-detail path, so match whatever 5xx envelope the surrounding handler uses.
3. Repeat at `backend/servers/api-server/src/routes/documents/core.rs:699,704,741` — the count path (`.count().unwrap_or(0)`) needs to bubble a 500 rather than silently returning `total: 0` on a pagination-count error.
4. Emit a structured `tracing::error!(operation = op, fault_id = %id, error = %e, "…")` line inside `internal_error` so ops keeps one log grep pattern per handler.
5. Add a shared `#[cfg(test)]` fixture (in `backend/servers/api-server/tests/common/mod.rs` if present, or per-file otherwise) that constructs a fault whose `fault_timeline` query fails deterministically — a repo trait stub is fine.
6. Do NOT rewire the caller contract otherwise; do NOT change response shape on the happy path.
7. Add 3 focused regression tests (see *Test plan*).

## Alternatives considered
- **Return `timeline: null` + a `timeline_error` field on 200** — rejected because it forces every existing client (ppt-web, mobile-rn, mobile-native) to learn a new degradation state; a 500 is the honest signal and ops already alerts on it.
- **Only add `tracing::error!` + keep the empty-vec response** — rejected because the alert would fire but callers would still render an authoritative-looking "no events" screen; the observability and the UX have to be fixed together to be trustworthy.

## Root-cause trace
1. Symptom: `GET /api/v1/faults/{id}` returns 200 with `timeline:[]` and `attachments:[]` even though the parent fault has known events; no error appears in the api-server log.
2. ← `backend/servers/api-server/src/routes/faults.rs:722` — `fault_repo.list_timeline(id, is_manager).await.unwrap_or_default()` swallows every `Err(sqlx::Error)` and hands back `vec![]`.
3. ← Same `.unwrap_or_default()` (`listings.rs:338,343`) and `.unwrap_or(0)` (`documents/core.rs:741`) — the pattern was propagated by copy-paste across three handlers.
4. Origin: introduced piecemeal per handler over the last several months (no single PR); the parent `find_by_id_with_details_for_org` call in each handler adopted proper `map_err` while the hydration follow-ups did not, producing a mixed contract inside the same function. The fix is to normalise on the parent's error-mapping helper.

## Test plan
- [ ] `backend/servers/api-server/tests/faults_details_error_tests.rs` — inject a repo stub whose `list_timeline` returns `Err(sqlx::Error::WorkerCrashed)`; assert the handler returns `500` with `INTERNAL_ERROR` body and that a `tracing::error!` line with `operation="list_timeline"` is captured (`tracing_test::traced_test`).
- [ ] Symmetric test for `list_attachments` and (in a per-crate location) for `documents/core.rs::count` — same shape.
- [ ] Regression: existing green-path integration test on `GET /api/v1/faults/{id}` still returns 200 with real timeline entries — no behaviour change on the happy path.
- [ ] Command: `cargo test -p api-server --test faults_details_error_tests` and `cargo test -p api-server` for the full suite.

## Out of scope
- Rewriting the `documents/core.rs` pagination contract beyond making the count query fail loudly (any deeper refactor stays out of this PR).
- Broader audit for `.unwrap_or_default()` on `Result<Vec<_>, sqlx::Error>` across the whole crate — this plan only fixes the three sites named above; a follow-up sweep issue can catch the rest.
- Migrations, schema changes, or client-side contract updates.

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-fault-details-silent-db-swallow.md`
- Mark the matching `backlog.json` row as `status: "done"`
