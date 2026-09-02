## code-review-api-handlers-neighbors-raw-db-error-leak

**Vector:** security
**Score:** 3
**Source:** pm-security (Phase 1.6 2026-09-02) + regression pattern from merged PR #2925
**Confidence:** medium

## Hypothesis
`backend/servers/api-server/src/routes/neighbors.rs` returns raw `sqlx::Error` `Display` output to the client body via `Json(ErrorResponse::new("DB_ERROR", e.to_string()))` on 4 handlers (lines 138, 149, 190, 229). This is the same information-leak pattern PR #2925 just fixed in `compliance.rs` — the sqlx error text can carry SQL fragments, table/column names, and constraint identifiers. Route the 4 sites through a local `db_error(msg, e)` helper (mirroring the shape used by `admin_tenants.rs`, `legal.rs`, `vendors/shared.rs`) so the client sees only a static message while `tracing::error!` records the raw error server-side.

## Evidence
- `backend/servers/api-server/src/routes/neighbors.rs:138` — `get_neighbors` list: `Json(ErrorResponse::new("DB_ERROR", e.to_string()))`
- `backend/servers/api-server/src/routes/neighbors.rs:149` — `count_neighbors`: same raw leak (paginate-total path)
- `backend/servers/api-server/src/routes/neighbors.rs:190` — `get_privacy_settings`: same raw leak
- `backend/servers/api-server/src/routes/neighbors.rs:229` — `update_privacy_settings`: same raw leak
- 9 other route modules already use a `db_error(msg, e)` helper (see `admin_tenants.rs:50`, `legal.rs:105`, `vendors/shared.rs:23`) — neighbors.rs never adopted it; `compliance.rs` was patched by #2925 with a source-level regression guard.

## Files
- `backend/servers/api-server/src/routes/neighbors.rs`
- `backend/servers/api-server/src/routes/compliance.rs`

## Dependencies
_None — self-contained fix, small blast radius._

## Required capabilities
- [x] C1 — Systematic debugging (bug/security vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Cause any handler in `neighbors.rs` to fail its DB call (e.g. force a sqlx error by pointing the test DB at a schema-drifted state, or wrap the query to short-circuit `Err(sqlx::Error::RowNotFound)`).
2. Hit `GET /api/v1/buildings/{id}/neighbors` as an authenticated resident.
3. **Expected:** response body is `{"code":"DB_ERROR","message":"<static string>"}`; the raw `sqlx::Error` `Display` is present only in server logs (via `tracing::error!`).
4. **Actual:** response body contains the raw sqlx error text (e.g. `"error returned from database: relation \"neighbors\" does not exist"`), leaking schema detail to the authenticated caller.

## Suggested approach
1. Add a private module helper matching the neighbors return shape:
   ```rust
   fn db_error(msg: &'static str, e: sqlx::Error) -> (StatusCode, Json<ErrorResponse>) {
       tracing::error!(error = ?e, "{}", msg);
       (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new("DB_ERROR", msg)))
   }
   ```
   Place it near the top of `backend/servers/api-server/src/routes/neighbors.rs` (mirroring `admin_tenants.rs:50` / `legal.rs:105`).
2. Replace the 4 `.map_err(|e| { tracing::error!(...); (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new("DB_ERROR", e.to_string()))) })` blocks with `.map_err(|e| db_error("Failed to <op>", e))?` at lines 138, 149, 190, 229. Keep the existing static context messages (`"Failed to get neighbors"`, `"Failed to count neighbors"`, etc.) — they become the client-visible copy.
3. Add a source-level guard test alongside the existing `compliance.rs::db_error_does_not_leak_raw_detail_to_client` (`compliance.rs:907`) — new test in `neighbors.rs` mod tests that constructs a synthetic sqlx error, calls `db_error("Failed to get neighbors", err)`, and asserts the response body's `message` field equals `"Failed to get neighbors"` (NOT the raw `Display`).
4. Do NOT widen the change beyond the 4 sites in `neighbors.rs` (this plan stays neighbors-scoped; `migration.rs` and unrelated leak audits are separate follow-ups).

## Alternatives considered
- **Import a shared `db_error` from a common util module.** Rejected because each of the 10 existing sites uses a slightly different signature (`(msg, e)` vs `(context)` vs `(e)`) and different return type (`Json<ErrorResponse>` vs `String` vs `Response`); introducing a workspace helper is a broader refactor. Consistent-per-module local helper matches the current codebase norm.
- **Only sanitize the message, keep returning `e.to_string()` in `debug` builds.** Rejected because the client body reaches production regardless; conditional behavior between builds is a footgun — one static message everywhere is auditable.

## Root-cause trace
1. Symptom: authenticated client receives an ErrorResponse whose `message` field contains raw sqlx Display text (schema/column/constraint identifiers) on any neighbors handler DB failure.
2. ← `backend/servers/api-server/src/routes/neighbors.rs:138,149,190,229` — `.map_err(|e| ... e.to_string() ...)` places the raw error into the client body.
3. ← historic pattern predates the `db_error` helper convention adopted in other route modules; `neighbors.rs` was landed before the compliance-audit sweep that introduced sanitizers elsewhere.
4. Origin: initial land of neighbors route (predates recent hardening PRs #2920 for compliance and #2925 for compliance regression).

## Test plan
- [ ] New source-level guard test in `neighbors.rs` mod tests: `fn db_error_does_not_leak_raw_detail_to_client` — construct a synthetic `sqlx::Error::RowNotFound`, call the new `db_error("Failed to get neighbors", err)`, assert response `Json<ErrorResponse>.message == "Failed to get neighbors"`.
- [ ] Regression assertion: `grep -n 'e\.to_string()' backend/servers/api-server/src/routes/neighbors.rs` returns 0 matches inside `Json(ErrorResponse::new(...))` bodies (encoded either as a byte-level test or a compile-time macro assertion mirroring the compliance regex guard).
- [ ] Command: `cargo test -p api-server routes::neighbors -- --nocapture`

## Out of scope
- `migration.rs` broad `e.to_string()` audit (separate plan; scope + reachability need pm-security follow-up first).
- Adding a workspace-wide `db_error` helper crate.
- Fixing `route-groups-no-protectedroute` (separate backlog item, higher-effort UI plan).

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-neighbors-raw-db-error-leak.md`
- Mark the matching `backlog.json` row as `status: "done"`
