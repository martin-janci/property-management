# code-review-api-handlers-oauth-client-error-leak

**Vector:** security
**Score:** 2
**Source:** commit db34026e (dispatcher Tier-1d api-handlers static review)
**Confidence:** high

## Hypothesis
`backend/servers/api-server/src/routes/oauth.rs` returns raw `sqlx::Error` text as the HTTP body from four OAuth-client management handlers (register / update / revoke / regenerate). Any caller who can reach these endpoints can enumerate the OAuth-client table schema by tripping constraint violations. The fix mirrors the compliance/budgets pattern: route the four leak sites through the `db_error()` sanitizer helper introduced by PR #2920 in `compliance.rs:67`.

## Evidence
- `grep -c 'e\.to_string()' backend/servers/api-server/src/routes/oauth.rs` → 4 (verified 2026-09-03)
- `backend/servers/api-server/src/routes/compliance.rs:67` — the `fn db_error(e: impl std::fmt::Display) -> (StatusCode, String)` sanitizer pattern PR #2920 established
- `backend/servers/api-server/src/routes/compliance.rs:907` — `fn db_error_does_not_leak_raw_detail_to_client()` regression-guard test PR #2925 added
- PR #2925 sibling — same defect class (raw sqlx error → HTTP body) in `compliance.rs:319`; the fix pattern is directly reusable here

## Files
- `backend/servers/api-server/src/routes/oauth.rs`
- `backend/servers/api-server/src/routes/compliance.rs`
- `backend/servers/api-server/tests/suites/oauth_client_registration_test.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Start api-server against a test tenant. Register an OAuth client, then `POST /oauth/register` again with the same `client_name` (or hit any register/update/revoke/regenerate path with a body that violates a NOT NULL / FK / uniqueness constraint on the oauth-client tables).
2. Expected: HTTP 500 body `{"code":"DB_ERROR","message":"internal database error"}` and the Postgres detail only in server logs. Actual today: HTTP 500 body containing the sqlx Display output — table names (`oauth_clients`), constraint names, sometimes SQL fragments.

## Suggested approach
1. Reuse the `db_error()` helper from `compliance.rs:67` — either import it (if it's already `pub(crate)`) or lift it into `util::errors` and reuse from both modules. Do NOT copy-paste a third variant.
2. Replace the four `e.to_string()` call sites in `oauth.rs` (register / update / revoke / regenerate handlers) with `db_error(e)`. Keep the existing `tracing::error!` calls unless the helper subsumes them.
3. Add a source-level guard test in `tests/suites/oauth_client_registration_test.rs` mirroring `compliance.rs:907`: read `oauth.rs` and assert it contains no `(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())` or `Json(ErrorResponse::new("DB_ERROR", e.to_string()))` fragments.
4. Add one end-to-end test: trip a real oauth-client uniqueness violation and assert the response body's `message` equals the sanitized string exactly.
5. Run `cargo test -p api-server oauth_client_registration_test` locally; then `cargo test -p api-server compliance` to catch any accidental helper-shape drift.

## Alternatives considered
- **Return `INTERNAL_SERVER_ERROR` with a fixed string inline (no helper)** — rejected because the compliance sweep already established the shared helper as the canonical shape; a fifth ad-hoc variant would just re-introduce drift.
- **Introduce a typed `OAuthClientError` enum with `IntoResponse`** — rejected as scope creep; the leak is fixed by four one-line changes plus a guard test, and a typed-error refactor deserves its own vector.

## Root-cause trace
1. Symptom: OAuth-client management endpoints echo raw Postgres error strings to the HTTP caller on constraint violations (schema disclosure).
2. ← Four handler bodies in `routes/oauth.rs` (register / update / revoke / regenerate) call `.to_string()` on the sqlx error and return it verbatim in the response.
3. ← Same idiom the compliance module carried before PR #2920 — the sanitizer helper existed only in `compliance.rs`, not workspace-wide.
4. Origin: OAuth-client module was written before the sanitizer pattern landed, and PR #2920 didn't sweep it (compliance-only scope).

## Test plan
- [ ] `backend/servers/api-server/tests/suites/oauth_client_registration_test.rs` — new `oauth_client_register_does_not_leak_raw_db_error` test firing a duplicate-name registration and asserting body `message` equals the sanitized string.
- [ ] Same file — source-level guard mirroring `compliance.rs:907`, asserting `oauth.rs` contains no raw-error idiom fragments.
- [ ] `cargo test -p api-server --test suites oauth_client_registration_test`

## Out of scope
- Sweeping other route modules (each is its own backlog row).
- Any change to OAuth-client business logic, endpoint contracts, or status codes.
- Lifting `db_error()` into a shared crate beyond `util::errors` — the minimal reuse suffices.

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-oauth-client-error-leak.md`
- Mark the matching `backlog.json` row as `status: "done"`
