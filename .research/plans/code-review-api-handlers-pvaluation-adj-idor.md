# code-review-api-handlers-pvaluation-adj-idor

**Vector:** security
**Score:** 3
**Source:** code-review 2026-10-14 (api-handlers Phase 1.5)
**Confidence:** high

## Hypothesis
The three property-valuation adjustment handlers — `list_adjustments`, `create_adjustment`, `delete_adjustment` in `backend/servers/api-server/src/routes/property_valuation.rs:498-545` — each discard the authenticated `AuthUser` (`_auth: AuthUser`) and operate on client-supplied `Path(comparable_id)` / `Path(adjustment_id)` with no organization check. The backing repo methods (`backend/crates/db/src/repositories/property_valuation.rs:582-638`) run on the raw non-RLS pool with no org filter — `delete_adjustment` is a bare primary-key `DELETE FROM comparable_adjustments WHERE id = $1`. Any authenticated user can `DELETE /api/v1/property-valuations/adjustments/{any_uuid}` to permanently destroy another tenant's adjustments, and `POST /api/v1/property-valuations/comparables/{other_org_uuid}/adjustments` to inject fraudulent amounts into another org's valuation workflow. Because adjustments directly influence the computed valuation amount, this is both cross-tenant data destruction *and* a valuation-tampering vector.

## Evidence
- `backend/servers/api-server/src/routes/property_valuation.rs:498-511` — `list_adjustments(_auth: AuthUser, Path(comparable_id): Path<Uuid>)` calls `state.property_valuation_repo.list_adjustments(comparable_id)` with no org check.
- `backend/servers/api-server/src/routes/property_valuation.rs:514-528` — `create_adjustment` same shape; mutates `data.comparable_id = comparable_id` from the client path then INSERTs.
- `backend/servers/api-server/src/routes/property_valuation.rs:531-545` — `delete_adjustment` same shape; bare primary-key delete via `state.property_valuation_repo.delete_adjustment(adjustment_id)`.
- `backend/crates/db/src/repositories/property_valuation.rs:582-614` — `create_adjustment` runs `.fetch_one(&self.pool)` on the raw pool.
- `backend/crates/db/src/repositories/property_valuation.rs:617-630` — `list_adjustments` runs `SELECT * FROM comparable_adjustments WHERE comparable_id = $1` on raw pool, no org filter.
- `backend/crates/db/src/repositories/property_valuation.rs:631-638` — `delete_adjustment` runs `DELETE FROM comparable_adjustments WHERE id = $1` on raw pool — unconstrained cross-tenant delete.
- `backend/servers/api-server/src/lib.rs:391` + `routes/property_valuation.rs:129-133` — router registers GET/POST `/comparables/{comparable_id}/adjustments` and DELETE `/adjustments/{adjustment_id}` under `/api/v1/property-valuations`.

## Files
- `backend/servers/api-server/src/routes/property_valuation.rs:498`
- `backend/servers/api-server/src/routes/property_valuation.rs:514`
- `backend/servers/api-server/src/routes/property_valuation.rs:531`
- `backend/crates/db/src/repositories/property_valuation.rs:582`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [x] C7 — Code-review reception (tick if you expect controversy)

Mode: cloud-ok

## Repro steps
1. Create two orgs (A, B); in org B, create a valuation with a comparable and an adjustment; note the adjustment's UUID.
2. Authenticate as a member of org A.
3. Issue `DELETE /api/v1/property-valuations/adjustments/<org_B_adjustment_uuid>` with the org-A token.
4. Expected: 403 FORBIDDEN or 404 NOT FOUND. Actual: 204 NO CONTENT; the adjustment is gone from org B's valuation. The valuation's computed amount silently changes.
5. Repeat with `POST /api/v1/property-valuations/comparables/<org_B_comparable_uuid>/adjustments` carrying a crafted body — the adjustment lands in org B's valuation workflow.

## Suggested approach
1. In `routes/property_valuation.rs:498-545`, rename each `_auth: AuthUser` to `auth: AuthUser`.
2. Add an org-check in the repo path: for `list_adjustments` and `create_adjustment`, join `comparable_adjustments` → `property_comparables` → `property_valuations` → `organization_id` and reject when it mismatches `auth.tenant_id`; for `delete_adjustment`, same traversal starting from the adjustment id. Mirror the ownership-check pattern used in `reserve_funds` / `vendors` repos (`let org_id = rls.tenant_id(); repo.op(&mut **rls.conn(), org_id, …)`), or add a dedicated `get_adjustment_org_id(adjustment_id)` repo helper that returns `Option<Uuid>` for cheap pre-flight checks.
3. In the DB layer, change the three statements to be org-aware: either (a) parameterise them with `organization_id` and add a WHERE clause joining through the comparable→valuation chain, or (b) migrate `property_valuation_repo` to the RLS pool with per-tenant row filters.
4. Add integration tests in `backend/servers/api-server/tests/suites/property_valuation_cross_tenant_idor_tests.rs` covering: cross-tenant delete rejected, cross-tenant list rejected, cross-tenant create rejected.
5. Validate: `cargo test -p api-server --test property_valuation_cross_tenant_idor_tests` + the IG3 fails-on-main.
6. No OpenAPI shape change (URL paths unchanged); no client regeneration needed unless the ownership error shape changes.

## Alternatives considered
- **Full RLS migration of `property_valuation_repo`.** — rejected because scope creep: the IDOR window must close now with a minimum-footprint change; RLS adoption for this repo should be a follow-up tracked under the broader `*_rls` migration plan.
- **Add a check-then-delete pattern via two round-trips (SELECT org then DELETE).** — rejected because of the TOCTOU gap and double round-trip cost; the single-statement DELETE/UPDATE with an org-join in the WHERE clause is both race-safe and faster.

## Root-cause trace
1. Symptom: `DELETE /api/v1/property-valuations/adjustments/<other_tenant_uuid>` returns 204 and the row is gone.
2. ← `delete_adjustment` at `routes/property_valuation.rs:531-545` discards `_auth` and calls `property_valuation_repo.delete_adjustment(adjustment_id)`.
3. ← `PropertyValuationRepository::delete_adjustment` at `crates/db/src/repositories/property_valuation.rs:631-638` runs `DELETE FROM comparable_adjustments WHERE id = $1` on the raw pool — no org filter.
4. Origin: the property-valuation module was authored without `AuthUser.tenant_id` wiring in the adjustment sub-router; sibling handlers in the same file use the same `_auth: AuthUser` pattern and may share the same class of bug (follow-up scan recommended but out of scope here).

## Test plan
- [ ] Integration test (DELETE): two orgs, cross-tenant DELETE returns 403/404 and the row survives. File `backend/servers/api-server/tests/suites/property_valuation_cross_tenant_idor_tests.rs`.
- [ ] Integration test (LIST): cross-tenant GET returns 403 or empty — never another tenant's rows.
- [ ] Integration test (CREATE): cross-tenant POST returns 403 — no injection into another tenant's workflow.
- [ ] IG3: all three tests must fail on `main`-snapshot before the fix lands.
- [ ] Exact command: `cargo test -p api-server --test property_valuation_cross_tenant_idor_tests`
- [ ] Fast backend check: `cargo clippy --workspace --all-targets -- -D warnings && cargo test -p api-server`

## Out of scope
- RLS migration of the broader `property_valuation_repo` beyond the three adjustment calls above.
- Audit-logging the cross-tenant attempts (nice-to-have; separate observability track).
- Scanning the other property-valuation handlers for the same bug class — follow-up ticket.

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-pvaluation-adj-idor.md`
- Mark the matching `backlog.json` row as `status: "done"`
