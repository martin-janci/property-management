# security-listing-analytics-portal-rls-2199

**Vector:** security
**Score:** 3
**Source:** Issue #2199 | PR #2183 (introduced the endpoint) | migration 00113 (installed the policy) | migration 00186 (introduced portal ownership without extending the policy)
**Confidence:** high

## Hypothesis
The per-listing analytics endpoint `GET /api/v1/my/listings/{id}/analytics` returns an empty summary and empty daily series for **every portal-owned listing**, even when analytics rows exist. The RLS policy `listing_analytics_tenant_isolation` (migration 00113) grants rows only via `is_super_admin()` OR an org-branch that checks `listings.organization_id = get_current_org_id()`. Portal listings have `organization_id = NULL` (migration 00186 moved ownership to `portal_owner_id`) and there is no portal-owner branch on the analytics policy. The read path in `RealityPortalRepository::get_listing_analytics` also runs on the RLS-subject pool with no `app.current_user_id` set. Fix by adding a `portal_get_listing_analytics` SECURITY DEFINER function (mirroring `portal_get_listing` in migration 00186) ownership-gated on `portal_owner_id = p_user_id OR created_by = p_user_id`, and route the analytics read through it.

## Evidence
- Issue #2199 body — cites `backend/crates/db/src/repositories/reality_portal/listings.rs:215-235` running raw `SELECT * FROM listing_analytics` on RLS-subject pool with no context set
- `backend/crates/db/migrations/00113_rls_reality_portal_professional.sql:67-75` — `FORCE ROW LEVEL SECURITY` on `listing_analytics` with only the org-branch policy
- `backend/crates/db/migrations/00186_portal_listing_ownership.sql` — added portal-owner context to `listings` but not to `listing_analytics`
- `backend/servers/reality-server/tests/portal_listings_my_list_analytics_tests.rs::analytics_owner_returns_200` — passes as superuser (bypasses RLS) and asserts `totalViews == 0`, indistinguishable from RLS filtering everything
- PR #2183 body — LIST half correctly sets `app.current_user_id` GUC; analytics half is the sibling that regressed

## Files
- `backend/crates/db/src/repositories/reality_portal/listings.rs`
- `backend/servers/reality-server/src/routes/portal_listings.rs`
- `backend/servers/reality-server/tests/portal_listings_my_list_analytics_tests.rs`
- `backend/crates/db/migrations/00113_rls_reality_portal_professional.sql`
- `backend/crates/db/migrations/00186_portal_listing_ownership.sql`

## Required capabilities
- [ ] C1 — Systematic debugging
- [x] C2 — Seed data (need `listing_analytics` rows + a portal-owned listing to exercise the RLS-subject read path)
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception (RLS + migration change is controversy-prone)

Mode: cloud-ok

## Repro steps
1. Seed a portal-owned listing (organization_id NULL, portal_owner_id = some user U) and call `track_listing_view(listing_id, 'website')` (or `INSERT INTO listing_analytics ...` as superuser to bypass its own policy).
2. As user U (JWT bearer), `GET /api/v1/my/listings/{listing_id}/analytics?from=2026-01-01&to=2026-12-31`.
3. Expected: 200 with `totalViews >= 1` and a daily-series row for the seeded date.
4. Actual (on `dev` today): 200 with `totalViews == 0` and empty daily series — RLS filtered the row.

## Suggested approach
1. Add migration `backend/crates/db/migrations/00XXX_portal_listing_analytics_security_definer.sql` that creates `portal_get_listing_analytics(p_listing_id UUID, p_user_id UUID, p_from DATE, p_to DATE)` SECURITY DEFINER, ownership-gated on `EXISTS (SELECT 1 FROM listings l WHERE l.id = p_listing_id AND (l.portal_owner_id = p_user_id OR l.created_by = p_user_id))`, returning the `listing_analytics` rows filtered by the date range. Mirror the existing `portal_get_listing` shape.
2. Update `backend/crates/db/src/repositories/reality_portal/listings.rs::get_listing_analytics` (lines 215-235) to call the new function via `sqlx::query_as!` against the RLS-subject pool with the caller's `user_id` — no `set_config` needed because the SECURITY DEFINER function is the trust boundary.
3. Optionally extend `listing_analytics_tenant_isolation` policy with an `OR EXISTS (... p.portal_owner_id = get_current_portal_user_id() ...)` branch to keep defense-in-depth on writes (analytics INSERT via `track_listing_view` is subject to the same policy's WITH CHECK).
4. Update `backend/servers/reality-server/tests/portal_listings_my_list_analytics_tests.rs::analytics_owner_returns_200` — seed at least one `listing_analytics` row for the portal-owned listing (`INSERT INTO listing_analytics (listing_id, viewed_at, source) VALUES ($1, NOW(), 'website')` in the test's arrange block) so the test can fail on the RLS bug and pass on the fix.
5. Add a new test `analytics_seeded_returns_non_zero_totals` that seeds N views and asserts `totalViews == N` — pins the actual read path, not just the 200 status.
6. Add `analytics_cross_portal_owner_returns_404_or_empty` — a portal user querying another portal user's listing must not leak analytics.
7. Regenerate sqlx offline data (`cargo sqlx prepare --workspace -- --all-targets --tests`) if new query macros land.

## Alternatives considered
- **Extend `listing_analytics_tenant_isolation` policy with a portal-owner branch and set `app.current_user_id` GUC in the read path** — rejected because the reality-server DB role is RLS-subject by design as defense-in-depth (per PR #2183's own commentary) and the whole `reality_portal/*` module already uses SECURITY DEFINER functions to cross the trust boundary; adding a policy branch and a GUC would introduce a second, weaker mechanism for the same trust hop. SECURITY DEFINER keeps the pattern uniform.
- **Move the read path to the api-server pool (which bypasses RLS via `app.rls_bypass = true`)** — rejected because reality-server owns the `/my/listings/*` surface and cross-server DB coupling for one endpoint would leak reality-server's public-listings trust model. Also breaks the LIST/analytics symmetry PR #2183 established.

## Root-cause trace
1. Symptom: `GET /api/v1/my/listings/{id}/analytics` returns `{ totalViews: 0, series: [] }` for a portal-owned listing that has recorded views.
2. ← `RealityPortalRepository::get_listing_analytics` at `backend/crates/db/src/repositories/reality_portal/listings.rs:215-235` runs `SELECT * FROM listing_analytics WHERE listing_id = $1 AND viewed_at BETWEEN ...` on the RLS-subject pool.
3. ← RLS policy `listing_analytics_tenant_isolation` at `backend/crates/db/migrations/00113_rls_reality_portal_professional.sql:70-75` filters every row because `p.organization_id = get_current_org_id()` is `NULL = NULL` (never true) and `is_super_admin()` is false.
4. ← Migration `backend/crates/db/migrations/00186_portal_listing_ownership.sql` introduced portal-owner ownership on `listings` (nullable `organization_id`, populated `portal_owner_id`) but did not add a portal-owner branch to the `listing_analytics` policy nor a `portal_get_listing_analytics` SECURITY DEFINER function.
5. Origin: PR that landed migration 00186 — the analytics policy was left behind when portal ownership shipped; PR #2183 exposed the surface via `GET /api/v1/my/listings/{id}/analytics` without noticing the pre-existing RLS gap.

## Test plan
- [ ] Extend `backend/servers/reality-server/tests/portal_listings_my_list_analytics_tests.rs::analytics_owner_returns_200` to actually seed `listing_analytics` rows for the portal listing (currently seeds none, so RLS filtering is invisible)
- [ ] New test `analytics_seeded_returns_non_zero_totals` — seeds 3 views on distinct days, asserts the owner sees `totalViews == 3` and 3 daily entries; this test must **fail** on `dev` (bug reproduces) and **pass** after the fix
- [ ] New test `analytics_cross_portal_owner_returns_empty_or_404` — user B (also a portal user) queries user A's listing analytics; must not leak
- [ ] Where feasible, run the assertions under a non-superuser DB role so RLS is actually in force (superuser bypasses everything and would keep hiding this class of bug)
- [ ] `cd backend && cargo test -p reality-server --test portal_listings_my_list_analytics_tests -- --nocapture`
- [ ] `cd backend && SQLX_OFFLINE=false cargo sqlx prepare --workspace -- --all-targets --tests` if new query macros are added

## Out of scope
- Repointing the web `useMyListings` hook from `/api/v1/realtors/me/listings` to `/api/v1/my/listings` (already flagged as a follow-up in PR #2183 body; a separate `frontend` task).
- Adding TypeSpec entries under `docs/api/typespec/` for the new endpoints (PR #2183 shipped with utoipa-only docs; TypeSpec sync is a broader task tracked elsewhere).
- Making `track_listing_view` (the writer) work for portal listings — same policy gap on WITH CHECK, but writes go through a `SECURITY INVOKER` function; that fix is a sibling task best rolled in the same migration, but the acceptance criterion here is *read-path* correctness.
- Broader audit of every reality_portal `SELECT *` that touches an RLS-forced table without setting context — worth doing but out of scope for this fix.

## After-merge
- Move this file to `plans/_archive/security-listing-analytics-portal-rls-2199.md`
- Mark the matching `backlog.json` row `security-listing-analytics-portal-rls-2199` as `status: "done"` with the resolving PR number appended to `sources`
- Close GitHub issue #2199 with a link to the merging PR
