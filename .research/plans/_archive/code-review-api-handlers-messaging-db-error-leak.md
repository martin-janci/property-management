# code-review-api-handlers-messaging-db-error-leak

**Vector:** security
**Score:** 2
**Source:** rotating-expert-review api-handlers 2026-11-09 (backlog id `code-review-api-handlers-messaging-db-error-leak`)
**Confidence:** high

## Hypothesis
`backend/servers/api-server/src/routes/messaging.rs` has 33 handler error branches that put `e.to_string()` into `Json(ErrorResponse::new("DB_ERROR", e.to_string()))` — the highest concentration in the api-server tree. Every failed thread/message/participant list/get/create/update/delete call therefore sends the raw sqlx `Display` (SQL fragments, table/column names, sometimes pool/TLS state) into the response body that reaches the manager client. Same class of leak and same remediation shape as the sibling `code-review-api-handlers-budgets-db-error-leak` plan — mechanical rewrite to a static message, keep the existing `tracing::error!` detail capture, add a grep-based CI guard. Qualifies for security fast-track promotion (vector=security + confidence=high + score≥2).

## Evidence
- `backend/servers/api-server/src/routes/messaging.rs:302` — first of the 33 sites: `Err(e) => { tracing::error!(...); (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new("DB_ERROR", e.to_string()))).into_response() }`. Raw sqlx `Display` reaches the body; detail is already captured by `tracing::error!` above it.
- 33 identical sites across the file. Confirmed by `grep -c 'DB_ERROR.*to_string' backend/servers/api-server/src/routes/messaging.rs` → `33`.
- `backend/crates/common/src/errors.rs:24-33` — `ErrorResponse::new(code, message)` is the standard constructor; passing a static `"Internal database error"` satisfies the existing contract with no schema change.
- Scope boundary: this plan covers only `messaging.rs`; the sibling plan `code-review-api-handlers-budgets-db-error-leak` covers `budgets.rs` (26 sites). The broader api-server tree has 124 total sites across many `routes/*.rs` files — remainder is a follow-up tracking issue filed after both sibling plans merge.

## Files
- `backend/servers/api-server/src/routes/messaging.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (enumerate all 33 leak sites, verify each rewrite compiles + preserves the existing `tracing::error!` signal)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Point api-server at a Postgres instance where a column expected by one of the messaging queries is missing (temporarily rename e.g. `messages.body` → `messages.body_tmp` on a dev DB).
2. As an authenticated user, call `GET /api/v1/organizations/{org}/messaging/threads/{thread}/messages`.
3. Expected: `500` with body `{"code":"DB_ERROR","message":"Internal database error",...}` (generic); full error captured only in `tracing::error!` server-side.
4. Actual (today): `500` with body `{"code":"DB_ERROR","message":"error returned from database: column \"body\" does not exist",...}` — raw sqlx `Display` reaches the client, exposing schema.

## Suggested approach
1. Run `rg -n 'ErrorResponse::new\("DB_ERROR", *e\.to_string\(\)\)' backend/servers/api-server/src/routes/messaging.rs` to enumerate the 33 sites.
2. For each site, replace the `e.to_string()` argument with the static message `"Internal database error"`. Keep the existing `tracing::error!` line unchanged — it is where the detail belongs.
3. Spot-check for sites where the error category is actually validation / not-found masquerading as `DB_ERROR` — leave those alone (change scope stays `DB_ERROR` arms only).
4. Author the regression test described in *Test plan* so a future developer re-adding `e.to_string()` fails CI.
5. Extend the grep-based CI guard introduced by the sibling budgets plan (or add it here if that plan has not merged yet): fail if `rg 'ErrorResponse::new\("DB_ERROR", *e\.to_string\(\)\)' backend/servers/api-server/src/routes/messaging.rs` returns any matches.
6. Local verify: `cargo test -p api-server --tests`, `cargo clippy -p api-server --all-targets -- -D warnings`, `cargo fmt --all`.

## Alternatives considered
- **Wait for the shared `api-core::errors::db_error` helper and migrate messaging onto it in a single workspace-wide PR** — rejected because the helper rollout is a separate refactor across 124 sites in many files; sitting on a known-reachable leak in the highest-concentration file to wait for a larger refactor is the wrong trade.
- **Localise the static message per handler (`"Failed to list messages"`, `"Failed to update message"`, …)** — rejected because the message is not meant to carry user-facing semantics (the `DB_ERROR` code does); varying the string per site expands the diff with no signal for the client.

## Root-cause trace
1. Symptom: authenticated `GET /api/v1/organizations/{org}/messaging/threads/...` on a broken DB returns a 500 whose body carries sqlx `Display` text mentioning DB column/table names — manager-side schema disclosure.
2. ← `backend/servers/api-server/src/routes/messaging.rs:302` (and 32 siblings) — `Json(ErrorResponse::new("DB_ERROR", e.to_string()))` interpolates the raw error into the body.
3. ← the pattern pre-dates any api-server `db_error` helper (none exists). Copy-pasted boilerplate turned a single-site mistake into 33 call sites — the heaviest concentration in the tree.
4. Origin: initial messaging routes shipped with this shape; subsequent handlers added to the file replicated it mechanically.

## Test plan
- [ ] `backend/servers/api-server/tests/suites/messaging_db_error_body_tests.rs` — new test `list_messages_returns_generic_body_on_db_error`: start api-server against a DB missing an expected messaging column (or mock the repo to return `sqlx::Error::Protocol("column \"body\" does not exist".into())`), authenticate, call the messaging list endpoint, assert body JSON `.message == "Internal database error"`, assert body does NOT contain `"column"` or `"error returned from database"`.
- [ ] Parallel regression test for one representative mutating site (e.g. `create_message_returns_generic_body_on_db_error`).
- [ ] Grep-based CI regression: `! rg -q 'ErrorResponse::new\("DB_ERROR", *e\.to_string\(\)\)' backend/servers/api-server/src/routes/messaging.rs` — exits non-zero on match; wire into `.github/workflows/backend.yml`.
- [ ] Command: `cargo test -p api-server --tests messaging_db_error_body` covers the new tests.

## Out of scope
- Rewriting the ~65 remaining `DB_ERROR` + `to_string` sites across other `routes/*.rs` files — follow-up tracking issue filed after this and the budgets sibling plan merge.
- Introducing a workspace-wide `api-core::errors::db_error` helper — kept for a separate refactor plan once both mechanical fixes land.
- Changing the client-visible error code (`DB_ERROR` stays as-is to preserve API contract).

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-messaging-db-error-leak.md`
- Mark the matching `backlog.json` row as `status: "done"`
