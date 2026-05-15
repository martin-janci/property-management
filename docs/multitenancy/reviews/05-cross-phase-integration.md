# Cross-Phase Integration Semantics — Reviewer #5

**Branch:** `integration/multitenancy-phases-2-5p5`
**Scope:** semantic bugs that emerge ONLY because phases 2–5.5 now sit together. Compile is clean; these would all execute without panicking and produce wrong runtime behavior.

---

## 1. Keystone middleware composition trace

`backend/crates/api-core/src/middleware/host_tenant.rs::host_tenant_middleware`

| Step | Phase | Behavior | Status |
|------|-------|----------|--------|
| 1. Public allowlist (`/health`, `/internal`, …) | 1+3 | Bypass entirely; `/internal` added in Phase 3 for Caddy ask | OK |
| 2. Trusted-host extraction (`Host` header; `X-Forwarded-Host` only if `TRUST_FORWARDED_HOST=true`) | 1 | Defends leak #1 | OK |
| 2.5 PlatformHost short-circuit (compare `host` to `cfg.platform_hosts`) | 4 | Runs **before** DB lookup → no wasted query | OK |
| 3. Dev-mode `/a/{slug}` rewrite | 1 | DB-backed; gated by `dev_mode` from `RUST_ENV` | OK |
| 4. Cache lookup → `AgencyDomainRepository::resolve_host_system` on miss | 1 | Single-flight via cache; positive 300 s / negative 30 s TTL | OK |
| 5. Soft-resolve allowlist (`/tenant-config`) for unresolved/missing host | 3 | Pass-through (no `ResolvedTenant` extension) | OK |
| 6. Fail closed → `404` for unknown hosts | 1 | OK |
| **5.5 SEAM(leak#15) — per-tenant rate-limit call** | 5.5 | **MISSING (comment-only)** | **BUG #1** |
| **5.5 SEAM(leak#19) — per-tenant metering call** | 5.5 | **MISSING (comment-only)** | **BUG #1** |

The "SEAM" markers in `host_tenant.rs` lines 481–484, 510–514, 549–553 are pure comments. `tenant_ops::TenantRateLimiterSet` and `tenant_ops::meter_request` are defined and unit-tested but **never instantiated, never put in `AppState`, never invoked from the keystone middleware**. Both defenses #15 and #19 are advertised as wired in `INTEGRATION-PLAN.md` and `ROADMAP.md` but are dormant. There is no `429` path and no `requests_total{org_id=…}` counter being emitted. Compile is clean because the symbols are merely unused in production callers.

---

## 2. PlatformHost × soft-delete × global-read interaction matrix

Tuple = (tenant_state, principal_kind, host_kind) → expected vs. actual behavior.

| tenant_state | principal_kind | host_kind | Expected | Actual |
|---|---|---|---|---|
| alive | Public | Agency host (member) | 200, RLS = org | OK |
| alive | Public | Agency host (non-member) | 403 | OK (RequestPrincipal step 3) |
| alive | Public | PlatformHost | 403 OR pass-through with `effective_org=None` | **403 always** — `RequestPrincipal` calls `is_active(user, Uuid::nil())` → false → 403. See **BUG #2**. |
| alive | Staff | PlatformHost | 403 | Same as above — incidentally correct, but for the wrong reason. |
| alive | Platform | Agency host | 200, `effective_org = Some(host_org)` | OK |
| alive | Platform | PlatformHost | 200, `effective_org = None` | OK |
| **soft-deleted** | Public/Staff | Agency host (member) | 0 rows on tenant tables; explicit 503/410 ideal | **0 rows on `tenant_isolation`-style policies (00140 OK), but `listings` still readable on agency host** (see **BUG #3**) |
| **soft-deleted** | (any) | PlatformHost (global portal) | published listings hidden | **published listings still visible** — 00140 left `listings_four_context` policy untouched, so the global-read OR-clause has no `get_current_org_not_deleted()` filter (**BUG #3**) |
| alive | none (anon) | PlatformHost | published listings union | OK (HostRlsConnection sets `app.global_read = on`) |
| (any) | (any) | Spoofed `Host: reality.example.com` against api.acme.com | 404 / rejected | Resolves as PlatformHost. Mitigated upstream by Caddy/proxy SNI, but **see Bug #6 for in-app source-of-truth split**. |

### `RequestPrincipal` × `PlatformHost` (BUG #2)

`backend/crates/api-core/src/extractors/principal.rs:135–180`

```rust
let effective_org = match (resolved, kind) {
    (Some(rt), PrincipalKind::Public) | (Some(rt), PrincipalKind::Staff) => {
        let active = repo.is_active(user_id, rt.organization_id).await…
        if !active { return Err((FORBIDDEN, …)); }
        Some(rt.organization_id)
    }
    (Some(rt), PrincipalKind::Platform) => Some(rt.organization_id),
    …
};
```

The match arm does NOT branch on `rt.source == TenantSource::PlatformHost`. For a PlatformHost request:
- `rt.organization_id == Uuid::nil()`, `rt.source == PlatformHost`.
- Public/Staff principal: `is_active(user, Uuid::nil())` → DB returns false → **403 "no active membership"**. The error message is misleading — there is no org to be a member of; this is the global portal.
- Platform principal: `effective_org = Some(Uuid::nil())`. Downstream code that uses `effective_org` as a real org id will silently misbehave (e.g. queries filtered by `org_id = Uuid::nil()` return 0 rows; FK inserts fail).

The contract from PlatformHost docs (`host_tenant.rs:67–69`) explicitly says "Code that branches on `source == PlatformHost` MUST NOT use the `organization_id` field as a real tenant id." `RequestPrincipal` violates this contract.

### `listings_four_context` × soft-delete (BUG #3)

`backend/crates/db/migrations/00140_rls_soft_delete_filter.sql:1310–1314`:

```sql
DROP POLICY IF EXISTS listings_tenant_isolation ON listings;
CREATE POLICY listings_tenant_isolation ON listings FOR ALL
    USING (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted())
    WITH CHECK (((is_super_admin() OR (organization_id = get_current_org_id()))) AND get_current_org_not_deleted());
```

Phase 4's migration 00136 dropped `listings_tenant_isolation` and created `listings_four_context` with the global-read OR-clause. After 00140 runs:

1. `DROP POLICY IF EXISTS listings_tenant_isolation` is a no-op (it already does not exist).
2. `CREATE POLICY listings_tenant_isolation` re-creates the OLD 2-context policy alongside the still-present `listings_four_context`.

Postgres ORs PERMISSIVE policies, so the four-context policy's global-read clause **still works** for live tenants — but `listings_four_context` was NEVER updated to AND `get_current_org_not_deleted()` into its global-read OR-arm. Net effect:

- A soft-deleted tenant's `is_published = TRUE` listings remain visible from the platform host (because `listings_four_context` still says `is_published AND is_global_read_context()`, full stop). Defense #16 is partially defeated for the global portal.
- For agency-host queries the soft-delete filter works, because the new `listings_tenant_isolation` policy adds `AND get_current_org_not_deleted()` and the `listings_four_context` policy's `organization_id = get_current_org_id()` arm does NOT have the filter — but Postgres ORs the two, so the unfiltered arm wins, meaning **even on the agency host the soft-deleted org still sees its own listings**. Defense #16 is defeated end-to-end for `listings`.

Root cause: the generation script (next section) never knew about `listings_four_context`.

---

## 3. Migration 00140 audit

`backend/scripts/generate-soft-delete-rls-migration.sh`

The script applies all migrations EXCEPT 00140 itself, then introspects `pg_policies` and emits `DROP/CREATE` pairs. The implementation is mechanical and per-table — no special-casing.

After 00136 runs, `pg_policies` for table `listings` contains the row `policyname = 'listings_four_context'`, NOT `'listings_tenant_isolation'`. So the script SHOULD emit:

```sql
DROP POLICY IF EXISTS listings_four_context ON listings;
CREATE POLICY listings_four_context ON listings FOR ALL
    USING ( <existing 4-context expr> AND get_current_org_not_deleted() )
    WITH CHECK ( <existing 2-context expr> AND get_current_org_not_deleted() );
```

But the committed file (`00140_rls_soft_delete_filter.sql:1313–1314`) emits the OLD `listings_tenant_isolation` name with the OLD 2-context predicate. This proves the committed migration was generated from a DB state where 00136 had NOT yet been applied — i.e. it was generated on the Phase 5.5 worktree before Phase 4 was merged. The script was NEVER re-run after the integration merge.

**This is exactly the "Phase 4 listings bespoke policy regression" the prompt anticipated.** It's not a script bug per se (the script trusts whatever `pg_policies` returns); it's an integration-process bug. Severity is high because it silently re-creates a 2-context policy with a name that *looks* like the canonical one, and any reviewer scanning for "listings policy" sees both and assumes the 4-context one wins (which it does for live orgs but not for soft-deleted ones).

Recommended fix: re-run `bash backend/scripts/generate-soft-delete-rls-migration.sh` on a DB with all migrations 00127–00139 applied, replace 00140 with the fresh output, AND drop the orphan `listings_tenant_isolation` create at the start (or have the script prefer the existing policy name + predicate, which it already does — the file just needs regeneration).

---

## 4. Stub inventory

Reviewer #4's findings on routes still using stubs are **confirmed and extended**.

| Route | File | Stub kind |
|---|---|---|
| `POST /admin/agencies/:id/domains` | `routes/admin/agencies.rs:168–178` | Returns `501 NOT_IMPLEMENTED`; comment says real provisioning lives in `agency_provisioning` (Phase 1) — never wired through capability gate. |
| `PUT /admin/agencies/:id/suspend` | `routes/admin/agencies.rs:144–159` | Calls real repo, but does NOT invalidate `tenant_resolution_cache` for the agency's hosts. Cached entries continue to resolve to the suspended org for up to 300 s. |
| `POST /admin/tenants/:id/{export,purge,restore}` | `routes/admin_tenant_lifecycle.rs:200–209` | Uses `AuthUser::is_platform_admin()` instead of `RequireCapability`. Comment explicitly acknowledges this. No MFA gate, no `admin-core` audit row, no capability grant check. |
| `purge` does not invalidate the cache | `routes/admin_tenant_lifecycle.rs:107–125` | After purge, `agency_domains` rows are deleted but cached positive entries continue resolving to the purged org for up to 5 minutes — handlers will then fail at the FK boundary instead of cleanly 404-ing. |
| ppt-web admin pages | `frontend/apps/ppt-web/src/features/admin/pages/*.tsx` | Every page is a `// TODO(phase-5-followup): wire to GET /api/v1/admin/...` placeholder — no real fetches. Admin UI compiles and renders but cannot perform any action. |
| `<ImpersonationBanner>` integration | `packages/admin-ui/src/components/ImpersonationBanner/ImpersonationBanner.tsx` + `routes/admin/impersonation.rs:62–73` | Backend doc says "sets a cookie / returns the opaque token" but `start` only returns JSON. Banner is purely prop-driven — no host-app wiring committed. |

---

## 5. Cargo workspace consistency

| Check | Result |
|---|---|
| `members = [crates/admin-core, crates/tenant-ops, …]` | Both present in `backend/Cargo.toml` (lines 6, 9). |
| Workspace deps for both crates | Present (`backend/Cargo.toml:108, 111`). |
| `governor` dep | Single version `0.6` (api-core only). |
| `metrics` dep | Single version `0.21.1`. Macro syntax used (`metrics::increment_counter!`, `metrics::counter!`) matches 0.21 — fix `21d75679` was correct. |
| `axum` versions | Two: `0.8.9` (workspace) and `0.6.20` (transitive via `tonic`). No app code imports the 0.6 path; this is benign. |
| `rand` versions | Three: `0.8.6`, `0.9.4`, `0.10.1`. Phase 2's invite-token fix (`2ec71fab`) targeted `0.10` SysRng. Confirmed `rand 0.10` is in lock — OK. The other two come from `argon2` and other older deps. Benign duplication. |
| `Cargo.lock` regenerated post-merge | `INTEGRATION-PLAN.md:102` calls for `cargo update -w` at end. Lockfile shows post-merge state (admin-core, tenant-ops both present); no missing entries observed. |

No workspace/lockfile blockers. The metering/rate-limit code compiles but is not wired.

---

## 6. Other findings worth flagging

**BUG #4 — Phase 5 capability gate trusts JWT role claim**
`backend/crates/admin-core/src/extractor.rs:106–123` calls `AuthUser::is_platform_admin()`, which reads `self.role` populated from the JWT `role` claim (`extractors/auth.rs:107–111`). Phase 2's whole point (defenses #10, #11) is to never trust JWT-carried role/tenant. The Phase 5 admin tree therefore re-introduces a stale-token vulnerability: a user whose `principal_kind` was demoted from `platform` to `staff` in the DB still passes `is_platform_admin()` until their JWT expires. The fix is to swap `AuthUser` for `RequestPrincipal` in `extractor.rs` and check `kind == PrincipalKind::Platform` from the trusted DB lookup. The TODO is even acknowledged in the source comment ("Phase 2 will introduce `principal_kind`; for now …").

**BUG #5 — `TenantResolutionCache` invalidation gaps**
The only call site for `cache.invalidate(host)` is `routes/agency_provisioning.rs:296` (Phase 1). After Phase 5/5.5 added admin endpoints that mutate `agency_domains` ownership (suspend), `organizations` lifecycle (soft-delete, purge, restore), and Phase 5's stub `add_domain`, none of those handlers invalidate the cache. Defense #3 (cache poisoning) is only as strong as the call site that actually invalidates — most mutation paths now leak.

**BUG #6 — Two sources of truth for "platform host"**
`backend/crates/api-core/src/middleware/host_tenant.rs::load_platform_hosts` builds the resolver's set from `PLATFORM_HOST` env + the hardcoded fallback `reality.example.com`. The DB's `reserved_platform_hosts` table (migration 00135) is independently seeded with `reality.example.com`, `app.example.com`, etc. If an operator changes `PLATFORM_HOST` in env without updating the DB seed, an agency could later register one of the env-platform hosts (the `agency_domains_host_not_reserved` constraint only blocks DB-seeded hosts). The resolver would treat the host as platform; RLS and the constraint would treat it as agency. Mismatch surface. The roadmap calls this out ("the same list is seeded into the DB-side `reserved_platform_hosts` table") but there is no code that keeps them in sync.

**BUG #7 — `ResolvedTenant` not serializable / portability between servers**
`ResolvedTenant` is `Copy` but only lives in axum extensions. Reality-server consumes it via `HostRlsConnection`. There is no test asserting reality-server actually mounts `host_tenant_middleware` for routes that use `HostRlsConnection`. If any reality-server route uses `HostRlsConnection` without the middleware, it returns 500 ("middleware did not run"). Worth a one-line grep audit. (Did not find a clear violation in this pass.)

**BUG #8 — `agency_domains` policy itself now soft-delete-filtered (chicken/egg)**
`00140` line 65 wraps `agency_domains` policy with `AND get_current_org_not_deleted()`. The Caddy ask-endpoint (`/internal/caddy-ask`) hits `agency_domains` via `resolve_host_system`, which uses a system-level repo (`AgencyDomainRepository::resolve_host_system`) — this should bypass RLS via the system role. Verified `resolve_host_system` in `repositories/agency_domain.rs:54` exists, but the bypass mechanism (super-admin context vs. trusted role) was not re-checked in this pass. If RLS is in effect for this query, a soft-deleted org's domain would suddenly appear "unknown" to Caddy and TLS would stop renewing — silent breakage. Worth verifying.

---

## 7. Verdict

Severity ranking (semantic, compile-clean, runtime-fail bugs):

| # | Bug | Severity | Why |
|---|---|---|---|
| 1 | Rate-limit (#15) and metering (#19) middleware unwired | **Critical** | Two roadmap-promised defenses are dormant. No 429s, no per-tenant counters, no operability foundation. Defense #15 was a hard gate in the brainstorming session. |
| 2 | `RequestPrincipal` doesn't branch on `PlatformHost` source | **Critical** | Public/Staff principals get a misleading 403 on platform host. Platform principals get `effective_org = Some(Uuid::nil())` — a poisoned sentinel that downstream queries will treat as a real org id. The `host_tenant.rs` contract is violated. |
| 3 | Migration 00140 leaves `listings_four_context` un-soft-delete-filtered AND re-creates a stray `listings_tenant_isolation` policy | **Critical** | Soft-deleted tenants' published listings remain visible on both global portal and (because Postgres ORs the two policies) on agency hosts. Defense #16 broken for the most-exposed table. Re-running the generator script would fix it. |
| 4 | Phase 5 admin capability gate trusts JWT-claim role | **High** | Stale tokens grant admin access; defenses #10/#11 partially defeated for the entire `/admin/*` tree. |
| 5 | Tenant lifecycle routes use `AuthUser::is_platform_admin()`, not `RequireCapability` | **High** | Phase 5.5's destructive routes (export/purge/restore) bypass capability registry, MFA gate, and admin-core audit. Roadmap defenses #17/#21 partially defeated. |
| 6 | Cache invalidation gaps on suspend/purge/soft-delete | **High** | Up to 5-minute window where requests resolve to a no-longer-extant tenant. After purge: FK errors instead of 404. |
| 7 | Two sources of truth for platform-host list (env vs. DB seed) | **Medium** | Drift between `PLATFORM_HOST` env and `reserved_platform_hosts` table can produce a host treated as platform by resolver but agency-eligible by DB constraint. |
| 8 | `agency_domains` RLS now AND-filtered with soft-delete; system-repo bypass unverified | **Medium** | Caddy ask-endpoint may stop seeing soft-deleted orgs' domains, silently breaking TLS renewal. Worth confirming the system-role bypass. |
| 9 | Impersonation backend doc/code mismatch (says "cookie", returns JSON) | **Low** | Frontend integration is glue work; banner is purely prop-driven. |
| 10 | Three `rand` versions, two `axum` versions in lockfile | **Low** | Benign — none of the duplicates cross app boundaries. |

The integration is **not release-blocking on compile** but is **release-blocking on semantics** until at minimum bugs 1, 2, 3, 4, 5 are fixed. Bug 3 is the most insidious because it manifests only when an org is soft-deleted, which is rare and probably untested.

---

## 8. Suggested next actions

1. **Regenerate 00140** on a fully-migrated DB (with 00127–00139 applied) and replace the file. This fixes Bug #3 mechanically.
2. **Wire `TenantRateLimiterSet` into `AppState` + invoke `check()` and `meter_request()` in `host_tenant_middleware`** at the existing SEAM markers. PlatformHost should get a separate global limiter (or no limit) — design choice, but `nil()` keying is wrong as-is.
3. **Patch `RequestPrincipal`** to branch on `rt.source == PlatformHost` and set `effective_org = None` (or 403, depending on principal_kind). Add a regression test.
4. **Patch `RequireCapability`** to read `RequestPrincipal` instead of `AuthUser`, checking `kind == PrincipalKind::Platform` from the DB lookup.
5. **Add `cache.invalidate(host)` calls** in `suspend_agency`, `purge_handler`, `restore_handler`, and the future `add_domain` real implementation.
6. **Unify the platform-host list**: make `load_platform_hosts` query `reserved_platform_hosts` at boot rather than re-deriving from env, OR add a startup assertion that env and DB agree.
