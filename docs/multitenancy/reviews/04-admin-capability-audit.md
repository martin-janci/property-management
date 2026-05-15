# Review #04 — Admin Surface, Capability Gating & Audit

Branch: `integration/multitenancy-phases-2-5p5`
Reviewer: Code Reviewer #4
Scope: `admin-core` crate, `/admin/*` route trees in `api-server`, capability/MFA/audit migrations.

---

## 1. Capability registry inventory

`Capability` enum lives at `backend/crates/admin-core/src/capability.rs:33-67`. Wired routes by capability:

| Capability | Routes consuming it | File |
|---|---|---|
| `AgenciesRead` | `GET /admin/agencies`, `GET /admin/agencies/{id}` | `routes/admin/agencies.rs:21,24` |
| `AgenciesWrite` | `POST /admin/agencies/{id}/domains` | `routes/admin/agencies.rs:32` |
| `AgenciesSuspend` | `POST /admin/agencies/{id}/suspend` | `routes/admin/agencies.rs:28` |
| `UsersRead` | `GET /admin/users`, `GET /admin/users/{id}` | `routes/admin/users.rs:23,27` |
| `UsersWrite` | (none) | UNUSED |
| `UsersImpersonate` | `POST/DELETE /admin/impersonation/*` | `routes/admin/impersonation.rs:29,33` |
| `MembershipsGrant` | `POST /admin/capabilities/grant` | `routes/admin/capabilities.rs:32` |
| `MembershipsRevoke` | `DELETE /admin/capabilities/{id}` | `routes/admin/capabilities.rs:36` |
| `SiteSettingsRead` | (none in HTTP routes) | only `SettingsStore` reads |
| `SiteSettingsWrite` | (none in HTTP routes) | `SettingsStore::set_raw` audits with this label |
| `MobileConfigWrite` | (none) | UNUSED |
| `FeatureFlagsWrite` | (none) | NOT wired to `admin_tenants::feature_flags_router` |
| `TenantExport` | (none) | NOT wired to `admin_tenant_lifecycle::export_handler` |
| `TenantPurge` | (none) | NOT wired to `purge_handler` |
| `TenantRestore` | (none) | NOT wired to `restore_handler` |
| `AuditRead` | `GET /admin/audit`, `GET /admin/capabilities/registry`, `GET /admin/capabilities/users/{id}` | `routes/admin/audit.rs:22`, `capabilities.rs:24,28` |
| `PrincipalKindEscalate` | `POST /admin/users/{id}/principal-kind` | `routes/admin/users.rs:32` (handler is a stub returning 501) |

Registry is initialized in `backend/servers/api-server/src/lib.rs:58` with `Capability::ALL`. `AdminDeps` and the `Arc<dyn …>` extensions are layered at `lib.rs:300-304`. Note: `main.rs` mounts the same routes at `lib.rs:78-82` AND again at `main.rs:397-402`; both paths instantiate routers, but only `lib.rs::create_router` wires the `Extension` layers — `main.rs` does NOT layer `AdminDeps`. This means **anything started from `main` directly (production binary) will fail every capability extraction** with `AdminError::Internal("AdminDeps missing from extensions")`. See finding #1.

---

## 2. Stub inventory — `require_platform_principal` migrations

| Site | Status | Recommended capability |
|---|---|---|
| `routes/admin/memberships.rs:35,76,226` (invite/revoke) | Stub on `RequestPrincipal::is_platform()` | `MembershipsGrant` (invite) / `MembershipsRevoke` (revoke). `accept` correctly does not require platform. |
| `routes/admin_tenants.rs:52` → `branding_router PUT` | Stub on legacy `extract_super_admin_token` | `SiteSettingsWrite` (or new `BrandingWrite`) |
| `routes/admin_tenants.rs:52` → `feature_flags_router GET/PUT` | Stub on legacy token | `AuditRead`/`FeatureFlagsRead` for GET; `FeatureFlagsWrite` for PUT (capability already exists, unused) |
| `routes/admin_tenant_lifecycle.rs:200` → `export_handler` | Stub on `AuthUser::is_platform_admin()` | `TenantExport` (capability exists, unused) |
| `routes/admin_tenant_lifecycle.rs:200` → `purge_handler` | Stub on `AuthUser::is_platform_admin()` | `TenantPurge` (capability exists, unused) |
| `routes/admin_tenant_lifecycle.rs:200` → `restore_handler` | Stub on `AuthUser::is_platform_admin()` | `TenantRestore` (capability exists, unused) |
| `routes/admin/users_lifecycle.rs:139` → `extract_admin_token` (suspend/reactivate/delete) | Pre-Phase-5; relies on JWT claim role string | Should at minimum migrate suspend/delete to `UsersWrite` (currently unused) — leaks #10/#11 means JWT role claim is no longer authoritative under the new identity model. |
| `routes/api_ecosystem.rs` (15+ sites) and `routes/operations.rs` (20+ sites) | Use `auth.is_platform_admin()` directly | Out of Phase 5 scope but they bypass the gate the same way. FLAG. |

**Architectural concern (finding #2):** `admin-core`'s `RequireCapability` extractor itself still gates on `AuthUser::is_platform_admin()` (`extractor.rs:113`) which reads the **JWT-embedded role claim**. Phase 2's whole point (`principal.rs:11-17`) was that JWT role claims must NEVER be trusted. The extractor should switch to `RequestPrincipal::is_platform()` which re-derives kind from `users` server-side. The TODO at `extractor.rs:111` acknowledges this. Until swapped, an attacker with a stolen JWT carrying a forged `roles: ["super_admin"]` claim passes step 2.

---

## 3. Audit log analysis

### Append-only enforcement

Migration `00138_create_capability_grants.sql:154-174` defines `audit_logs_reject_mutation()` and attaches `BEFORE UPDATE` + `BEFORE DELETE` triggers. Implementation is **correct and load-bearing**: triggers run for every role including superuser and bypass-RLS contexts. `RAISE EXCEPTION 'audit_logs is append-only'` with errcode `feature_not_supported`. Tests `audit_append_only_tests.rs:35-76` assert the migration text contains both triggers.

Caveats:
- `TRUNCATE` is not blocked. A `TRUNCATE audit_logs` from a superuser still wipes the table. Add `BEFORE TRUNCATE` trigger or revoke `TRUNCATE` from all roles. (Finding #3)
- Trigger function uses `SECURITY INVOKER` (default). A superuser could `DROP TRIGGER` then mutate. The original migration 00025 should set ownership/grants to make this require an explicit migration. Currently the protection is best-effort.

### Payload hashing

`audit.rs:77-82` SHA-256-hashes payloads via `hash_payload`. `PgAuditWriter::record` (`audit.rs:111`) only stores the hash inside `details.payload_hash` — raw payload never touches the DB. Correct defense against accidental PII leakage. `RequireCapability` extractor never passes a payload (`extractor.rs:160-171`), so admin denial/allowance rows have `payload_hash: null` — that's intentional.

### Denied-event coverage

`audit_denied()` (`extractor.rs:183-211`) is called on:
- Step 2 fail (not-platform): YES (`extractor.rs:114-122`)
- Step 3 fail (no grant): YES (`extractor.rs:128-135`)
- Step 4 fail (stale MFA): YES (`extractor.rs:146-153`)
- Step 1 fail (unauthenticated, no `AuthUser`): **NO** — finding #4. Anonymous probes against admin routes leave no audit trail. The brief explicitly mentions "noisy abuse from a stolen credential" as a thing operators must see; an attacker with no token also produces no signal.
- `AdminDeps`/marker missing in extensions: NO — fails with `Internal` before any audit. Acceptable since this is a misconfiguration.

### `support_activity_log`

The brief mentioned `support_activity_log` as the target table for denied events. **It does not exist** in migrations. The implementation reuses `audit_logs` (`audit.rs:124-130`) with `action='resource_accessed'` and a `details.outcome` field. The `audit_logs.action` enum was not extended with allowed/denied actions — distinguishing requires JSON filter on `details->>'outcome'`. Functionally OK, but `/admin/audit` (`routes/admin/audit.rs:67-91`) does NOT expose a `details` filter, so a UI viewer cannot easily slice "all denied events for user X this hour." (Finding #5)

---

## 4. Self-grant defense (leak #21) audit

### App-layer enforcement

`PgCapabilityGrantsRepository::grant` (`capability.rs:281-296`) explicitly rejects `user_id == granted_by` with `AdminError::Internal("no_self_grant: …")`. The string error masks itself as a 500, not a 403 — finding #6 (cosmetic). The check is correctly placed BEFORE the INSERT.

### DB-layer enforcement

`00138_create_capability_grants.sql` deliberately omits `CHECK (granted_by != user_id)` per the test `migration_protects_capability_grants_against_self_grant_at_app_layer`. This is a documented architectural choice but **single-layer**; if any future call site bypasses the trait method, the DB will accept it. Recommend adding the CHECK as belt-and-braces — the test contradicts the brainstorming session's defense-in-depth principle.

### Leak #12 — `PrincipalKindEscalate` self-promotion

`routes/admin/capabilities.rs:101-112`: granter must already hold `PrincipalKindEscalate` to grant it. The check uses `grants.user_has(auth.user_id, PrincipalKindEscalate)` which honors revocation/expiry. Correct.

**Hole (finding #7):** A platform principal with `MembershipsGrant` (the route's own gating capability) can grant capabilities OTHER THAN `PrincipalKindEscalate` to themselves' second account — or grant `MembershipsGrant` itself to a colluding shadow account. Combined with a stolen credential of any `MembershipsGrant` holder, this opens a chain to full takeover. Mitigations:
- Require `PrincipalKindEscalate` (or a new `CapabilitiesGrant` capability) instead of `MembershipsGrant` — naming is misleading too: `MembershipsGrant` reads as "grant org memberships", not "grant platform capabilities".
- Add a separate `PlatformCapabilityGrant` capability and gate `/admin/capabilities/grant` on it.

`auth.user_id` here comes from `AuthUser` (line 97) — same JWT-trust issue as finding #2.

### Frontend `RequirePlatformPrincipal` and information disclosure

Router refuses to render any admin tree if `isPlatformPrincipal=false` (`router.tsx:51-55`) and uses `<Navigate to="/" />` instead of "you do not have permission" — correct, leak #21 disclosure rule honored. Hook returns `false` for ALL capabilities when `!isPlatformPrincipal` (`useCapability.tsx:62`) — defense in depth.

---

## 5. Impersonation audit trail

### TTL

`IMPERSONATION_TTL = Duration::minutes(15)` (`impersonation.rs:24`). Asserted by test `token_ttl_is_15_minutes`. ✅

### Audit on start/end

`PgImpersonationService::start_impersonation` (`impersonation.rs:99-158`) writes audit row with `actor_id`, `target_user_id` (in `target_id` slot), `details.justifying_capability`, `details.expires_at`. ✅
`end_impersonation` (`impersonation.rs:160-194`) writes a second audit row with `details.action="end"`. ✅
`no_self_impersonation` enforced (`impersonation.rs:106-110`).

**Concern (finding #8):** start/end audit writes use `_ = self.audit.record(...).await` — errors are silently swallowed. If the audit writer fails, the impersonation token is still issued. The brief's threat model treats the audit row as load-bearing — token issuance should fail if audit fails (or the operation should be transactional). Compare with capability-grant handler (`routes/admin/capabilities.rs:114-125`) which propagates errors normally.

### Frontend banner

`<ImpersonationBanner>` (`packages/admin-ui/src/components/ImpersonationBanner/ImpersonationBanner.tsx`) is correctly built (sticky, `z-index:9999`, red, `role="alert"`, `aria-live="assertive"`). However, **no caller mounts it**. `grep -rn ImpersonationBanner frontend/apps` returns no hits. The component is exported (`packages/admin-ui/src/index.ts:26`) but not used by `ppt-web`. This is finding #9: the banner exists in isolation, so leak #21's "must be impossible to hide" is currently NOT enforced end-to-end.

The plain impersonation token is returned in the response body (`routes/admin/impersonation.rs:46-58`). There is no cookie set, no `Set-Cookie` header — host app must store and present it on subsequent requests, but no middleware reads it back to drive the banner. The whole "presence of impersonation token implies banner" loop is unimplemented on the frontend.

---

## 6. Frontend `useCapability` flow

- TS enum: `frontend/packages/admin-ui/src/capabilities.ts` (closed string union).
- Hook + provider: `frontend/packages/admin-ui/src/hooks/useCapability.tsx` — pure context, no fetch. The provider is fed `capabilities: ReadonlyArray<Capability>` and `isPlatformPrincipal: boolean`.
- Backend endpoint expected by the host app: `GET /api/v1/admin/capabilities/users/:user_id` (referenced in JSDoc at `useCapability.tsx:5` and `index.ts:6`).

The backend endpoint EXISTS at `routes/admin/capabilities.rs:62-72` and is gated on `Capability::AuditRead`. **Issue (finding #10):** the endpoint requires `AuditRead` — but a fresh platform principal with NO grants yet still needs to call this to discover they have none. Bootstrap problem: the user cannot list their own capabilities until they hold `AuditRead`. The frontend will get a 403 and render the admin tree as if `isPlatformPrincipal=false` (or the host app crashes). Recommend a `GET /admin/capabilities/me` self-introspection endpoint with no capability gate (only `principal_kind == platform` required).

The `AdminRouter` is exported (`apps/ppt-web/src/features/admin/index.ts:10`) but **no caller mounts it** in `ppt-web`. Same gap as the impersonation banner — the React tree never instantiates `<AdminRouter>`. This is finding #11.

---

## 7. Verdict — top 5 issues

1. **`main.rs` does NOT layer `AdminDeps` extensions** — production `cargo run -p api-server` (which uses `main.rs`'s router builder rather than `lib.rs::create_router`) will return 500 `AdminDeps missing` on every `/admin/*` call. Tests pass via `lib.rs`. ❌ BLOCKING
2. **Capability extractor still trusts JWT role claim** (`extractor.rs:113` calls `AuthUser::is_platform_admin()` which reads `claims.roles`) — directly contradicts Phase 2's "never trust JWT role claims" guarantee. Should switch to `RequestPrincipal::is_platform()`. ❌
3. **Phase 5.5 lifecycle routes (`export`/`purge`/`restore`), Phase 3 branding/feature-flags, and Phase 2 memberships still use stub gates** despite their target capabilities (`TenantExport`, `TenantPurge`, `TenantRestore`, `FeatureFlagsWrite`, `MembershipsGrant`, `MembershipsRevoke`) already existing in the registry. Five capabilities are dead code; six routes accept any platform admin without per-capability scoping. ⚠️
4. **Frontend `<AdminRouter>` and `<ImpersonationBanner>` are exported but never mounted** in `ppt-web`. Capability-gated UI exists in isolation. The "impersonation must be visible" guarantee is not enforced end-to-end. ⚠️
5. **`/admin/capabilities/users/{id}` requires `AuditRead`** to list — bootstrap deadlock for fresh platform users. Need a `GET /admin/capabilities/me` self-introspection endpoint that is platform-principal-only. ⚠️

### Other findings

6. Self-grant rejection returns 500 not 403 (`capability.rs:293`).
7. `MembershipsGrant` is the gate on `/admin/capabilities/grant`, allowing lateral capability proliferation among platform admins. Misleading name + over-broad scope.
8. Impersonation audit-write errors are silently swallowed; token is issued anyway.
9. `audit_logs` triggers do not block `TRUNCATE`.
10. `/admin/audit` cannot filter by `details->>'outcome'`, making "show me denied events" impossible from the UI.
11. No `support_activity_log` table exists; reuse of `audit_logs` is functional but the brief implied a dedicated table.
12. Unused capabilities: `UsersWrite`, `MobileConfigWrite`. Either wire them or remove from registry.

### Summary

| Area | Status |
|---|---|
| `admin-core` extractor logic & ordering | ✅ |
| Capability registry + grants storage | ✅ |
| Audit append-only DB triggers (UPDATE/DELETE) | ✅ |
| Payload hashing | ✅ |
| Self-grant defense (app layer) | ✅ |
| Self-grant defense (DB layer) | ⚠️ Single-layer by choice |
| `PrincipalKindEscalate` granter check | ✅ |
| Impersonation TTL + audit chain | ⚠️ Audit failures swallowed |
| Frontend gating components | ⚠️ Built but unmounted |
| Production binary wiring (`main.rs`) | ❌ Missing `AdminDeps` layer |
| Phase 5/5.5/3/2 stub migration to `RequireCapability` | ❌ Still on stubs |
| JWT-claim independence of capability gate | ❌ Still reads JWT role |
| Bootstrap path for fresh platform principal | ⚠️ Self-introspection needs gate relaxation |
