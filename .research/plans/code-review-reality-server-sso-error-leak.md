# code-review-reality-server-sso-error-leak

**Vector:** security
**Score:** 2
**Source:** signals/2026-08-18-reality-server-tier1d.json (dispatcher tier1d 2026-08-18, backlog id `code-review-reality-server-sso-error-leak`)
**Confidence:** high

## Hypothesis
reality-server's SSO handlers surface raw internal error `Display` strings to unauthenticated public clients via `SsoError::new(<code>, &e.to_string())` on ~20 sites in `routes/sso.rs`. On INTERNAL_SERVER_ERROR paths (token exchange failure, userinfo failure, user-create failure, session-create failure, invalid_token, pm_session_invalid), the response body leaks DB / crypto / JWT / reqwest internals — column names, constraint names, sometimes pool/TLS state — to the internet-facing OAuth flow. The smallest safe change mirrors the existing `util::errors::db_error("<op>", e)` pattern already used elsewhere in the crate (`code-review-reality-server-db-error-leak-to-client` addressed listings/realtors/layout/health): rewrite each offending `SsoError::new(<code>, &e.to_string())` to route the raw error through `tracing::error!` and return a scrubbed generic body, then add a `grep`-based guard so the pattern cannot regress.

## Evidence
- `backend/servers/reality-server/src/routes/sso.rs:265` — `Json(SsoError::new("token_exchange_failed", &e.to_string()))` on the public OAuth token-exchange path (unauthenticated).
- `backend/servers/reality-server/src/routes/sso.rs:275, 439, 923, 1114` — `user_info_failed` returns raw `e.to_string()` on four SSO variants.
- `backend/servers/reality-server/src/routes/sso.rs:287, 515, 958, 1131, 1158` — `user_create_failed` (five sites) leaks user-create Err verbatim.
- `backend/servers/reality-server/src/routes/sso.rs:299, 527, 971, 1142, 1169` — `session_create_failed` (five sites) leaks session-create Err verbatim.
- 20 total `SsoError::new(.*e\.to_string\(\))` matches confirmed by grep at :265, :275, :287, :299, :422, :439, :451, :515, :527, :687 (via nested map_err), :923, :958, :971, :1079, :1114, :1131, :1142, :1158, :1169 (this run, 2026-08-18).

## Files
- `backend/servers/reality-server/src/routes/sso.rs`
- `backend/servers/reality-server/src/util/errors.rs`

## Dependencies

## Required capabilities
- [ ] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Start reality-server locally (`cd backend && cargo run -p reality-server`).
2. Trigger any of the SSO handlers with a request that forces an internal error — e.g. POST `/api/v1/sso/callback` with a valid-shape but expired/invalid provider token so `token_exchange` upstream fails, or a DB-write path where the target user row already exists so `user_create_failed` fires.
3. Observe: response body carries the raw error string (e.g. `"token_exchange_failed: reqwest::Error { … request/network detail … }"`, or `"user_create_failed: sqlx::Error(Database(<constraint name>))"`).
4. Expected after fix: body carries only a generic `{"code":"token_exchange_failed","message":"Internal server error"}`; full error is written to `tracing::error!` server-side.

## Suggested approach
1. Add (or reuse) a scrubber helper in `backend/servers/reality-server/src/util/errors.rs` — mirror the existing `db_error(ctx, e) -> (StatusCode, Json<SsoError>)` shape used by the DB-leak plan; expose it as `sso_error(code, ctx, e)` so it always emits `tracing::error!(code, ctx, error = %e, "sso path failed")` and returns a generic response body.
2. Rewrite each of the ~20 `SsoError::new(<code>, &e.to_string())` sites in `backend/servers/reality-server/src/routes/sso.rs` to call `sso_error(<code>, <op-name>, e)` instead. Keep the HTTP status code identical (INTERNAL_SERVER_ERROR / UNAUTHORIZED as today).
3. Preserve the two intentional non-Err paths: `SsoError::new` with a static string as its second arg is fine and should stay as-is (e.g. any site where the second arg is a literal, not `e.to_string()`).
4. Add a grep guard test at `backend/servers/reality-server/tests/no_sso_error_leak.rs` (or extend an existing lint-shape test) — walk `routes/sso.rs`, fail if any line matches `SsoError::new\(.*e\.to_string\(\)`.
5. Update the plan of `code-review-reality-server-db-error-leak-to-client` cross-reference in-comment if needed to note the SSO surface is now covered.
6. `cargo fmt --all && cargo clippy -p reality-server --all-targets -- -D warnings && cargo test -p reality-server`.

## Alternatives considered
- **Wrap the `SsoError::new` constructor to always scrub** — rejected because it changes the semantics of legitimate uses (many sites pass a *static* client-facing message as arg 2, e.g. `"missing state cookie"`). A constructor-level scrub would either erase useful messages or require a new type; the helper-plus-grep-guard shape is a smaller diff and matches the pattern already used for DB errors.
- **Return `IntoResponse` for `sqlx::Error` etc via `From` impls that always redact** — rejected because reality-server error paths mix DB errors, `reqwest::Error`, decode errors, and internal invariants; a per-source-type `From` doesn't cover the `anyhow::Error`-shaped middle ground the sso handlers use, and the ~20 sites are already explicit `map_err` closures. A local rewrite is more predictable than global `From` refactoring.

## Root-cause trace
1. Symptom: an unauthenticated public POST to a reality-server SSO endpoint (`/api/v1/sso/**`) returns 500 with a body that contains DB constraint names, reqwest network detail, or JWT decoder internals.
2. ← Immediate cause: `routes/sso.rs:265` `Err((StatusCode::INTERNAL_SERVER_ERROR, Json(SsoError::new("token_exchange_failed", &e.to_string()))))` — the error's `Display` is stringified straight into the response body.
3. ← Upstream cause: no shared error scrubber for SSO paths — the crate has `util::errors::db_error` (documented specifically to *prevent* this kind of leak) but SSO handlers were written directly against `SsoError::new(<code>, <msg>)` without an equivalent redaction helper.
4. Origin: sso handlers landed with the raw-`e.to_string()` pattern in place (predates the 2026-08-01 `util::errors::db_error` plan); the DB-error-leak plan intentionally scoped listings/realtors/layout/health and left sso.rs alone.

## Test plan
- [ ] Add `backend/servers/reality-server/tests/sso_error_shape_tests.rs` — assert that an SSO path forced into a 500 (e.g. by seeding a duplicate-key insert) returns a body whose `message` field is *not* a raw `sqlx::Error` / `reqwest::Error` `Display` and *does* equal a stable generic string.
- [ ] Regression: grep-shape test in `backend/servers/reality-server/tests/no_sso_error_leak.rs` — fail on any `SsoError::new\(.*e\.to_string\(\)` match in `routes/sso.rs`.
- [ ] Local run: `cd backend && cargo test -p reality-server sso_error_shape_tests no_sso_error_leak`.

## Out of scope
- Wider rewrite of every error-return site in reality-server — the DB-leak plan already covered listings/realtors/layout/health; this plan is scoped to `routes/sso.rs` only.
- Changing HTTP status codes returned by SSO handlers (500 vs 401 vs 400) — same shapes today, same after.
- Client-side handling of the scrubbed error message — the SDK contract doesn't change.

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-sso-error-leak.md`
- Mark the matching `backlog.json` row as `status: "done"`
