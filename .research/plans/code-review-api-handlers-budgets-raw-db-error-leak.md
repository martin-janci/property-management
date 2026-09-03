# code-review-api-handlers-budgets-raw-db-error-leak

**Vector:** security
**Score:** 2
**Source:** commit db34026e (dispatcher Tier-1d api-handlers static review)
**Confidence:** high

## Hypothesis
Every database-error branch in `routes/budgets.rs` calls `Json(ErrorResponse::new("DB_ERROR", e.to_string()))` alongside `StatusCode::INTERNAL_SERVER_ERROR`. `sqlx::Error::Database` renders the raw Postgres message — table names, column names, constraint names, and SQL fragments — so any authenticated caller who trips a constraint violation can enumerate the schema. The fix is the same as PR #2925/#2920 applied to `compliance.rs`: route the 37 leak sites through the module-local `db_error()` sanitizer helper (or a shared equivalent).

## Evidence
- `backend/servers/api-server/src/routes/budgets.rs:260` (first of 37): `Json(ErrorResponse::new("DB_ERROR", e.to_string()))`
- `grep -c 'e\.to_string()' backend/servers/api-server/src/routes/budgets.rs` → 37 (verified 2026-09-03)
- `backend/servers/api-server/src/routes/compliance.rs:67` — the `fn db_error(e: impl std::fmt::Display) -> (StatusCode, String)` sanitizer pattern PR #2920 established
- `backend/servers/api-server/src/routes/compliance.rs:907` — `fn db_error_does_not_leak_raw_detail_to_client()` is the regression-guard test PR #2925 landed for the compliance module
- `tracing::error!("Failed to create budget: {:?}", e)` already logs the detail server-side, so the client-body copy is pure over-exposure

## Files
- `backend/servers/api-server/src/routes/budgets.rs`
- `backend/servers/api-server/src/routes/compliance.rs`
- `backend/servers/api-server/tests/suites/budgets_tests.rs`

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
1. Start api-server against a test tenant. Send `POST /api/v1/budgets` with a body that violates a NOT NULL / FK / uniqueness constraint (e.g. a duplicate `code` for the same building, or a missing required column value).
2. Expected: HTTP 500 with an opaque body such as `{"code":"DB_ERROR","message":"internal database error"}` and the actual Postgres message only in server logs. Actual today: HTTP 500 with `{"code":"DB_ERROR","message":"error returned from database: duplicate key value violates unique constraint \"budgets_org_code_key\" ..."}` — the constraint name, and the columns it covers, are echoed to the caller.

## Suggested approach
1. Add a module-scoped `fn db_error(e: impl std::fmt::Display) -> (StatusCode, Json<ErrorResponse>)` helper at the top of `budgets.rs` (or lift `compliance.rs:67` into a shared `util::errors::db_error` and reuse it). It must return `INTERNAL_SERVER_ERROR` with a fixed, sanitized body (`ErrorResponse::new("DB_ERROR", "internal database error")`) and MUST `tracing::error!` the raw `Display` server-side.
2. Replace every `Json(ErrorResponse::new("DB_ERROR", e.to_string()))` site — start at `budgets.rs:260` and sweep all 37 hits — with `db_error(e)`. Do NOT change status codes or map paths.
3. Delete the paired `tracing::error!` calls that duplicate the helper's log (or keep them — pick one, don't emit both).
4. Add a source-level guard test in `tests/suites/budgets_tests.rs` mirroring `compliance.rs:907`: `assert!(!source.contains("map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))"))` against `budgets.rs`. This catches the next regression the way PR #2925 did for compliance.
5. Add one end-to-end test that trips a real constraint violation via the repo and asserts the response body's `message` field equals the sanitized string exactly (not `.contains("internal")`, which would still pass on a leak).
6. Run `cargo test -p api-server budgets_tests` and `cargo test -p api-server compliance_tests` (guard the neighbour didn't regress) locally.

## Alternatives considered
- **In-place `.map_err(|_| INTERNAL_SERVER_ERROR)` without a helper** — rejected because it silently drops the log context the current `tracing::error!` calls provide, degrading observability while fixing the leak.
- **Rewrite the whole module to use a typed `BudgetError` enum with `IntoResponse`** — rejected: too large a diff for a leak fix, and the same result is achieved with the tiny helper PR #2925 already validated. Save the typed-error refactor for a separate `refactor` vector.

## Root-cause trace
1. Symptom: `POST /api/v1/budgets` with a constraint-violating payload returns HTTP 500 whose body echoes the raw Postgres error string (schema disclosure).
2. ← Handler at `budgets.rs:260` (create_budget) maps sqlx errors via `Json(ErrorResponse::new("DB_ERROR", e.to_string()))` — the same pattern PR #2920 removed from compliance.rs.
3. ← All 36 other budget handlers copy the same idiom (list/get/update/delete/submit/approve/activate/close/summary/category_variance/variance_alerts/add_item/list_items/update_item/delete_item/record_actual/list_actuals/create_category/list_categories …).
4. Origin: the raw-error idiom predates the `db_error()` helper PR #2920 introduced in `compliance.rs:67`; when the sweep happened it only covered compliance, and no source-level guard prevented the same shape reappearing elsewhere.

## Test plan
- [ ] `backend/servers/api-server/tests/suites/budgets_tests.rs` — new `budgets_do_not_leak_raw_db_error_to_client` end-to-end test that fires a constraint-violating create and asserts the response body's `message` equals the sanitized copy exactly.
- [ ] `backend/servers/api-server/tests/suites/budgets_tests.rs` — source-level guard mirroring `compliance.rs:907`: read `budgets.rs`, assert `!source.contains("(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())")` and `!source.contains("Json(ErrorResponse::new(\"DB_ERROR\", e.to_string()))")`.
- [ ] `cargo test -p api-server --test suites budgets` (or the module-scoped runner the workspace uses).

## Out of scope
- Rewriting other route modules (oauth.rs is its own backlog row `code-review-api-handlers-oauth-client-error-leak`).
- Introducing a typed `BudgetError` enum or changing status-code mappings.
- Deleting the redundant `tracing::error!` calls beyond the ones the helper subsumes.

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-budgets-raw-db-error-leak.md`
- Mark the matching `backlog.json` row as `status: "done"`
