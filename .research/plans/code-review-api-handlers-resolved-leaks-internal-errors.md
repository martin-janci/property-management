# code-review-api-handlers-resolved-leaks-internal-errors

**Vector:** security
**Score:** 2
**Source:** Phase 1.5 rotating expert review 2026-07-29 (api-handlers segment)
**Confidence:** high

## Hypothesis
`backend/servers/api-server/src/routes/layout/resolved.rs` embeds raw `sqlx::Error` and `serde_json::Error` text into 5xx client responses via its `err500(format!("db error: {e}"))` pattern. PR #2478's layout-hardening sweep introduced a defensive `internal_error()` helper in sibling `admin.rs` (line 29-37) that logs the real error server-side via `tracing::error!` and returns a generic `"internal server error"` message to the client — the sibling `tenant.rs` received the same treatment, but `resolved.rs` was skipped. The smallest safe change is to copy the same helper into `resolved.rs` (or lift it to `layout/mod.rs`) and replace every `err500(format!(…))` call with `.map_err(internal_error)`, closing the residual error-text leak on the *public* resolved-layout endpoint.

## Evidence
- `backend/servers/api-server/src/routes/layout/resolved.rs:36-41` — `err500` closure embeds `msg` verbatim into the JSON error body.
- `backend/servers/api-server/src/routes/layout/resolved.rs:82,88,92,107,115` — `.map_err(|e| err500(format!("db error: {e}")))?;` leaks raw `sqlx::Error` text.
- `backend/servers/api-server/src/routes/layout/resolved.rs:98,110,123` — leaks raw `serde_json::Error` text (e.g. `"stored published config invalid: {e}"`), which can echo stored payload fragments.
- `backend/servers/api-server/src/routes/layout/admin.rs:29-37` — sibling `internal_error()` helper introduced by PR #2478 with the exact defensive shape needed here.
- `backend/servers/api-server/src/routes/layout/tenant.rs:29-32` — same helper applied by PR #2478 to sibling handlers; resolved.rs was the only layout route file left out.

## Files
- `backend/servers/api-server/src/routes/layout/resolved.rs`
- `backend/servers/api-server/src/routes/layout/admin.rs`
- `backend/servers/api-server/src/routes/layout/tenant.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (security-adjacent, needs careful test authoring)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

Pure Rust change, no browser or device needed. `cargo test -p api-server --test layout_resolved_error_leak_tests` verifies via `axum::body::to_bytes` on the response, no live DB required beyond the existing test harness.

## Repro steps
1. On `dev` at any commit ≥ `f03c7c7` (2026-07-29), trigger `GET /api/v1/layout/resolved/{screen}?platform=web` with a valid tenant JWT for a screen whose *published* config is stored but has a serde-incompatible payload (or where the `layouts` connection pool fails mid-request).
2. Observed: the 500 response body contains `{"errors":["db error: <raw sqlx text>"]}` or `{"errors":["stored published config invalid: <raw serde msg>"]}` — leaking implementation details and, worse, echoing stored payload fragments.
3. Expected (after fix): the 500 response body contains `{"errors":["internal server error"]}` and the real error text appears only in the server-side `tracing::error!` log line — matching the admin.rs / tenant.rs contract.

## Suggested approach
1. **Lift `internal_error`** — move the helper defined at `admin.rs:29-37` up to a shared location, e.g. a new `fn internal_error(err: impl std::fmt::Display) -> (StatusCode, Json<ValidationErrorsResponse>)` in `backend/servers/api-server/src/routes/layout/mod.rs` (or a `layout/errors.rs` module). Update `admin.rs` and `tenant.rs` to import it from the shared location — no behavior change for them, just deduplication.
2. **Delete `err500` closure** in `resolved.rs` (lines 36-41). Its `err404` sibling stays (404s are safe to expose; they carry only static strings, no formatted error content).
3. **Replace call sites in `resolved.rs`** (lines 82, 88, 92, 98, 107, 110, 115, 123) — swap `.map_err(|e| err500(format!("db error: {e}")))?` for `.map_err(internal_error)?` and `.map_err(|e| err500(format!("stored published config invalid: {e}")))?` for `.map_err(internal_error)?`. The helper's `tracing::error!` line preserves the diagnostic context that the raw text used to leak into the response.
4. **Add regression test** at `backend/servers/api-server/tests/layout_resolved_error_leak_tests.rs`: stub a `layouts` table row whose `published` column contains invalid JSON (e.g. `'{"platforms": "not-a-map"}'::jsonb`), call the resolved endpoint, assert `status == 500` **and** `body.errors == ["internal server error"]` (no leaked field text). This test fails on `dev` today and passes after the fix — IG3 satisfied.
5. **Verify** with `cd backend && cargo test -p api-server layout_resolved_error_leak_tests && cargo clippy -p api-server -- -D warnings`. No frontend/mobile touch, so `just verify` narrows to the backend impact band.
6. **Do NOT** widen the scope to the other error-mapping patterns in api-server — this plan is strictly the layout/resolved.rs gap left by PR #2478. Similar leaks elsewhere are candidates for follow-up code-review-finding items.

## Alternatives considered
- **Keep `err500(format!(...))` but sanitize** — rejected because sanitizing free-form error text is error-prone (a new `sqlx::Error` variant or a nested `serde_json::Error` context could re-leak). The defensive posture PR #2478 established is "log real, return generic" — matching it exactly is safer than partial sanitization.
- **Return a per-error trace-id and store the raw error in a log-sink** — rejected as over-engineered for this scope. The tracing subscriber already correlates via request-id middleware; adding a client-visible trace-id here would touch the response schema (breaking `ValidationErrorsResponse`) and belongs in a broader "error UX" epic, not a security-gap fix.

## Root-cause trace

1. Symptom: `GET /api/v1/layout/resolved/{screen}` returns 500 with `{"errors":["db error: <raw sqlx text>"]}` — raw internal error text leaked to unauthenticated-tenant clients.
2. ← Immediate cause at `backend/servers/api-server/src/routes/layout/resolved.rs:36-41` — the local `err500` closure embeds `msg` (which callers build via `format!("db error: {e}")`) verbatim into the response body's `errors` array.
3. ← Upstream cause at `backend/servers/api-server/src/routes/layout/resolved.rs:82,88,92,98,107,110,115,123` — 8 call sites uniformly format the raw `sqlx::Error` / `serde_json::Error` into the leaked message.
4. Origin: PR #2478 (`fix(layout): review-hardening sweep — authz, publish TOCTOU, webhook replay, defensive rendering`, merged 2026-07-29) introduced the `internal_error` defensive helper in `admin.rs` and `tenant.rs` but did not migrate `resolved.rs`, leaving the pre-hardening leak pattern intact on the public resolved-layout endpoint.

## Test plan
- [ ] New: `backend/servers/api-server/tests/layout_resolved_error_leak_tests.rs::resolved_500_returns_generic_message` — inject a `published` row with structurally invalid JSON, assert `body.errors == ["internal server error"]`, no substring match on `serde` / `sqlx` / stored payload keys.
- [ ] Regression: `resolved_500_does_not_leak_db_error_text` — force a pool failure (e.g. via a closed-connection mock or a poisoned test pool), assert same generic body.
- [ ] Command: `cd backend && cargo test -p api-server --test layout_resolved_error_leak_tests`

## Out of scope
- Refactoring the `err404` closure in `resolved.rs` — 404 bodies carry only static strings, no leaked payload; leave alone.
- Similar `format!("db error: {e}")` patterns elsewhere in `backend/servers/api-server/src/routes/` — file separate code-review items if found; this plan is strictly the layout hardening-sweep gap.
- Introducing a `trace-id` in error responses — belongs in a broader error-UX epic.
- Changing `ValidationErrorsResponse` shape — the plan preserves the wire schema; only the free-text content of the `errors` array changes on the 500 path.

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-resolved-leaks-internal-errors.md`
- Mark the matching `backlog.json` row as `status: "done"`
