# Repo Map — where things live (read before grepping the tree)

Purpose: a cached, coarse "where-things-live" index so agents resolve *where a feature
lives* by reading one file instead of fan-out grepping the monorepo. Kept deliberately
**coarse** (directories, naming conventions, hot files) so it rarely goes stale. For deep
detail, the nested `CLAUDE.md` files in each subtree are loaded on demand when you work there.

> If something here is wrong, fix it in the same PR as the code change — this file is part
> of the agent contract. Counts are approximate (snapshot 2026-07).

## Top-level layout

```
backend/        Rust workspace (Cargo) — accounting-server, api-server, deploy-server, reality-server + crates
frontend/       pnpm workspace — accounting-web, admin-web, mobile, ppt-web, reality-web + shared packages
mobile-native/  Kotlin Multiplatform — Reality Portal Android/iOS
docs/           specs, use-cases, API (typespec/OpenAPI), screen-maps, this map
scripts/        setup, version bump/sync, health-check, install-hooks
.research/      research dispatcher/routine, backlog, plans, management artifacts
.github/        CI workflows (backend.yml, frontend.yml, mobile-native.yml, api-validation.yml, …)
```

## The naming convention (this is the big time-saver)

A single domain (e.g. `faults`, `leases`, `esg_reporting`) is implemented as a **vertical
slice with matching file names** across layers. Once you know the domain noun, you know the files:

| Layer | Path pattern | Example (`faults`) |
|-------|--------------|--------------------|
| HTTP route/handler | `backend/servers/api-server/src/routes/<domain>.rs` | `routes/faults.rs` |
| DB repository | `backend/crates/db/src/repositories/<domain>.rs` (often singular) | `repositories/fault.rs` |
| Migrations | `backend/crates/db/migrations/*.sql` (timestamped, additive) | grep the noun |
| API contract | `docs/api/typespec/**` → generated OpenAPI → TS/Rust clients | — |
| Frontend client | `frontend/packages/api-client` (generated) | — |

To find a feature: map the noun → `routes/<noun>.rs` + `repositories/<noun>.rs`. Prefer this
over `grep -r <noun> backend/`.

## backend/ (Rust workspace)

**Servers** (`backend/servers/`):
- `accounting-server` (port 8082) — tenant-scoped accounting resource server (architecture
  option C). Consumers: accounting-web, ppt-web.
- `api-server` (port 8080) — Property Management API + OAuth provider. ~77 top-level route
  modules in `src/routes/` (~194 files total, incl. subdirs
  `accounting/ admin/ ai/ aml_dsa/ announcements/ documents/ emergency/ enhanced_tenant_screening/ forms/ integrations/ iot/ layout/ organizations/ platform_admin/ reserve_funds/ vendors/`).
  Consumers: ppt-web, admin-web, mobile.
- `reality-server` (port 8081) — Reality Portal public API. Multi-region via `REGION` env.
  Consumers: reality-web, mobile-native.
- `deploy-server` — deployment control plane (has its own migrations).

**Crates** (`backend/crates/`):
- `accounting-core` — accounting domain logic backing `accounting-server`. Pure logic.
- `common` — `TenantContext`, `TenantRole` (12 roles), core errors/types. Used everywhere.
- `layout-core` — Layout & Content Manager contract: screen configs, merge resolver
  (base → platform → tenant → kill), publish/rails validation. Pure logic, no DB.
  Control plane: `db/src/repositories/layout.rs` + migration 00221; routes at
  `api-server/src/routes/layout/` (admin + tenant + resolved) and
  `reality-server/src/routes/layout.rs` (public resolved).
  Spec: `docs/superpowers/specs/2026-07-19-layout-content-manager-design.md`.
  Publish webhook: `api-server/src/routes/layout/webhook.rs` → `reality-web/src/app/api/layout-revalidate`
  (POST, HMAC-SHA256 signature, Next.js ISR revalidation). Envs: `LAYOUT_WEBHOOK_URL`, `LAYOUT_WEBHOOK_SECRET`.
  Frontend: `ppt-web/src/features/layout/` + `reality-web/src/lib/layout.ts` +
  `reality-web/src/components/listings/LayoutSections.tsx` (registries, defensive
  renderers, checked-in web manifests for PUT /platform-admin/layout/manifests).
  Tenant editor: `ppt-web/src/features/layout/` (/dashboard/customize, org-admin entry point).
  Editor: `admin-web/src/features/layout-editor/` (routes `/platform/layout`, `/platform/layout/manifests`).
  Preview bridge: `@ppt/shared` layout-preview (postMessage bridge) + `admin-web/src/features/layout-editor/PreviewPanel` (iframe, framed ppt-web); preview-resolve endpoint (`/api/v1/platform-admin/layout/preview-resolve`) returns resolved layout for local (unsaved) drafts.
  Mobile: RN features/layout (dashboard, cached next-launch activation) + mobile-native shared/layout + Android registry dispatch + iOS listing detail via shared resolved layout dispatch (Swift compile-unverified on Linux; run scripts/build-ios.sh on macOS before release); canonical mobile manifest in apps/mobile.
- `api-core` — Axum extractors, auth middleware, OpenAPI (utoipa), CORS/tracing.
- `db` — SQLx pool, models, **~111 repositories** in `src/repositories/`, migrations (~222 sql files).
- `integrations` — external API clients (Airbnb, Booking.com, portals).
- `admin-core`, `tenant-ops` — admin + tenant lifecycle logic.

**Security / multi-tenancy hot spots** (touch carefully — see memory & prior IDOR/RLS work):
- Postgres **RLS** with `app.current_organization_id` GUC → `get_current_org_id()` + FORCE RLS
  (migration 00179). By-id queries MUST stay org-keyed; CI superuser pool can bypass FORCE RLS.
- Raw-pool vs RLS-context connection: repos must route through the RLS-context connection
  (`RlsConnection` / `&mut PgConnection` executor), not a raw pool.
  `RlsConnection` is defined in `api-core/src/extractors/rls_connection.rs`.
- `sqlx` offline data: `cargo sqlx prepare` / `.sqlx/` — regenerate on query changes.

**Largest route files** (likely hot / high-churn): `reports.rs` (~120K), `auth.rs` (~107K),
`financial.rs` (~80K), `api_ecosystem.rs` (~74K), `buildings.rs` (~73K), `messaging.rs` (~68K),
`infrastructure.rs` (~68K), `faults.rs` (~66K). (`aml_dsa` is now a route subdir, not a single
file.) Largest repos: `vote.rs` (~80K), `lease.rs` (~77K), `llm_document.rs` (~73K),
`integration.rs` (~73K), `api_ecosystem.rs` (~71K), `dispute.rs` (~67K), `announcement.rs` (~66K),
`board_meetings.rs` (~65K).

## frontend/ (pnpm workspace)

**Apps** (`frontend/apps/`), alphabetized:
- `accounting-web` — React + Vite, accounting console. Backend: accounting-server.
- `admin-web` — React + Vite, super-admin console.
- `mobile` (`@ppt/mobile`) — React Native + Expo. Backend: api-server.
- `ppt-web` (`@ppt/web`) — React 19 + Vite SPA, Property Management. Backend: api-server.
- `reality-web` (`@ppt/reality-web`) — Next.js 16 SSR/ISR public portal. Backend: reality-server.
  i18n in `messages/{cs,de,en,hu,pl,sk}.json`.

**Packages** (`frontend/packages/`):
- `api-client` (`@ppt/api-client`) — **generated** from `docs/api/generated/openapi.yaml`.
- `reality-api-client` — generated (`@hey-api/openapi-ts`) from `docs/api/generated/openapi.yaml`.
- `accounting-api-client` — generated from `docs/api/generated/accounting-openapi.json`.
- `admin-ui`, `dev-panel`, `e2e`, `screen-map`, `shared`, `sitemap`, `ui-kit`, `vite-plugin-ppt-worktree`.

> API clients are generated — change the **TypeSpec** (`docs/api/typespec/`), not the client by hand.

## mobile-native/ (Kotlin Multiplatform)

`shared/` (KMP shared module, Ktor client), `androidApp/` (Jetpack Compose), `iosApp/` (SwiftUI).
Versions in `gradle/libs.versions.toml`. Reality Portal only.

## Cross-cutting

- **Versioning:** `VERSION` file is source of truth; `scripts/update-version.sh` propagates.
- **Tests:** backend `cargo test -p <crate>`; frontend `pnpm test`; mobile `./gradlew test`.
  Pick the smallest scope — see the `ppt-tests` skill.
- **CI gates:** `backend.yml`, `frontend.yml`, `mobile-native.yml`, `api-validation.yml` (see CLAUDE.md).
- **Local stack:** `ppt-dev-stack` skill (`stack up pm-local`).
- **Remote build:** no local Rust/node toolchain in agent envs — offload via `rbuild` (see memory).
