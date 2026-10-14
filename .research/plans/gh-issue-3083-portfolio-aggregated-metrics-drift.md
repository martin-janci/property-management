# gh-issue-3083-portfolio-aggregated-metrics-drift

**Vector:** bug
**Score:** 3
**Source:** Issue #3083
**Confidence:** high

## Hypothesis
PR #3081 reconciled the `property_performance_metrics` table via migration 00236 but left the sibling `portfolio_aggregated_metrics` table with the same class of schema drift: `PortfolioAnalyticsRepository::calculate_portfolio_metrics` / `get_portfolio_metrics` / `get_portfolio_summary` project columns no migration ever created, so the aggregated endpoints still 500/400 at runtime. Adding a companion migration that `ALTER TABLE ADD COLUMN`s each missing field (same add-don't-trim strategy 00236 used, to preserve the generated OpenAPI shape) resolves the drift; after that, the deleted success tests in `analytics_portfolio_success_tests.rs` should be restored.

## Evidence
- `backend/crates/db/src/repositories/portfolio_analytics.rs` — `calculate_portfolio_metrics` / `get_portfolio_metrics` / `get_portfolio_summary` project columns absent from the current `portfolio_aggregated_metrics` DDL
- `backend/crates/db/migrations/00236_reconcile_property_performance_metrics.sql` — pattern this plan mirrors for the aggregated table
- `backend/servers/api-server/tests/suites/analytics_portfolio_success_tests.rs` — portfolio-summary / benchmark success cases were deleted under "BIT-567"; need restoration after the migration lands
- Issue #3083 (owner-filed, follow-up to PR #3081 merged 2026-10-14)

## Files
- `backend/crates/db/src/repositories/portfolio_analytics.rs`
- `backend/crates/db/migrations/00236_reconcile_property_performance_metrics.sql`
- `backend/servers/api-server/tests/suites/analytics_portfolio_success_tests.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [x] C2 — Seed data
- [x] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

Mode: cloud-ok

## Repro steps
1. Boot the dev stack via `ppt_dev_up` or `stack up pm-local`.
2. Seed an org, portfolio, and a couple of properties.
3. `GET /api/portfolios/{portfolio_id}/metrics` and `GET /api/portfolios/{portfolio_id}/summary` as the portfolio's owner.
4. Expected: 200 with aggregated metrics. Actual (on `dev` after PR #3081): 500 / 400 — SQLx reports columns missing from `portfolio_aggregated_metrics` (same drift class 00236 fixed for `property_performance_metrics`).

## Suggested approach
1. Read `backend/crates/db/src/repositories/portfolio_analytics.rs` and enumerate every column name `calculate_portfolio_metrics`, `get_portfolio_metrics`, and `get_portfolio_summary` reference against `portfolio_aggregated_metrics` (and any benchmark table they touch).
2. Diff the enumerated set against the current DDL for `portfolio_aggregated_metrics` (search `backend/crates/db/migrations/*.sql` for the latest `CREATE TABLE portfolio_aggregated_metrics` + any later `ALTER TABLE`).
3. Author `backend/crates/db/migrations/00237_reconcile_portfolio_aggregated_metrics.sql` that `ALTER TABLE portfolio_aggregated_metrics ADD COLUMN <col> <type> [NOT NULL DEFAULT …]` for every missing column — mirror 00236's add-don't-trim choice so the generated-client API shape is unchanged. Columns with non-`Option` struct fields in the repo get `NOT NULL DEFAULT`.
4. Realign any `ON CONFLICT (...)` target in the repository code to a real unique constraint on `portfolio_aggregated_metrics` — PR #3081's "42P10" fix for the sibling table is the recipe; replicate it here.
5. Run `cargo sqlx prepare` from `backend/` and commit the updated `.sqlx/` offline data.
6. Restore the deleted `get_portfolio_summary` / benchmark success cases in `backend/servers/api-server/tests/suites/analytics_portfolio_success_tests.rs`, mirroring the `T1b` / `T2b` 200-control pattern PR #3081 added so the happy path is covered — not just the deny path.
7. Run `cargo test -p api-server --test suites analytics_portfolio_success_tests` and confirm the restored success cases pass; run `cargo fmt --all` + `cargo clippy -p api-server -p db -- -D warnings` before pushing.

## Alternatives considered
- **Trim the repository projections to only existing columns** — rejected because the issue explicitly notes the add-don't-trim strategy preserves the generated OpenAPI client shape (same reasoning 00236 followed for `property_performance_metrics`); trimming would ripple through the TS/Kotlin clients.
- **Drop and recreate the `portfolio_aggregated_metrics` table** — rejected because it loses historical rows (if any) and is a far larger blast radius than targeted `ADD COLUMN`s. Only ADDing is consistent with the migration-is-additive discipline the repo already uses.

## Root-cause trace
1. Symptom: `GET /api/portfolios/{id}/metrics` returns 500 on `dev` after PR #3081 — SQLx column-missing error on `portfolio_aggregated_metrics`.
2. ← At `backend/crates/db/src/repositories/portfolio_analytics.rs`, `calculate_portfolio_metrics` / `get_portfolio_metrics` / `get_portfolio_summary` project columns the current DDL does not declare.
3. ← The sibling `property_performance_metrics` drift (same class) was fixed by migration 00236 in PR #3081; the aggregated-metrics table was in that PR's "Out of scope" note and still unreconciled on `dev`.
4. Origin: an earlier refactor of `portfolio_analytics.rs` that added projection columns without a companion migration (same antecedent that produced the `property_performance_metrics` gap 00236 closed).

## Test plan
- [ ] `backend/servers/api-server/tests/suites/analytics_portfolio_success_tests.rs` — restore `get_portfolio_summary` / benchmark success cases; assert 200 with populated metrics for an owner request
- [ ] Confirm the restored cases fail on `dev` HEAD before migration 00237, pass after
- [ ] `cd backend && cargo test -p api-server --test suites analytics_portfolio_success_tests`
- [ ] `cd backend && cargo sqlx prepare` and commit `.sqlx/` deltas

## Out of scope
- Adding new metrics or endpoints — the plan reconciles existing projections only.
- Changing the generated OpenAPI client (TS / Kotlin) — ADD COLUMN preserves the current shape.
- Backfilling historical rows — the migration adds columns with `DEFAULT`s, no backfill needed.

## After-merge
- Move this file to `plans/_archive/gh-issue-3083-portfolio-aggregated-metrics-drift.md`
- Mark the matching `backlog.json` row as `status: "done"`
