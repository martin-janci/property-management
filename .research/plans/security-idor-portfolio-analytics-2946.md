# security-idor-portfolio-analytics-2946

**Vector:** security
**Score:** 3
**Source:** Issue #2946
**Confidence:** high

## Hypothesis
`portfolio_analytics` property-metrics handlers (`upsert_property_metrics`, `get_property_metrics`) compute the caller's `org_id` from `AuthUser.tenant_id` but bind it to `_org_id` (underscore = intentionally unused) and never pass it to the repository call — so writes and reads span all tenants. A caller in org A can overwrite org B's property metrics and read them back verbatim. The smallest fix is to pass `org_id` through to the repository and add a `WHERE organization_id = $N` clause in `portfolio_analytics::upsert_property_metrics` and `::get_property_metrics` in the DB crate.

## Evidence
- Issue #2946 (bug+security, OPEN 2026-09-06): "portfolio_analytics property metrics computed org_id then discarded"
- `backend/servers/api-server/src/routes/portfolio_analytics.rs:281` — `let _org_id = user.tenant_id.ok_or_else(...)?;` then `s.portfolio_analytics_repo.upsert_property_metrics(req).await` (no `org_id` argument)
- `backend/servers/api-server/src/routes/portfolio_analytics.rs:314` — same pattern for `get_property_metrics(building_id, period_start, period_end)`
- Same file already threads `org_id` correctly for benchmarks (`:98,124,147,170,198,227`) and portfolio metrics (`:266,362,403`) — so the pattern is established; these two handlers are the outliers.

## Files
- `backend/servers/api-server/src/routes/portfolio_analytics.rs`
- `backend/crates/db/src/repositories/portfolio_analytics.rs`

## Required capabilities
- [x] C1 — Systematic debugging (bug/security fix; walk the call chain)
- [x] C2 — Seed data (two tenants + property metrics rows in each)
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Seed two orgs A (id=`aaaa…`) and B (id=`bbbb…`), each with a building and a `property_metrics` row.
2. Auth as an org-A user; call `POST /api/v1/portfolio-analytics/property-metrics` with the request body targeting org-B's building_id (or a metric row row-id that belongs to org B).
3. Auth as the org-A user again; call `GET /api/v1/portfolio-analytics/property-metrics/{building_id}` for org-B's building_id and observe org-B's metrics returned.
4. Expected: 403/404 on both. Actual: write succeeds and the read returns cross-tenant data.

## Suggested approach
1. In `portfolio_analytics.rs:281`, change `let _org_id = …` to `let org_id = …` and update the repo call to `s.portfolio_analytics_repo.upsert_property_metrics(org_id, req).await`.
2. In `portfolio_analytics.rs:314`, same rename; call becomes `s.portfolio_analytics_repo.get_property_metrics(org_id, building_id, query.period_start, query.period_end).await`.
3. In `backend/crates/db/src/repositories/portfolio_analytics.rs`, update `upsert_property_metrics` to accept `org_id: Uuid` and either (a) add `AND organization_id = $` on the update clause / `INSERT … organization_id = $` on the insert, or (b) look up the target `building_id`'s org and reject the write when it doesn't equal `org_id`. Prefer (a) so the ORM/upsert stays a single statement.
4. Update `get_property_metrics` to filter by `organization_id = $` as well.
5. Rerun `cargo sqlx prepare` (workspace-wide) if the query shapes changed.
6. Add integration tests under `backend/servers/api-server/tests/` — one asserting cross-tenant write is rejected, one asserting cross-tenant read returns 404, one asserting same-tenant round-trip still works.
7. Grep the same file for any other `_org_id` bindings introduced by copy-paste; there are none as of 2026-09-08 but keep the grep in the PR checklist.

## Alternatives considered
- **RLS-only fix (defer to Postgres row-level security)** — rejected because RLS on the `property_metrics` table would work but the api-server does not run its pool with tenant-scoped session variables today, so a scoped repository parameter is the shorter, verifiable path and matches every other handler in the same file.
- **Add a `verify_building_belongs_to_org` middleware wrapping only these two handlers** — rejected because it duplicates the RLS pattern at the app layer without covering the write path's target row, and adds a second round-trip per request.

## Root-cause trace
1. Symptom: cross-tenant read/write on `portfolio_analytics` property-metrics endpoints.
2. ← `_org_id` binding at `backend/servers/api-server/src/routes/portfolio_analytics.rs:281,314` — auth succeeds, tenant is derived, then the value is thrown away.
3. ← Repository signatures `upsert_property_metrics(req)` and `get_property_metrics(building_id, period_start, period_end)` in `backend/crates/db/src/repositories/portfolio_analytics.rs` — no `org_id` parameter, so the SQL has no tenant predicate.
4. Origin: original `portfolio_analytics` scaffold — the benchmarks handlers in the same file threaded `org_id`; property-metrics handlers were added later and missed the pattern.

## Test plan
- [ ] `backend/servers/api-server/tests/portfolio_analytics_idor.rs` — new file. Two-tenant round-trip: setup org A + org B; org-A caller cannot upsert org-B's `property_metrics`; org-A caller cannot read org-B's `property_metrics`; org-A caller can round-trip its own row.
- [ ] Regression: same-tenant upsert + get still works end-to-end (existing coverage will already assert this once the signature change lands).
- [ ] `cd backend && cargo test -p api-server portfolio_analytics_idor` and `cargo test -p db portfolio_analytics`.

## Out of scope
- The other `portfolio_analytics` handlers already thread `org_id` — no reason to touch them.
- Issue #2945 (portfolio_properties) is a separate row/plan.
- Renaming the repo methods for clarity — keep the signature diff minimal.

## After-merge
- Move this file to `plans/_archive/security-idor-portfolio-analytics-2946.md`
- Mark the matching `backlog.json` row as `status: "done"`
