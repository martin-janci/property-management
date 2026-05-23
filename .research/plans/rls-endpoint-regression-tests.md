# rls-endpoint-regression-tests

**Vector:** test-gap
**Score:** 4
**Source:** PR #421 | Issue #160 | Issue #375
**Confidence:** high

## Hypothesis
PR #421 migrated 13 endpoints in `market_pricing`, `notification_preferences`, and `reports` from non-RLS repository calls to their `*_rls` variants, and additionally added a brand-new cross-tenant IDOR guard in the four `reports` handlers (`organization_id != rls.tenant_id()` → 403). That PR shipped with **zero test files in its diff** (3 files changed, all production `.rs`). The smallest change that closes the gap is an endpoint-level integration test that asserts a tenant cannot read another tenant's report/pricing data through these handlers — mirroring the existing `workflow_cross_tenant_idor_tests.rs` pattern.

## Evidence
- PR #421 `fix(api-server): route market_pricing, notification_preferences & reports DB lookups through RLS` — diff is exactly 3 files (`market_pricing.rs`, `notification_preferences.rs`, `reports.rs`), no test file.
- `backend/servers/api-server/src/routes/reports.rs` — each of the 4 report handlers now early-returns `StatusCode::FORBIDDEN` when `!rls.is_super_admin() && query.organization_id != rls.tenant_id()`; this branch has no test exercising it.
- Issue #160 "Technical Debt: RLS Migration Backlog (48 endpoints)" lists "Add RLS tests for migrated endpoints" as required work, and names market_pricing (8), notification_preferences (4), reports (1) among the affected components.
- Issue #375 documents that the `security-tests` job (RLS penetration suite) was failing deterministically and only runs on title-matched PRs, so RLS regressions are not reliably gated on `main`.
- Existing precedent test `backend/servers/api-server/tests/workflow_cross_tenant_idor_tests.rs` proves the endpoint-level cross-tenant IDOR test shape is already supported in this crate.

## Files
- `backend/servers/api-server/src/routes/reports.rs`
- `backend/servers/api-server/src/routes/notification_preferences.rs`
- `backend/servers/api-server/src/routes/market_pricing.rs`
- `backend/servers/api-server/tests/workflow_cross_tenant_idor_tests.rs`
- `backend/crates/db/tests/rls_penetration_tests.rs`

## Required capabilities
- [ ] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [x] C2 — Seed data
- [x] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**
- If C4 or C5 is ticked → `local` (implementer must run on the user's Mac)
- Otherwise → `cloud-ok` (can run as a claude.ai routine via the `ppt-bridge` MCP)

Mode: cloud-ok

## Repro steps
1. Seed two organizations (org A, org B) each with a building and a unit, plus a manager user in org A.
2. As org A's manager, call `GET /api/v1/reports/fault-statistics?organization_id=<org-B-uuid>` (and the voting-participation / occupancy / consumption variants).
3. Expected: `403 FORBIDDEN`. Confirm there is currently no automated test asserting this; the guard added in PR #421 is unverified, so a regression that drops the `is_super_admin()` check would ship undetected.

## Suggested approach
1. Add a new integration test file `backend/servers/api-server/tests/reports_cross_tenant_idor_tests.rs`, modeled on `workflow_cross_tenant_idor_tests.rs`, using the shared harness under `backend/servers/api-server/tests/common`.
2. Cover all four report handlers (`get_fault_statistics_report`, `get_voting_participation_report`, `get_occupancy_report`, `get_consumption_report`): assert org A → org B's `organization_id` returns 403, and org A → its own `organization_id` returns 200.
3. Add a super-admin positive case asserting cross-org access is still permitted (the `is_super_admin()` branch).
4. Add notification-preferences coverage (`backend/servers/api-server/tests/notification_preferences_rls_tests.rs`) asserting `get_preferences` / `update_preference` only ever see the authenticated user's rows when run under an RLS connection.
5. For market_pricing, assert `request_recommendation` / `get_current_rent` return 404 (not another tenant's data) when given a `unit_id` belonging to a different org, exercising `find_by_id_rls`.
6. Run the suite locally/in-bridge and confirm the new tests pass against current `main`; then temporarily revert the `reports.rs` IDOR guard to confirm the cross-tenant test fails (proves it is a real regression test, not a tautology).
7. Restore the guard; leave the tests in place.

## Alternatives considered
- **Extend `rls_penetration_tests.rs` at the db layer instead** — rejected because the IDOR guard added in #421 lives in the HTTP handler (`organization_id` query param vs `tenant_id()`), not in a repository method, so a db-layer test cannot exercise the 403 branch.
- **Make `security-tests` a required check and run it on all backend PRs (per Issue #375)** — rejected as the primary fix because that is CI-policy work that does not itself add coverage for these three route modules; it is complementary and tracked separately on #375.

## Root-cause trace
N/A — test-gap doesn't need backward tracing. The endpoints are believed correct post-#421; this plan adds the regression coverage that PR #421 omitted, with particular focus on the untested cross-tenant IDOR guard in `reports.rs`.

## Test plan
- [ ] New `backend/servers/api-server/tests/reports_cross_tenant_idor_tests.rs` — cross-tenant 403 + same-tenant 200 + super-admin cross-org 200 for all four report handlers.
- [ ] Regression scenario: with the `reports.rs` org-id guard temporarily removed, the cross-tenant case returns 200 and the test fails; with the guard present it returns 403 and passes.
- [ ] `cargo test -p api-server --test reports_cross_tenant_idor_tests` (and the notification-preferences / market_pricing test files added alongside).

## Out of scope
- Migrating the remaining endpoints in Issue #160 (voting, buildings, organizations, documents, signatures, etc.) — this plan only covers the three modules touched by PR #421.
- Fixing or re-enabling the `security-tests` CI gating policy from Issue #375.
- Any production code change to the handlers beyond the temporary local revert used to validate the regression test.

## After-merge
- Move this file to `plans/_archive/rls-endpoint-regression-tests.md`
- Mark the matching `backlog.json` row as `status: "done"`
