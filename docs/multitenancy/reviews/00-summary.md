# Multitenancy Integration — Review Summary

**Branch:** `integration/multitenancy-phases-2-5p5` (HEAD `813db045`)
**Reviews:** 6 parallel agents, all read-only
**Date:** 2026-05-15

| # | Reviewer focus | ✅ | ⚠️ | ❌ | Report |
|---|----------------|----|----|----|--------|
| 1 | Leak-defense traceability (22 leaks) | 9 | 7 | 3 | `01-security-leak-defenses.md` |
| 2 | RLS + migrations | 8 | 5 | 3 | `02-rls-and-migrations.md` |
| 3 | Phase 2 identity deep dive | 5 | 1 | 1 (out of 7) | `03-phase2-identity-deep-dive.md` |
| 4 | Admin / capability gating | — | 4 | 1 | `04-admin-capability-audit.md` |
| 5 | Cross-phase integration | — | — | 10 semantic bugs | `05-cross-phase-integration.md` |
| 6 | Frontend (admin-ui, theming) | — | 6 | 2 | `06-frontend.md` |

## The 7 most critical findings (consolidated, dedup'd)

These are surfaced by ≥2 reviewers and would all break in production despite the integration branch compiling clean. Each is **blocking**.

### B1 — Rate-limit & metering are dormant (#15, #19) — Reviewers 1, 5

`TenantRateLimiterSet` and `meter_request` are defined and unit-tested in `api-core::middleware::tenant_ops`, but the keystone `host_tenant_middleware` only contains `// SEAM(leak#XX)` comments. **Nothing is instantiated, nothing is invoked.** `operability.md` admits this; `INTEGRATION-PLAN.md` does not. One tenant can starve every other today.
- Fix: instantiate `TenantRateLimiterSet` in `AppState`, wrap `host_tenant_middleware` to invoke it after `ResolvedTenant` is set, call `meter_request` in the response path.

### B2 — Migration 00140 leaves duplicate listings policies — Reviewers 2, 5

The Phase 5.5 soft-delete-filter migration was generated against a base that didn't include Phase 4. So 00140 emits a stray `listings_tenant_isolation` (2-context) policy alongside the still-present `listings_four_context` (4-context). Postgres ORs them — soft-deleted tenants' published listings remain visible on both global portal and agency host. **Defense #16 (soft-delete) broken end-to-end for `listings`.**
- Fix: regenerate 00140 against the integrated schema, OR add a `00141_fix_listings_soft_delete.sql` that DROPs the redundant `listings_tenant_isolation` and rewrites `listings_four_context` to include `AND get_current_org_not_deleted()`.

### B3 — Stale tenant-data manifest — Reviewer 2

`backend/manifests/tenant-data-manifest.json` is from `2c56505f` (`migration_count=127`). It omits every Phase 2-5 tenant table (`user_memberships`, `user_invites`, `agency_branding`, `tenant_feature_flags`, `tenant_settings`). Phase 5.5's purge/export silently leaves tenant data in those 5 tables. **Defense #17 (GDPR purge completeness) hole.**
- Fix: re-run `bash backend/scripts/check-rls-coverage.sh --emit-manifest` on the integrated schema, commit the regenerated manifest. Add a CI gate (`purge_completeness_tests`) that fails if the manifest is older than the latest migration timestamp.

### B4 — RequireCapability extractor trusts JWT role claim (defeats #10, #11, #21) — Reviewers 4, 5, 1

`admin-core::extractor::RequireCapability` calls `AuthUser::is_platform_admin()`, which reads the JWT `roles` claim. **This directly contradicts Phase 2's "JWT role claims are never trusted" guarantee.** The entire `/admin/*` tree is therefore vulnerable to token-claim forgery. `RequireCapability` must check `RequestPrincipal::is_platform()` (server-derived, per-request), not `AuthUser`.
- Fix: rewrite `RequireCapability::from_request_parts` to extract `RequestPrincipal` and check `principal_kind == Platform` from there.

### B5 — RequestPrincipal mishandles PlatformHost — Reviewer 5

`RequestPrincipal` for the `(Some(rt), PrincipalKind::Public|Staff)` branch calls `is_active(user, rt.organization_id)` — but for `PlatformHost`, `rt.organization_id == Uuid::nil()`. Result: every Public/Staff request to platform host returns 403 with a misleading "no active membership in organization 00000000-…" error. The `(Some(rt), PrincipalKind::Platform)` branch sets `effective_org = Some(Uuid::nil())` — a poisoned sentinel that downstream queries treat as a real org id.
- Fix: explicit `(Some(rt), _) if rt.source == TenantSource::PlatformHost` arm. For Platform principals, `effective_org = None`. For Public/Staff, return 403 with "platform host requires platform principal".

### B6 — AdminRouter is dead code — Reviewers 4, 6

The five `/admin/*` pages, `<RequirePlatformPrincipal>` gate, and capability provider are scaffolded but **`AdminRouter` is never mounted in `ppt-web/src/App.tsx`**. The whole admin section is unreachable from the browser. Compounding: every list is `[]`, every action is `console.warn('TODO')` — they're real components but data-layer stubs.
- Fix: mount `<AdminRouter />` under `/admin/*` in `App.tsx`, then wire the data layer for at least one page (agencies) end-to-end.

### B7 — main.rs production binary doesn't layer admin extensions — Reviewer 4

`api-server::main.rs::serve()` builds the router but does NOT layer `AdminDeps`/`AuditWriter`/`MfaRecency`/`CapabilityGrantsRepository`/`ImpersonationService` extensions. Only `lib.rs::create_router` (used by tests) does. **Production hits 500 on every `/admin/*` call.**
- Fix: copy the `Extension(...)` chain from `lib.rs::create_router` into `main.rs::serve()` (or refactor so both share one helper).

## Important non-blocking findings

### N1 — Reality-server didn't adopt unified identity (Reviewer 3)
ROADMAP listed Phase 2 deliverable: reality-server adopts `RequestPrincipal` + unified `users`. Not done. `portal_users` still owns portal authn. `update_user`/`update_password_hash`/SSO upsert do not mirror into `users`. Phase 2's "one identity per human" promise is half-done.

### N2 — Defense #13 (wrong-tenant policy re-eval) entirely missing (Reviewer 1, 3)
No `AuthPolicy` module, no per-org auth-policy code, no test. The paused `feature/per-org-auth-policy` branch was never adopted. A P0 leak silently dropped.

### N3 — set_principal_kind allows actor forgery (Reviewer 3)
The function accepts a caller-supplied `actor` UUID with no cross-check against `app.current_user_id`. Audit attribution can be forged. Hardware-MFA promotion path is also unbuilt.

### N4 — Phase 5.5 lifecycle routes use stub gate (Reviewers 4, 5)
`/admin/tenants/{id}/{export,purge,restore}` use `AuthUser::is_platform_admin()` instead of `RequireCapability(TenantExport|TenantPurge|TenantRestore)`. No MFA, no admin-core audit.

### N5 — Phase 2/3 stub gates not migrated (Reviewers 4, 5)
`memberships`, `admin_tenants` (branding/feature-flags), `admin_tenant_lifecycle` still use `require_platform_principal` despite their target capabilities (`MembershipsGrant`, `MembershipsRevoke`, `FeatureFlagsWrite`, `TenantExport/Purge/Restore`) already defined as dead code in the registry.

### N6 — Cache invalidation gap on suspend/purge/soft-delete (Reviewer 5)
`AgencyDomainRepository::register/release` doesn't invalidate the `TenantResolutionCache` — a 5-minute stale-resolution window after any agency change.

### N7 — Branding sanitizer CSS-url bypass (Reviewer 6)
`tenant-config.ts` denies `"`, `<`, `>`, `{`, `}`, `;` but allows `url(…)` and `'`. An agency-controlled `css_vars` row could inject `url(//evil.com)` if any consumer uses the var as a `background-image`.

### N8 — building_disabled returns 200, not 503 (Reviewer 6)
The kill-switch page renders inside the layout (HTTP 200) instead of a real 503. CDN/monitoring/SEO mistreat it as a normal page.

### N9 — MFA UX missing entirely (Reviewer 6)
No challenge modal, no 401 interceptor, no re-auth flow. Admin actions requiring recent MFA will 401 silently.

### N10 — Capability bootstrap deadlock (Reviewer 4)
`/admin/capabilities/users/{id}` requires `AuditRead`. A fresh platform principal cannot self-introspect to discover what they have. Need `GET /admin/capabilities/me` gated only by platform-principal.

## Recommended pre-merge action plan

**Day 1 — fix B-blockers (must, in this order):**
1. B7 (main.rs extensions) — 30-min copy-paste from lib.rs.
2. B4 (RequireCapability uses RequestPrincipal) — 1h refactor + tests.
3. B5 (RequestPrincipal PlatformHost branch) — 1h + 4-context tests.
4. B2 + B3 (regenerate manifest, regenerate 00140) — 1-2h.
5. B1 (wire rate-limit + metering into middleware) — 2-3h + tests.
6. B6 (mount AdminRouter, wire one page end-to-end) — 2-4h.

**Day 2 — N-issues that block production confidence (pick 4):**
- N4 + N5 (migrate stub gates to real capabilities) — bundled, 1-2h.
- N3 (set_principal_kind enforces actor = current_user_id) — 30min.
- N6 (cache invalidation on agency_domains write) — 1h.
- N8 (building_disabled returns 503) — 30min.

**Defer to follow-up PR (not blocking):**
- N1 (reality-server unified identity) — call this Phase 2.5.
- N2 (auth-policy re-eval, defense #13) — call this Phase 2.6 or merge with the paused branch.
- N7, N9, N10 — frontend hardening sprint.

## Score before vs. after

| Phase | Defenses claimed | Defenses actually present (review) | Gap |
|-------|------------------|-----------------------------------|-----|
| 0 | 5 | 5 | 0 |
| 1 | 4 | 4 | 0 |
| 2 | 7 | 5 fully + 1 weak + 1 missing | -2 |
| 3 | 2 | 2 (one with sanitizer hole) | -0.5 |
| 4 | 1 invariant (I-D) | 1 (with B2 regression) | -0.5 |
| 5 | 1 | 0 (RequireCapability broken) | -1 |
| 5.5 | 8 ops items | 5 (rate limit + metering dormant; manifest stale) | -3 |

**Total real defenses: 22 - 7 to fix - 2 deferred = 13 fully landed.** The integration branch is closer to a "scaffolded prototype" than to "Phase 5.5 complete." The path to "complete" is the action plan above, ~2 days of focused fixes.
