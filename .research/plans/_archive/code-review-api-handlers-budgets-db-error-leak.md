# code-review-api-handlers-budgets-db-error-leak

**Vector:** security
**Score:** 3
**Source:** rotating-expert-review api-handlers 2026-11-09 (backlog id `code-review-api-handlers-budgets-db-error-leak`)
**Confidence:** high

## Hypothesis
`backend/servers/api-server/src/routes/budgets.rs` has 26 handler error branches that put `e.to_string()` into `Json(ErrorResponse::new("DB_ERROR", e.to_string()))` on `Err(e)` arms where `e` is a `sqlx::Error`. Every failed budget list/get/create/update/delete/rollover/approval call therefore sends the raw sqlx `Display` — SQL fragments, table/column names, sometimes pool-state or TLS-error host info — into the response body that reaches the Property Management manager client. The smallest safe change is a mechanical rewrite: every one of the 26 call sites becomes `ErrorResponse::new("DB_ERROR", "Internal database error")` with the raw error captured via the already-present `tracing::error!` line; a grep-based CI guard keeps the pattern from regressing. Same remediation shape as the existing `code-review-reality-server-db-error-leak-to-client` plan but on the manager API (behind auth) instead of the public portal.

## Evidence
- `backend/servers/api-server/src/routes/budgets.rs:468` — `Err(e) => { tracing::error!("Failed to list budgets: {:?}", e); rls.release().await; (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new("DB_ERROR", e.to_string()))).into_response() }` — the tracing line already captures `e`, so the body-side `e.to_string()` is pure leak with no ops value.
- 26 identical sites in this file at lines: 468, 505, 542, 583, 624, 694, 735, 776, 1105, 1142, 1179, 1236, 1270, 1307, 1339, 1367, 1404, 1441, 1481, 1521, 1561, 1598, 1630, 1667, 1704, 1741. Confirmed by `grep -c 'DB_ERROR.*to_string' backend/servers/api-server/src/routes/budgets.rs` → `26`.
- `backend/crates/common/src/errors.rs:24-33` — `ErrorResponse::new(code, message)` is the standard constructor; passing a static `"Internal database error"` as the message satisfies the existing contract (no schema change). No api-server-side `db_error` helper exists yet (only reality-server has one); this plan only migrates budgets.rs — a shared helper is a follow-up.
- Scope boundary: the broader api-server tree has 124 total `DB_ERROR` + `to_string` sites across many `routes/*.rs` files; this plan is scoped to `budgets.rs` (highest-value handler module after `messaging.rs`). Sibling plan `code-review-api-handlers-messaging-db-error-leak` covers the other top-concentration file.

## Files
- `backend/servers/api-server/src/routes/budgets.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (enumerate all 26 leak sites, verify each rewrite compiles + preserves the existing `tracing::error!` signal)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Point api-server at a Postgres instance where a column expected by one of the budgets queries is missing (easiest: temporarily rename a column in a dev DB, e.g. `budgets.name` → `budgets.name_tmp`).
2. As an authenticated manager, call `GET /api/v1/organizations/{org}/budgets`.
3. Expected: `500` with body `{"code":"DB_ERROR","message":"Internal database error",...}` (generic); full error captured only in `tracing::error!` server-side.
4. Actual (today): `500` with body `{"code":"DB_ERROR","message":"error returned from database: column \"name\" does not exist", ...}` — raw sqlx `Display` reaches the manager client, exposing schema shape.

## Suggested approach
1. Run `rg -n 'ErrorResponse::new\("DB_ERROR", *e\.to_string\(\)\)' backend/servers/api-server/src/routes/budgets.rs` to enumerate the 26 sites.
2. For each site, replace the `e.to_string()` argument with the static message `"Internal database error"` (keep the existing `tracing::error!` line above it — it is where the detail belongs). Keep the `rls.release().await` and `.into_response()` lines unchanged.
3. Spot-check that no site has a materially different message intent (e.g. a `BAD_REQUEST`/`VALIDATION_ERROR` arm hiding among the `DB_ERROR` ones — if so, leave it alone).
4. Author the regression test described in *Test plan* so a future developer re-adding `e.to_string()` fails CI.
5. Add a grep-based lint step to `.github/workflows/backend.yml` (or a new `just check-no-raw-db-leaks` recipe wired into the workflow): fail if `rg 'ErrorResponse::new\("DB_ERROR", *e\.to_string\(\)\)' backend/servers/api-server/src/routes/budgets.rs` returns any matches. Ship the CI guard in the same PR as the code change.
6. Local verify: `cargo test -p api-server --tests`, `cargo clippy -p api-server --all-targets -- -D warnings`, `cargo fmt --all`.

## Alternatives considered
- **Introduce a shared `api-core::errors::db_error` helper and migrate budgets.rs onto it in this PR** — rejected because the helper is a workspace-wide refactor (124 sites across the api-server tree); conflating the mechanical fix with the helper rollout multiplies diff and review surface. File a follow-up plan for the helper + broader migration.
- **Localise the static message to each handler ("Failed to list budgets", "Failed to update budget", …)** — rejected because the message is not meant to carry user-facing semantics (the `DB_ERROR` code already does that); varying the string per site gives no signal to the client and expands the diff.

## Root-cause trace
1. Symptom: authenticated `GET /api/v1/organizations/{org}/budgets` on a broken DB returns a 500 whose body carries sqlx `Display` text mentioning DB column/table names — manager-side schema disclosure.
2. ← `backend/servers/api-server/src/routes/budgets.rs:468` (and 25 siblings) — `Json(ErrorResponse::new("DB_ERROR", e.to_string()))` interpolates the raw error into the body.
3. ← the pattern pre-dates any api-server `db_error` helper (none exists; only reality-server has one). Copy-pasted boilerplate across the handler file turned a single-site mistake into 26 call sites.
4. Origin: initial budgets routes shipped with this shape (no single-commit introduction; the pattern replicates on every new handler added to the file).

## Test plan
- [ ] `backend/servers/api-server/tests/suites/budgets_db_error_body_tests.rs` — new test `list_budgets_returns_generic_body_on_db_error`: start api-server against a DB where the `budgets` table is missing / column dropped (or mock the repo to return `sqlx::Error::Protocol("column \"secret\" does not exist".into())`), authenticate as a manager, call `GET /api/v1/organizations/{org}/budgets`, assert body JSON `.message == "Internal database error"`, assert body does NOT contain the substring `"column"` or `"error returned from database"`.
- [ ] Parallel regression tests for one representative mutating site (e.g. `create_budget_returns_generic_body_on_db_error` covering line 505).
- [ ] Grep-based CI regression: `! rg -q 'ErrorResponse::new\("DB_ERROR", *e\.to_string\(\)\)' backend/servers/api-server/src/routes/budgets.rs` — exits non-zero on match; wire it into `.github/workflows/backend.yml`.
- [ ] Command: `cargo test -p api-server --tests budgets_db_error_body` covers the new tests.

## Out of scope
- Rewriting the other 98 `DB_ERROR` + `to_string` sites across the api-server tree — sibling plan covers `messaging.rs`, further coverage is a follow-up (file a tracking issue once both this and the messaging plan merge).
- Introducing a workspace-wide `api-core::errors::db_error` helper — kept for a separate refactor plan once the mechanical fix lands here and in `messaging.rs`.
- Changing the client-visible error code (`DB_ERROR` stays as-is to preserve API contract).

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-budgets-db-error-leak.md`
- Mark the matching `backlog.json` row as `status: "done"`
