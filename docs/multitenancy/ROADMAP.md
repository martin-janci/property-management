# Multitenant SaaS Roadmap — Phases 2–5.5

**Source:** `_bmad-output/brainstorming/brainstorming-session-2026-05-14-175627.md`
**Status as of 2026-05-15:**

| Phase | Title | Branch | State |
|-------|-------|--------|-------|
| 0 | Foundation hardening (RLS backfill, CI gate, retire orphan column) | `feature/phase-0-rls-hardening` | merged into Phase 1 branch |
| 1 | Keystone — tenant resolution (agency_domains, host middleware, RlsConnection, dev `/a/{slug}`, agency provisioning) | `feature/phase-1-tenant-resolution` | done, awaiting PR |
| 2 | Identity unification | `feature/phase-2-identity-unification` | this roadmap |
| 3 | Hosting & theming | `feature/phase-3-hosting-theming` | this roadmap |
| 4 | Publishing & global portal | `feature/phase-4-publish-channels` | this roadmap |
| 5 | Super-admin control plane | `feature/phase-5-superadmin-console` | this roadmap |
| 5.5 | Tenant lifecycle & operability | `feature/phase-5p5-tenant-lifecycle` | this roadmap |

The brainstorming session is the source of truth for *why*; this document is the source of truth for *what to build* in each phase.

Per session: every phase is "done well" (TDD, complete). MVP-first means *sequencing*, not cutting corners. P0 leak defenses are NOT optional — they are part of the design of the phase that owns them.

---

## Phase 2 — Identity Unification

**Status:** highest-risk phase (7 of 13 cluster-1/2 leaks land here).
**Branch:** `feature/phase-2-identity-unification` (off `feature/phase-1-tenant-resolution`).

### Goals

1. One `users` table covers public (house-hunters), staff (agency employees), and platform (super-admins).
2. `portal_users` collapses into `users`; reality-server adopts the unified identity stack.
3. Tenancy is expressed via membership rows, not embedded in tokens.
4. Tokens carry a user only — tenant authority is computed per request from `resolved-host ∩ memberships`, fail closed on mismatch (defense for leak #11).

### Deliverables

#### Schema

- **Migration `00127_users_principal_kind.sql`** — `ALTER TABLE users ADD COLUMN principal_kind VARCHAR(20) NOT NULL DEFAULT 'staff' CHECK (principal_kind IN ('public', 'staff', 'platform'))`. Backfill: existing `users` rows = `staff`; existing super-admins (carried over from Phase 0 `platform_admin` table) = `platform`.
- **Migration `00128_user_memberships.sql`** — explicit membership rows: `(user_id, organization_id, role, granted_by, granted_at, expires_at, revoked_at)`. Capability-gated, audited (every grant/revoke writes `SupportActivityLog`). Invites: single-use, expiring, email-bound (defense for leak #9).
- **Migration `00129_principal_kind_guards.sql`** — DB-level trigger forbidding non-`platform`-driven UPDATEs to `principal_kind` (defenses for leaks #8 and #12). The only allowed transition is via a SECURITY DEFINER function called from an audited admin path.
- **Migration `00130_merge_portal_users_into_users.sql`** — copy `portal_users` → `users` with `principal_kind='public'`. Email collision policy: NEVER auto-merge — collisions land in a `user_merge_collisions` table for manual review (defense for leak #7). FK rewrites for any table referencing `portal_users.id`.

#### Backend

- **`api-core` extractors:** `RequestPrincipal` extractor that reads JWT (user id only), then resolves the *effective* tenant from `ResolvedTenant ∩ user_memberships` per request. If the tenant claimed in the URL/host is not in the user's membership set, return 403. (defense for leak #11)
- **`api-core::auth_policy`:** re-evaluate auth policy at every privilege change (defense for leak #13). Hooks into the per-org password/verification policy work paused on `feature/per-org-auth-policy` — adopt that branch's `AuthPolicy` type.
- **Token format:** drop `org_id` claim. Keep `sub` (user id), `kind` (principal_kind), `iat`/`exp`. Authorization is computed server-side per request.
- **Membership API (under `/admin/memberships`):** capability-gated CRUD. Invite flow uses signed tokens, single-use, email-bound, 24h expiry.
- **`reality-server` identity stack:** adopt the same `RequestPrincipal` extractor and the unified `users` table (defense for leak #5 in reality-server context).
- **Super-admin promotion path:** out-of-band, requires a `platform`-kind issuer + a hardware-MFA verification step. Heavily audited (defense for leak #12).

#### Tests

- **DB-level guards:** principal_kind escalation tests (raw UPDATEs blocked outside the SECURITY DEFINER path).
- **Token-scope test:** a user with memberships in orgs A and B presenting a token to org C's host gets 403, not silent success.
- **Revocation test:** revoke membership → next request to that org's host gets 403 even with a still-valid JWT.
- **Merge collision test:** two `portal_users` with the same email → collision row written, neither auto-merged into existing `users`.
- **Invite test:** single-use, expiring, email-bound; replay attempts fail.

#### Acceptance gates

- All Phase 2 tests pass under `cargo test`.
- `rls_smoke_tests` still green (no regression).
- `check-rls-coverage.sh` gate covers new tables.
- A dedicated security review (cluster-1/2 leak walk-through) is recorded before merging Phase 2 to Phase 3 base.

---

## Phase 3 — Hosting & Theming

**Branch:** `feature/phase-3-hosting-theming` (off Phase 2).

### Goals

1. Caddy + on-demand TLS in front of both servers; `agency_domains` is the ask-endpoint source of truth.
2. `/tenant-config` endpoint serves branding (logo, colors, fonts, CSS vars) and per-tenant feature flags keyed by resolved host.
3. Frontend (Next.js reality-web) consumes branding via design tokens (`--ppt-*`) — no per-agency code fork.
4. SSG/ISR caching keyed by resolved host so branding does not leak across domains.

### Deliverables

#### Infra (Caddy)

- **`infra/caddy/Caddyfile`** — on-demand TLS block; `ask` directive points at `https://api.<platform-host>/internal/caddy-ask` which queries `agency_domains` and 200/403s. Wildcard cert for `*.<platform-host>` for subdomain plan; on-demand for custom domains. Defense for leak #4: ask-endpoint hard-rejects hosts not in `agency_domains` (rate-limited, no Let's Encrypt request without 200).

#### Backend

- **`api-server::routes::caddy_ask`** — `GET /internal/caddy-ask?domain=…` → 200 (proceed) or 403 (do not request cert). Rate-limited with a separate per-IP budget. Internal-only (binds to `127.0.0.1` or behind mTLS).
- **`api-server::routes::tenant_config`** — `GET /tenant-config` returns `{ tenant_id, name, branding: { logo, colors, fonts, css_vars }, feature_flags, locales }`. Keyed by `ResolvedTenant`. Cache headers `Vary: Host` + 60s `s-maxage`.
- **Migration `00131_create_agency_branding.sql`** — `(organization_id PK, logo_url, primary_color, accent_color, font_family, css_vars JSONB, updated_at)`. (Already partly created in Phase 1's `00108_create_agency_branding.sql` — extend.)
- **Migration `00132_create_tenant_feature_flags.sql`** — `(organization_id, flag_key, enabled, value JSONB, updated_at)`. Defense for #22 (per-tenant kill switch): a `building_disabled` flag flipped by super-admin returns 503 for that tenant.

#### Frontend (reality-web)

- **`src/lib/tenant-config.ts`** — server-side fetch of `/tenant-config` keyed by host. Memoized per request.
- **`src/middleware.ts`** — read `Host`, fetch `/tenant-config`, set context for the request. SSG/ISR pages opt into `revalidateTag('tenant:<id>')`.
- **`src/app/[locale]/layout.tsx`** — inject `--ppt-*` CSS variables + apply `branding.font_family` + `<link rel="icon" href="{logo_url}">`. The platform-host case (no resolved tenant) uses the default reality-portal branding.
- **`src/lib/feature-flags.ts`** — typed flag accessor; SSR-safe; supports the `building_disabled` kill switch.

#### Tests

- **Caddy ask-endpoint:** unknown host → 403, known → 200, rate-limited under flood (defense #4).
- **Tenant config isolation:** request as agency A returns A's branding; B's branding is never in the cache key for A.
- **Kill switch:** `building_disabled=true` → 503 page render, no leakage of agency content.

---

## Phase 4 — Publishing & Global Portal

**Branch:** `feature/phase-4-publish-channels` (off Phase 3).

### Goals

1. Listings carry an explicit publish state.
2. Global portal sees published listings across all agencies via a 4th RLS context, not via a sync pipeline (invariant I-D).
3. Sequence: ship binary `is_published` first; channel-set evolution stays compatible.

### Deliverables

#### Schema

- **Migration `00133_listings_publish_state.sql`** — `ALTER TABLE listings ADD COLUMN is_published BOOLEAN NOT NULL DEFAULT FALSE, ADD COLUMN published_at TIMESTAMPTZ`. Backfill from current visibility flag (audit existing column first).
- **Migration `00134_listings_global_read_policy.sql`** — drop existing `listings_tenant_isolation` policy, replace with: `is_super_admin() OR organization_id = get_current_org_id() OR (is_published AND is_global_read_context())`. New SECURITY DEFINER function `is_global_read_context()` returns true when the connection's request-scope flag `request.global_read = on`.

#### Backend

- **`api-core::extractors::HostRlsConnection`** — when `ResolvedTenant.source == PlatformHost` (new variant), `SET LOCAL request.global_read = on` instead of setting an `org_id`. This is the 4th RLS context: published-everywhere read.
- **`reality-server` listing routes:** when running on the platform host, queries see the union of published listings across orgs. When running on an agency host, queries see all of that agency's listings (published + drafts).
- **`api-server::routes::listings::publish`** — POST `/listings/{id}/publish` and `/unpublish`. Capability-gated to `listings:publish` membership role. Audited.

#### Tests

- **4th RLS context test:** insert published + unpublished listings across two agencies; platform-host context sees only published from both; agency-A context sees both of A's; agency-B sees none of A's.
- **Publish/unpublish test:** unpublish hides from platform-host while preserving on agency-host (single source of truth, I-D).
- **No-sync test:** there is no background job dispatching listings — assert no scheduler entry exists for "publish-sync".

---

## Phase 5 — Super-admin Control Plane

**Branch:** `feature/phase-5-superadmin-console` (off Phase 4).

### Goals

1. Super-admin mechanism = capability-gated, never role-boolean.
2. Mechanism lives in shared crates; both servers honor it identically.
3. One unified console (ppt-web) with domain areas (Reality / PPT / platform-wide).
4. Mandatory audit + MFA for `platform` principals (defense #21).

### Deliverables

#### Crates

- **`backend/crates/admin-core`** — new crate:
  - `Capability` enum + `CapabilityRegistry` (registers capabilities at startup; constants like `Capability::AgenciesWrite`, `Capability::SiteSettingsWrite`, `Capability::MobileConfigWrite`, `Capability::Impersonate`).
  - `RequireCapability` extractor that checks `RequestPrincipal.kind == Platform AND capability ∈ principal_capabilities`. On allow, writes a `SupportActivityLog` entry with `(actor, capability, target, timestamp, ip, user_agent, payload_hash)`.
  - `AuditWriter` trait + Postgres implementation. Drop-only — never UPDATE/DELETE on audit rows.
  - `SettingsStore` trait — generic per-tenant settings (`agency_branding`, `mobile_config`, etc.) backed by `tenant_settings` table.
- **`backend/crates/admin-core` consumed by both `api-server` and `reality-server`** under `/admin/*` route trees.

#### Frontend

- **`frontend/packages/admin-ui`** — new TS package:
  - `<ResourceTable>` — generic CRUD table with capability checks.
  - `<SettingsForm>` — schema-driven form bound to `SettingsStore`.
  - `<AuditViewer>` — paginated audit-log viewer with filters.
  - `<ImpersonationBanner>` — visible during impersonated sessions.
- **`frontend/apps/ppt-web/src/features/admin/`** — unified console:
  - `pages/agencies` — list/create/edit agencies + their domains + branding.
  - `pages/users` — global user search, principal_kind transitions (audited), membership grants.
  - `pages/feature-flags` — per-tenant kill switches.
  - `pages/audit` — `<AuditViewer>` for any actor/target.
  - `pages/platform` — platform-wide settings (welcome page, default features).

#### Schema

- **Migration `00135_create_tenant_settings.sql`** — `(organization_id, key VARCHAR, value JSONB, updated_at, updated_by)`.
- **Migration `00136_capability_grants.sql`** — `(user_id, capability VARCHAR, granted_by, granted_at, expires_at, revoked_at)`. Defense #21: MFA enforced at the API layer for any capability grant; `platform`-principal creation is its own out-of-band flow (the `00129_principal_kind_guards.sql` SECURITY DEFINER path).

#### Tests

- **Capability gate:** non-platform principal hitting an admin route → 403, audit log row written.
- **Impersonation test:** impersonation requires `Capability::Impersonate`; banner present in client; audit row links impersonator → impersonated.
- **MFA enforcement test:** capability grant without recent MFA challenge → 403.

---

## Phase 5.5 — Tenant Lifecycle & Operability

**Branch:** `feature/phase-5p5-tenant-lifecycle` (off Phase 5).

### Goals

Plug operability holes (#16, #17, #18, #19) so a tenant can be exported, restored, or GDPR-purged without a database engineer.

### Deliverables

#### Schema

- **Migration `00137_organizations_soft_delete.sql`** — `ALTER TABLE organizations ADD COLUMN deleted_at TIMESTAMPTZ`. RLS policy on every tenant table updated to `... AND deleted_at IS NULL` for the tenant context (super-admin still sees soft-deleted rows). Defense #16: soft-delete is the default; hard-restore is a tooling path, not a UI button.

#### Backend

- **`backend/crates/admin-core::tenant_lifecycle`** — module:
  - `export_tenant(org_id) -> TenantExport` — walks the tenant-data manifest from Phase 0's `check-rls-coverage.sh` (the same source of truth — defense #17), serializes per-table rows + S3 keys, returns a tarball location. Capability-gated to `Capability::TenantExport`.
  - `purge_tenant(org_id) -> PurgeReport` — purges per the manifest, S3-included; transactional per table; emits an audit row with the manifest version. Defense #17: CI test fails if the manifest is stale relative to schema.
  - `restore_tenant_export(path)` — admin-only restore from an export tarball into a fresh `organization_id`. Defense #16: never restores in place.
- **`api-server::routes::admin::tenant_lifecycle`** — POST `/admin/tenants/{id}/export`, `/purge`, `/restore`. All audited, all capability-gated, all behind MFA.

#### Keystone middleware finalization

- **Per-tenant rate limits (defense #15):** sliding-window limiter keyed by `ResolvedTenant.organization_id`. Configurable per-tenant in `tenant_settings`.
- **Per-tenant metering (defense #19):** request count + DB query count + bytes-out tagged with tenant id, exported via `/metrics`. Defense gives a billing/abuse-detection foundation.
- **Tenant-prefixed cache wrapper (defense #20):** `TenantedRedis` wrapper that prefixes every key with `t:<org_id>:`. Direct `redis::Client` use forbidden by a clippy lint or grep CI gate.

#### Backups & Ops Doc

- **`docs/multitenancy/operability.md`** — runbook covering: encrypted backup configuration (defense #18), MFA for `platform` principals (defense #21, finalized here), break-glass procedure, GDPR purge SLA, restore test cadence.

#### Tests

- **Export round-trip:** export tenant → drop tenant → restore export → equality across the manifest.
- **Purge completeness:** generated manifest test asserts every tenant-scoped table is in the purge plan; CI fails on a new tenant table without a purge entry.
- **Rate-limit test:** flooding requests for tenant A does not affect tenant B's latency.

---

## Cross-phase invariants (carry through every phase)

- **I-A:** agency data isolated except the explicit, revocable publish channel.
- **I-B:** `principal_kind` enforced rigorously (Phase 2 lands the DB guard).
- **I-C:** super-admin sees across tenants through one mechanism (Phase 5 finalizes; Phase 0 retired the orphan column).
- **I-D:** a listing exists once; agency view and global view are filters of the same row (Phase 4 lands the policy).
- **I-E:** request tenant resolved from trusted host before any data is touched (Phase 1's keystone — already shipped).

## Cross-phase rule (one sentence)

> Every request resolves tenant + authz from trusted server-side state, never from client input, and fails closed.

If a piece of code in any phase violates this rule, that piece of code is wrong — not the rule.
