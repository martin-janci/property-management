# code-review-legal-rs-db-error-leak

**Vector:** security
**Score:** 2
**Source:** rotating-expert-review segment=api-handlers (signals/2026-10-25-api-handlers-tier1d.json)
**Confidence:** high

## Hypothesis

`backend/servers/api-server/src/routes/legal.rs` defines a module-local `db_error(msg, e)` helper (line 105) that logs the raw error server-side with `tracing::error!` and then returns `Json(ErrorResponse::new("DB_ERROR", e.to_string()))` to the API client. The `.to_string()` on an `sqlx::Error` leaks table names, column names, and SQL fragments through the public API — the sanitised-error convention already proven in `routes/compliance.rs:67` (regression-tested at `compliance.rs:907`) is not applied here. Collapsing the client-facing message to a static opaque string closes the leak without losing operator detail (tracing already captures it).

## Evidence

- `backend/servers/api-server/src/routes/legal.rs:105` — `fn db_error(msg, e) { tracing::error!(…); Json(ErrorResponse::new("DB_ERROR", e.to_string())) }` returns raw `sqlx::Error` Display.
- `backend/servers/api-server/src/routes/compliance.rs:67` — the convention: static opaque message, operator detail in `tracing::error!` only.
- `backend/servers/api-server/src/routes/compliance.rs:907` — regression test guarding that convention.
- Signal id `code-review-legal-rs-db-error-leak` (confidence=high, +2, vector=security) in `.research/signals/2026-10-25-api-handlers-tier1d.json`.

## Files

- `backend/servers/api-server/src/routes/legal.rs:105`
- `backend/servers/api-server/src/routes/compliance.rs:67`

## Dependencies

<!-- No prior work must land first; the fix is single-file and the convention is already in-tree. -->

## Required capabilities

- [x] C1 — Systematic debugging (bug / security root-cause)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception

Mode: cloud-ok

## Repro steps

1. Trigger any `legal.rs` handler against a database that will fail a query (e.g. force a `UNIQUE` violation, point at a schema missing a column, or stop the Postgres container mid-request). Hit any of the module's endpoints that call `db_error(...)`.
2. Expected: response body carries a static opaque `"Internal server error"` string under `message`. Actual: response body carries the full `sqlx::Error` Display including table / column names and the SQL fragment.

## Suggested approach

1. In `backend/servers/api-server/src/routes/legal.rs`, change `db_error`'s returned `ErrorResponse::new("DB_ERROR", e.to_string())` to `ErrorResponse::new("DB_ERROR", "Internal server error")` (or match the exact string used by `compliance.rs:67`).
2. Keep the `tracing::error!(…, error = ?e, …)` call unchanged so operators still see the detail in logs.
3. Add a Rust integration test mirroring the shape at `compliance.rs:907` under `backend/servers/api-server/tests/suites/` — hit a `legal.rs` endpoint, induce a DB failure, assert the response message does NOT contain the raw `sqlx::Error` Display markers (e.g. `relation "`, `column "`, `::` type casts).
4. Grep the module for any direct `e.to_string()` leaks outside the helper (`rg 'to_string\\(\\)' backend/servers/api-server/src/routes/legal.rs`) and route them through the sanitised helper.
5. Run `cargo clippy -p api-server --all-targets -- -D warnings` and the new integration test to confirm no regression.

## Alternatives considered

- **Return a typed error enum with a serializer that strips Display** — rejected because it widens the diff across all `legal.rs` handlers to replicate what `compliance.rs` already got right with a single helper edit; the module-local helper is the exact right scope.
- **Only redact in release builds, leave Display in debug** — rejected because the regression test in `compliance.rs:907` doesn't branch on build profile; staying consistent with the proven convention keeps the audit surface flat.

## Root-cause trace

1. Symptom: API clients hitting `legal.rs` endpoints receive raw `sqlx::Error` Display in `message`.
2. ← `backend/servers/api-server/src/routes/legal.rs:105` — `db_error` injects `e.to_string()` directly into `ErrorResponse::new`.
3. ← Module author added `db_error` as a local shortcut without routing through the sanitised helper used in `compliance.rs:67`.
4. Origin: pre-existing in the first commit of `legal.rs` (helper written as a one-shot before the sanitisation convention was documented).

## Test plan

- [ ] New integration test under `backend/servers/api-server/tests/suites/legal_db_error_sanitize_test.rs` — mirrors the regression at `compliance.rs:907`; induces a DB failure and asserts the response body message equals the opaque string and does NOT contain `relation "`, `column "`, or SQL fragments.
- [ ] Regression scenario: run the compliance-sanitisation test alongside the new test to confirm both pass.
- [ ] Local command: `cargo test -p api-server --test legal_db_error_sanitize_test` plus `cargo clippy -p api-server --all-targets -- -D warnings`.

## Out of scope

- Auditing other route modules for the same leak pattern (track separately via `code-review-api-handlers-db-error-leak`).
- Introducing a crate-wide typed-error enum (see Alternatives).
- Changing the `tracing::error!` call signature or log level.

## After-merge

- Move this file to `plans/_archive/code-review-legal-rs-db-error-leak.md`
- Mark the matching `backlog.json` row as `status: "done"`
