# code-review-api-handlers-platform-admin-jwt-only-gate

**Vector:** security
**Score:** 3
**Source:** Phase 1.5 code review 2026-06-24 (`api-handlers` segment) · PR #1741 (same pattern fixed in routes/admin/*)
**Confidence:** high

## Hypothesis
`backend/servers/api-server/src/routes/subscriptions.rs` and `backend/servers/api-server/src/routes/feature_packages.rs` gate every platform-admin write endpoint (create / update / delete plans, coupons, payment methods, feature packages) with a local `require_super_admin` helper that only inspects `claims.roles` from the JWT. A super-admin whose role is revoked in the DB keeps full subscription / billing / feature-package mutation power until the JWT TTL expires — the same stale-JWT pattern PR #1741 (Airbnb reservations) and PR #1746 (portal-listings ownership) hardened in `routes/admin/*` using DB-backed `RequireCapability` (admin-core) / `require_platform_admin` (api-core middleware). The fix is mechanical: replace the local helper with the canonical DB-backed middleware on the router.

## Evidence
- `backend/servers/api-server/src/routes/subscriptions.rs:69-132` — local `require_super_admin` calls only `has_super_admin_role(&claims.roles)`; no DB capability/role lookup. 8 platform-admin write call-sites: lines 353, 485, 518, 787, 1135, etc. Mounted at `lib.rs:276` (`/api/v1/subscriptions`) and `lib.rs:279` (admin sub-router) with no router-level `require_capability` layer.
- `backend/servers/api-server/src/routes/feature_packages.rs:49-112` — identical JWT-only `require_super_admin` helper; 6 platform-admin write call-sites: 250, 285, 321, 365, 401, etc. Mounted at `lib.rs:347` with no `require_capability` layer.
- `backend/crates/admin-core/src/extractor.rs:72` — canonical `RequireCapability(Capability)` extractor used by `routes/admin/*` (e.g. `routes/admin/users_lifecycle.rs:33-49` uses `require_capability(Capability::UsersRead)`).
- `backend/crates/api-core/src/middleware/authorization.rs:228` — canonical `require_platform_admin` middleware reads `TenantRole` from request extensions (set by `ValidatedTenantExtractor`, DB-backed), not JWT. Already used by `routes/infrastructure.rs:39` and `routes/marketplace.rs:212` post-#1741/#1746 hardening wave.
- Comparison context: PRs #1741 (require manager role to list Airbnb reservations — guest PII) and #1746 (portal-listings ownership IDOR tests + FE wiring) closed the same JWT-role-claim gap in `routes/admin/*`. Issue #1787 follow-up filed for `booking_channel` — same pattern.

## Files
- `backend/servers/api-server/src/routes/subscriptions.rs:69`
- `backend/servers/api-server/src/routes/feature_packages.rs:49`
- `backend/servers/api-server/src/lib.rs:276`
- `backend/crates/api-core/src/middleware/authorization.rs:228`

## Dependencies

(none — independent of other queued work)

## Required capabilities
- [x] C1 — Systematic debugging (vector=security; trace JWT→DB authz layering)
- [ ] C2 — Seed data
- [x] C3 — Dev instance running (need DB to verify role-revocation scenario reaches a 403, not 2xx)
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion (cargo fmt + clippy + cargo test required by `backend.yml`)
- [x] C7 — Code-review reception (authz change; expect senior review)

Mode: cloud-ok

## Repro steps
1. Bring up the local stack with seed: `stack up pm-local` (or via `ppt-bridge` MCP).
2. Authenticate as a user whose JWT carries `"super_admin"` in `roles[]` and obtain `access_token`.
3. As that user, hit a platform-admin write endpoint (e.g. `POST /api/v1/subscriptions/admin/plans` with a valid body) — receive **201 / 200** (current behaviour).
4. With the access token still un-expired, set the user's DB role to a non-admin tenant role (or remove the platform_admin grant) via direct SQL or the admin/users_lifecycle endpoint.
5. Re-issue the same `POST /api/v1/subscriptions/admin/plans` request using the original access token.
   - **Expected:** 403 Forbidden (DB role revoked).
   - **Actual on `dev`:** 201 / 200 — `require_super_admin` reads stale JWT role claim, never consults DB.

## Suggested approach
1. In `backend/servers/api-server/src/lib.rs:276-280`, wrap the `/api/v1/subscriptions` admin sub-router and the `/api/v1/feature-packages` router (line 347) with `axum::middleware::from_fn(require_platform_admin)` from `api_core::middleware::authorization`. Keep the manager-facing read endpoints (`get_active_plans`, public coupon validation, etc.) on the non-admin sub-router so anonymous / authenticated-non-admin reads still flow.
2. In `backend/servers/api-server/src/routes/subscriptions.rs`, delete the local `SUPER_ADMIN_ROLES` constant (lines 49-55), `has_super_admin_role` (57-67) and `require_super_admin` (69-132). Replace every `require_super_admin(&headers, &state)?` call-site (8 of them: 353, 485, 518, 787, 1135, …) with the `AuthUser` extractor — `require_platform_admin` middleware has already gated the request by the time the handler runs, so the handler just needs the user id.
3. In `backend/servers/api-server/src/routes/feature_packages.rs`, repeat step 2 for its 6 call-sites (250, 285, 321, 365, 401, …) and the local helper (29-112).
4. Verify the router still mounts both layers in the correct order — `TenantExtractor` / `ValidatedTenantExtractor` MUST run before `require_platform_admin` so the `TenantRole` extension is populated; the lib.rs router already establishes this ordering for `routes/admin/*` (see lines around 142-146).
5. Update `backend/servers/api-server/tests/` — add a `subscriptions_platform_admin_authz_tests.rs` mirroring the pattern in `tests/infra_migration_platform_admin_tests.rs` (which already covers infrastructure + migration). Test (a) anonymous → 401, (b) authenticated non-admin → 403, (c) authenticated platform_admin → 2xx, (d) role-revoked-mid-session → 403.

## Alternatives considered
- **Add new `Capability::SubscriptionsWrite` + `Capability::FeaturePackagesWrite` variants and use `require_capability(...)`** — rejected because it expands the `Capability` enum (admin-core) for a one-off slice that doesn't need per-action granularity; the existing `require_platform_admin` already expresses "platform-tier write" exactly, matches `infrastructure.rs` / `marketplace.rs` precedent, and lands the same defence in one PR rather than two.
- **Keep `require_super_admin` and add a DB-roundtrip inside it** — rejected because every handler would still bear the JWT-then-DB check inline, drifting from the per-route layer pattern used elsewhere; also leaves the dead `SUPER_ADMIN_ROLES` constant and per-file role-list drift (the sibling finding `code-review-api-handlers-role-const-drift` — same constants differ between files).

## Root-cause trace
1. Symptom: a revoked super-admin retains full subscription/billing/feature-package mutation power until JWT TTL expires (~15 min by default).
2. ← Handler call-sites `require_super_admin(&headers, &state)?` (`subscriptions.rs:353` etc.) gate on JWT claims only, never re-validating against DB membership.
3. ← Local helper `require_super_admin` (`subscriptions.rs:69-132`) decodes the JWT and reads `claims.roles`; there is no `state.db_pool` lookup. Same shape in `feature_packages.rs:49-112`.
4. ← Router (`lib.rs:276, 347`) mounts these handlers without `require_platform_admin` middleware (which IS used for `routes/infrastructure.rs` and `routes/marketplace.rs` after PR #1741/#1746).
5. Origin: the per-route DB-backed authz migration wave landed for `routes/admin/*` and the marketplace/infrastructure slices but missed `subscriptions` + `feature_packages` — they were authored before the `require_platform_admin` middleware existed and never re-aligned.

## Test plan
- [ ] `backend/servers/api-server/tests/subscriptions_platform_admin_authz_tests.rs` — new file mirroring `infra_migration_platform_admin_tests.rs`. Cases: (a) anon → 401, (b) regular tenant user → 403, (c) platform-admin user → 2xx on a representative write (`POST /api/v1/subscriptions/admin/plans`), (d) the role-revoke-mid-token case (login, get token, revoke role via DB, re-request → 403).
- [ ] `backend/servers/api-server/tests/feature_packages_platform_admin_authz_tests.rs` — same matrix against `/api/v1/feature-packages` (POST/PUT/DELETE).
- [ ] Failing-on-`dev` test (IG3): test (d) currently returns 2xx because the handler reads only the JWT — the new middleware makes it 403. Include this case in the same file; it documents the regression that was previously latent.
- [ ] Local run: `cd backend && cargo test -p api-server --test subscriptions_platform_admin_authz_tests` and `... --test feature_packages_platform_admin_authz_tests` — both should pass after the fix.
- [ ] Full backend gate: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p api-server`.

## Out of scope
- Migrating `subscriptions.rs` non-admin endpoints (list active plans, public coupon validation) to a different gate — they remain readable as today (no behaviour change).
- Adding new `Capability::*` variants for sub-package granularity — explicitly rejected in *Alternatives*; that's a separate refactor.
- Touching the `marketplace.rs` / `infrastructure.rs` local `require_platform_admin` helpers — already DB-backed; they would benefit from being collapsed into the api-core middleware but that's a separate cleanup unrelated to this gap.
- Adding capability matrices to docs — leave the docs aligned with whatever the existing admin-core docs say.

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-platform-admin-jwt-only-gate.md`
- Mark the matching `backlog.json` row (`code-review-api-handlers-platform-admin-jwt-only-gate`) as `status: "done"`
