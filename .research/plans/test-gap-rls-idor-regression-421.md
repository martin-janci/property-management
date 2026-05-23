# test-gap-rls-idor-regression-421

**Vector:** test-gap
**Score:** 4
**Source:** PR #421
**Confidence:** high

## Hypothesis
PR #421 closed two real cross-tenant IDOR surfaces — `market_pricing`'s `unit_repo.find_by_id` / `building_repo.find_by_id` (fetched any unit/building regardless of tenant) and `reports::get_building_name` (leaked arbitrary building names across tenants) — by routing the three mounted modules onto the `RlsConnection` extractor. It shipped with **zero** test files. There is no regression guard proving these endpoints now reject a tenant requesting another tenant's resource id, so a future refactor that drops the `_rls` variant (exactly the kind of churn these files keep seeing) would silently re-open the leak. The smallest fix is one integration test module that asserts cross-tenant 403/404 for the migrated handlers, mirroring the existing `workflow_cross_tenant_idor_tests.rs`.

## Evidence
- PR #421 body: "the deprecated `unit_repo.find_by_id` / `building_repo.find_by_id` fetched *any* unit/building by id regardless of tenant"; "`reports::get_building_name`: looked up an arbitrary building id unscoped — a building-name leak across tenants"
- `gh pr view 421 --json files` lists no file under `backend/servers/api-server/tests/`
- `grep -rlnE 'market_pricing|notification_preferences|get_building_name|record_price_change' backend/servers/api-server/tests/` → no matches (no existing coverage of these routes)
- `backend/servers/api-server/tests/workflow_cross_tenant_idor_tests.rs` — the established pattern for the same class of fix (#336/#340 `/workflows/*` IDOR closure)

## Files
- `backend/servers/api-server/src/routes/market_pricing.rs`
- `backend/servers/api-server/src/routes/reports.rs`
- `backend/servers/api-server/src/routes/notification_preferences.rs`
- `backend/servers/api-server/tests/workflow_cross_tenant_idor_tests.rs`

## Required capabilities
- [ ] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [x] C2 — Seed data
- [x] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok (no C4/C5 — backend Rust integration tests against a seeded Postgres; runnable via `ppt-bridge` MCP)

## Repro steps
1. Seed two tenants A and B, each with one building and one unit (B owns building `b_bldg`, unit `b_unit`).
2. Authenticate as a member of tenant A.
3. Call `POST /api/v1/market-pricing/recommendations` (or `GET` recommendation details) with B's `b_unit` / `b_bldg` id; call `GET` on a report whose `get_building_name` resolves `b_bldg`; call the notification-preferences read for B's user.
4. Expected (post-#421, RLS enforced): each returns 403/404 with no B data. Actual today: **untested** — no automated assertion exists, so a regression is invisible. (On the pre-#421 commit `fc8af0a~`, the market_pricing/reports calls would return B's data — confirming the test discriminates the fix.)

## Suggested approach
1. Add `backend/servers/api-server/tests/rls_route_idor_tests.rs` following the structure of `workflow_cross_tenant_idor_tests.rs` (reuse its `tests/common` harness for two-tenant setup + token minting).
2. Cover `market_pricing` first — request a recommendation / `get_current_rent` / `record_price_change` for tenant B's unit & building id while authenticated as tenant A; assert the cross-tenant id is not resolvable (403/404, never B's row).
3. Cover `reports::get_building_name` — drive one report handler that threads it (per #421's table, 4 report handlers) with B's building id as A; assert no B building name leaks into the response.
4. Cover `notification_preferences` `get_preferences` / `update_preference` — confirm A cannot read or mutate B's preferences via id.
5. Register the new test module if `tests/` uses an explicit module list; otherwise Cargo auto-discovers top-level `tests/*.rs`.
6. Run serially with the other RLS suites (`--test-threads=1`, per #369/#422) since they share a non-superuser RLS-enforcing DB role.

## Alternatives considered
- **Extend `workflow_cross_tenant_idor_tests.rs` in place** — rejected because that file is scoped to `/workflows/*`; mixing three unrelated route families into it muddies ownership and makes serial-vs-parallel test config harder to reason about. A sibling file keeps the pattern but isolates the surface.
- **Unit-test the repository `_rls` variants directly** — rejected because the IDOR surface is the *handler→connection wiring* (#421's actual change), not the repo method; a repo-level unit test would pass even if a handler regressed back to the non-RLS variant.

## Root-cause trace
N/A — test-gap doesn't need backward tracing. (The underlying defect was already root-caused and fixed in #421; this plan adds the missing regression guard, it does not re-fix the bug.)

## Test plan
- [ ] `backend/servers/api-server/tests/rls_route_idor_tests.rs` — new module: cross-tenant access to market_pricing / reports / notification_preferences returns 403/404, never the other tenant's data
- [ ] Regression discriminator: the market_pricing/reports cases fail when `routes/{market_pricing,reports}.rs` are reverted to the pre-#421 non-`_rls` repo calls (proves the test guards #421, not just the happy path)
- [ ] `cargo test -p api-server --test rls_route_idor_tests -- --test-threads=1` (serial, non-superuser RLS role per #422)

## Out of scope
- Refactoring/splitting the route modules (tracked separately as `refactor-notification-preferences-rs-hot` and the other churn-hotspot vectors).
- `documents.rs:1630` `restore_version` RLS migration and the intentionally-public `documents.rs:2480/2605` endpoints — different surface, noted in the 2026-05-23 brief.
- Any change to the `_rls` repository variants themselves (#421 added none; none needed).

## After-merge
- Move this file to `plans/_archive/test-gap-rls-idor-regression-421.md`
- Mark the matching `backlog.json` row as `status: "done"`
