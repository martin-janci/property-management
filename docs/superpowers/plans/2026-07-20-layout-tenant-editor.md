# Layout Tenant Editor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Org-admin self-service layout customization in ppt-web (spec §3.4 two-role model, tenant side): a "Customize dashboard" page where OrgAdmin+ users toggle/reorder/re-mode sections and edit whitelisted props for their org's `ppt/dashboard` — strictly within the superadmin-authored rails — saving through the rails-validated `PUT /api/v1/layout/tenant-override`.

**Architecture:** One small backend addition (the tenant-override GET envelope gains the platform manifest, so the tenant UI knows each section's supported modes), an api-client extension (tenant-override fetchers carrying `Authorization` + `X-Tenant-ID`, with a 422-errors-preserving error type), a pure controlled `TenantSectionEditor` in ppt-web (controls appear ONLY where rails allow: eye toggle iff hideable, ↑/↓ iff reorderable, mode select iff mode-editable, per-prop inputs iff whitelisted — everything else renders as read-only rows), and a `DashboardCustomizePage` (seed-on-load, dirty flag, Save with verbatim 422 list, reset-to-default) routed at `/dashboard/customize` behind `requiredRoles={['org_admin','super_admin']}` with a role-gated "Customize" link on the manager dashboard.

**Tech Stack:** Rust/axum (one handler touch), TypeScript, TanStack Query 5, react-i18next (6 locales), Vitest + Testing Library.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-07-19-layout-content-manager-design.md` §3.4 (tenant powers: visibility + order + modes + whitelisted props), §6 (tenant editor = capability-restricted variant), §6.3 (server-side rails gate is the truth — the UI hides out-of-rails controls, it does not enforce). All prior slices are merged on `dev` (#2424, #2426, #2427, #2428).
- **Branch:** `feature/layout-tenant-editor` from `dev`.
- Contract shapes (backend, merged): GET `/api/v1/layout/tenant-override?screen=` → `{ override: Row|null, rails: Rails|{}, published: ScreenConfig|null }` where `Row.override_config` holds the `TenantOverride`; PUT body `{ screen, override_config }` with `TenantOverride { order?: string[]; sections?: Record<string, { visible?: boolean; mode?: string; props?: Record<string, unknown> }> }`; 422 → `{ errors: string[] }` (rails violations, rendered VERBATIM); 403 when role below OrgAdmin; 404 when screen unknown/unpublished. Rails: `{ hideable: string[], mode_editable: string[], reorderable: boolean, prop_whitelist: Record<string, string[]> }`.
- **Backend envelope extension (Task 1):** GET tenant-override additionally returns `"manifest": <web manifest JSON or null>` — read via `LayoutRepository::get_manifest(&…, "web")` on the same sanctioned public connection pattern already used in that file for global tables. Additive only; existing keys unchanged.
- Tenant calls MUST send `X-Tenant-ID` — `authenticatedFetchJson` adds only `Authorization`; mirror `packages/api-client/src/financial/api.ts` (`getToken()` + `getOrg()` from `../auth/…`, omit the header when org is null).
- UI hides controls the rails don't grant — but NEVER renders disabled controls (house rule from the admin editor); a section with no granted controls renders as a plain read-only row. Sections listed in `published` are the universe; override may only reference them (server enforces `NotInBase`).
- Whitelisted prop values: input text is parsed as JSON when `JSON.parse` succeeds, otherwise stored as a string (documented in a code comment + i18n hint). Empty input removes the prop from the override.
- Effective display state = published base + override (visible/mode per section) so the editor shows what the org will get; base values render as the fallback when the override doesn't touch a field.
- Reset-to-default = save an empty override `{}` (server keeps the row; resolver then applies no tenant delta).
- Seed discipline: page seeds local override state from the envelope once per load (single fixed screen `ppt/dashboard`; no screen switching in this page — simpler than the admin editor; refetch after save must NOT clobber unsaved edits → same conditional dirty-clear via sent-JSON ref as the admin editor).
- i18n: new keys under `layout.customize.*` added to ALL SIX locale files (`en,sk,cs,de,pl,hu`), `t('layout.customize.…')` (ppt-web has no defaultValue convention — keys must exist; keep en as source and translate the rest sensibly).
- Gates: backend `cargo check -p api-server && cargo clippy -p api-server --all-targets -- -D warnings`; frontend `pnpm -F @ppt/api-client test`, `pnpm -F @ppt/web test`, workspace `pnpm check && pnpm typecheck` (known pre-existing failures: ppt-web FileDisputePage 3; do not touch). NO jest-dom matchers in any admin-web files (n/a here — ppt-web's setup DOES provide jest-dom, mirror its existing tests).
- Commit scopes: `feat(api-server)`, `feat(api-client)`, `feat(ppt-web)`, `docs(...)`. ADAPT rule as before (mirror files named per task; logic/contracts fixed; report adaptations).

## File Structure

```
backend/servers/api-server/src/routes/layout/tenant.rs   # + manifest in GET envelope
frontend/packages/api-client/src/layout/api.ts           # + tenant-override types/fetchers
frontend/packages/api-client/src/layout/hooks.ts         # + useTenantLayout/useSaveTenantLayoutOverride
frontend/packages/api-client/src/layout/api.test.ts      # + tests
frontend/apps/ppt-web/src/features/layout/TenantSectionEditor.tsx
frontend/apps/ppt-web/src/features/layout/TenantSectionEditor.test.tsx
frontend/apps/ppt-web/src/features/layout/DashboardCustomizePage.tsx
frontend/apps/ppt-web/src/features/layout/DashboardCustomizePage.test.tsx
frontend/apps/ppt-web/src/routes/groups/core.tsx         # + lazy route /dashboard/customize
frontend/apps/ppt-web/src/features/dashboard/pages/ManagerDashboardPage.tsx  # + role-gated Customize link
frontend/apps/ppt-web/messages/{en,sk,cs,de,pl,hu}.json  # + layout.customize.*
docs/repo-map.md                                          # extend layout bullet
```

---

### Task 1: Backend — manifest in the tenant-override envelope

**Files:**
- Modify: `backend/servers/api-server/src/routes/layout/tenant.rs` (handler `get_tenant_override`)

**Interfaces:**
- Consumes: existing handler + `LayoutRepository::get_manifest` + the file's existing sanctioned-public-connection idiom for global tables (see how `get_config` is fetched in the same handler — mirror it).
- Produces: response JSON gains `"manifest": <manifest JSON | null>` (the `web` platform manifest's `manifest` column, or null when none uploaded). Existing keys (`override`, `rails`, `published`) unchanged. utoipa description updated.

- [ ] **Step 1:** In `get_tenant_override`, after loading the config row, fetch `repo.get_manifest(<same public-conn executor as get_config>, "web")` (propagate DB errors like the neighbouring calls), and extend the `serde_json::json!({...})` with `"manifest": manifest_row.map(|r| r.manifest)`.
- [ ] **Step 2:** Update the `#[utoipa::path]` response description to mention the manifest field.
- [ ] **Step 3:** Verify FOREGROUND: `cd backend && cargo check -p api-server && cargo clippy -p api-server --all-targets -- -D warnings` — clean.
- [ ] **Step 4:** Commit — `feat(api-server): include web manifest in tenant layout envelope`

---

### Task 2: api-client — tenant-override domain (TDD)

**Files:**
- Modify: `frontend/packages/api-client/src/layout/api.ts`
- Modify: `frontend/packages/api-client/src/layout/hooks.ts`
- Test: `frontend/packages/api-client/src/layout/api.test.ts` (extend)

**Interfaces:**
- Consumes: `getToken()` + `getOrg()` (mirror `financial/api.ts` for exact import paths and header idiom).
- Produces (Tasks 3–4 rely on exact names): types `TenantSectionPatch { visible?: boolean; mode?: string; props?: Record<string, unknown> }`, `TenantOverride { order?: string[]; sections?: Record<string, TenantSectionPatch> }`, `LayoutRails { hideable: string[]; mode_editable: string[]; reorderable: boolean; prop_whitelist: Record<string, string[]> }`, `ManifestComponent { required?: boolean; supported_modes?: string[]; default_mode?: string }`, `LayoutManifest { platform: string; components: Record<string, ManifestComponent> }`, `BaseSection { type: string; visible?: boolean; mode?: string; props?: Record<string, unknown> }`, `TenantLayoutEnvelope { override: { override_config: TenantOverride } | null; rails: LayoutRails | Record<string, never>; published: { sections: BaseSection[] } | null; manifest: LayoutManifest | null }`; error `TenantLayoutError extends Error { status: number; errors: string[] }`; `fetchTenantLayout(screen): Promise<TenantLayoutEnvelope>`; `saveTenantLayoutOverride(screen, override: TenantOverride): Promise<unknown>` (throws `TenantLayoutError` carrying the 422 `errors` verbatim); hooks `useTenantLayout(screen)` and `useSaveTenantLayoutOverride(screen)` (`useMutation` invalidating the tenant-layout key on success); key factory extension `layoutKeys.tenant(screen)`.
- Implementation note: use a local `tenantRequest` helper doing raw `fetch('/api/v1/layout/tenant-override…')` with `Authorization` (getToken) + `X-Tenant-ID` (getOrg, omitted when null) + `Content-Type`, parsing `{errors}` on non-2xx into `TenantLayoutError` — do NOT use `authenticatedFetchJson` (no org header, error body lost).

- [ ] **Step 1: failing tests** — (a) `fetchTenantLayout` GETs `/api/v1/layout/tenant-override?screen=ppt%2Fdashboard` with both headers and returns the envelope; (b) omits `X-Tenant-ID` when `getOrg()` is null (mock the auth module with `vi.mock`); (c) `saveTenantLayoutOverride` PUTs `{ screen, override_config }`; (d) 422 → `TenantLayoutError` with `errors` verbatim.
- [ ] **Step 2: implement; run** `pnpm -F @ppt/api-client test` — all green (existing 131+ too).
- [ ] **Step 3: Commit** — `feat(api-client): tenant layout override fetchers and hooks`

---

### Task 3: TenantSectionEditor (pure component, TDD)

**Files:**
- Create: `frontend/apps/ppt-web/src/features/layout/TenantSectionEditor.tsx`
- Test: `frontend/apps/ppt-web/src/features/layout/TenantSectionEditor.test.tsx`

**Interfaces:**
- Consumes: Task 2 types (from `@ppt/api-client`).
- Produces: controlled `props { baseSections: BaseSection[]; rails: LayoutRails; manifest: LayoutManifest | null; override: TenantOverride; onChange(next: TenantOverride): void }`. Rendering/behavior contract:
  - Rows = published base sections, ordered by `override.order` when present (listed first, unlisted keep base order — same semantics as the resolver), else base order.
  - Effective visible = `override.sections[type].visible ?? base.visible ?? true`; effective mode = `override.sections[type].mode ?? base.mode`.
  - Eye toggle rendered ONLY when `type ∈ rails.hideable`; toggling patches `override.sections[type].visible` (setting it equal to the base value still records the explicit patch — simplest; server accepts it).
  - ↑/↓ rendered ONLY when `rails.reorderable`; reorder writes the FULL current order array to `override.order`.
  - Mode `<select>` ONLY when `type ∈ rails.mode_editable` AND the manifest lists `supported_modes` for it; options = supported modes; first option = effective mode.
  - Per-prop inputs ONLY for names in `rails.prop_whitelist[type]`: one labeled text input per name; value shown = `JSON.stringify(override.sections[type].props?.[name])` when set (strings unquoted), else empty with the base value as placeholder; commit on blur — parse as JSON if possible else string; empty removes the prop (and drops empty `props`/section objects).
  - Sections with no granted controls render read-only (type + effective state badges). NO disabled controls anywhere.
  - A hidden-effective section row gets a dimmed style + "hidden" badge.
- All strings via `t('layout.customize.…')`.

- [ ] **Step 1: failing tests** (mock react-i18next key-echo): (1) hideable-only section shows the eye and no arrows/mode/props; toggling calls `onChange` with the visible patch; (2) non-granted section renders no interactive controls; (3) reorderable rails render arrows; ↓ writes full `order` array; (4) mode select renders only for mode-editable + manifest modes; change patches mode; (5) whitelisted prop input commits JSON (`5` → number 5) and plain text as string; clearing removes the prop; (6) `override.order` drives row order.
- [ ] **Step 2: implement + pass** (`pnpm -F @ppt/web test -- TenantSectionEditor`), Biome clean. Follow ppt-web test conventions (jest-dom IS available here).
- [ ] **Step 3: Commit** — `feat(ppt-web): tenant section editor component`

---

### Task 4: DashboardCustomizePage + route + link (TDD)

**Files:**
- Create: `frontend/apps/ppt-web/src/features/layout/DashboardCustomizePage.tsx`
- Modify: `frontend/apps/ppt-web/src/routes/groups/core.tsx` (lazy route `/dashboard/customize` wrapped in `<ProtectedRoute requiredRoles={['org_admin', 'super_admin']}>` — mirror the `SessionsPage` lazy idiom)
- Modify: `frontend/apps/ppt-web/src/features/dashboard/pages/ManagerDashboardPage.tsx` (a "Customize" `<Link to="/dashboard/customize">` in the header, rendered only when `useAuth().user?.role` is `'org_admin' | 'super_admin'`)
- Modify: `frontend/apps/ppt-web/messages/{en,sk,cs,de,pl,hu}.json` (`layout.customize.*` keys used by Tasks 3–4)
- Test: `frontend/apps/ppt-web/src/features/layout/DashboardCustomizePage.test.tsx` (+ extend `ManagerDashboardPage.test.tsx` for the role-gated link)

**Interfaces:**
- Consumes: `useTenantLayout('ppt/dashboard')`, `useSaveTenantLayoutOverride`, `TenantSectionEditor`, `useToast` (`src/components`), `useAuth` (`src/contexts/AuthContext`).
- Produces behavior:
  - Loading → spinner; envelope with `published: null` → info panel `layout.customize.notPublished` (no editor); error → error panel + retry.
  - Editor seeded from `envelope.override?.override_config ?? {}` once per load; dirty flag on change; Save button (disabled when clean) → mutation; success toast + conditional dirty-clear via sent-JSON ref (admin-editor discipline); `TenantLayoutError` 422 → persistent verbatim `<ul>` of `errors` (role=alert), cleared on next change.
  - Reset-to-default button (confirm) → sets local override `{}` + marks dirty (user still saves explicitly).
  - Back link to `/dashboard/manager`.

- [ ] **Step 1: failing tests** — (1) renders editor rows from the envelope's published sections (mock `@ppt/api-client` layout hooks/fetchers via `vi.mock`); (2) not-published envelope shows the info panel and no editor; (3) toggle → Save calls `saveTenantLayoutOverride('ppt/dashboard', patch)`; (4) 422 errors render verbatim; (5) ManagerDashboardPage shows the Customize link for `org_admin` and hides it for `resident` (mock `useAuth`).
- [ ] **Step 2: implement + route + link + i18n keys (all six locales); run** `pnpm -F @ppt/web test` full — only the 3 pre-existing FileDisputePage failures remain.
- [ ] **Step 3: Commit** — `feat(ppt-web): dashboard customize page for org admins`

---

### Task 5: Gates + docs

**Files:**
- Modify: `docs/repo-map.md` (layout bullet: `Tenant editor: ppt-web /dashboard/customize (features/layout/)`)
- Modify: `docs/screens/ppt/dashboard.md` (Agent Log entry: customize entry point added) — plus a new Agent Log line only in docs that exist; note absences.
- Test: full gates.

- [ ] **Step 1:** repo-map + screen-map entries.
- [ ] **Step 2:** `cd backend && cargo check -p api-server && cargo clippy -p api-server --all-targets -- -D warnings`; `cd frontend && pnpm check && pnpm typecheck && pnpm -F @ppt/api-client test && pnpm -F @ppt/web test`. All green modulo known pre-existing failures; `check:fix` only our files.
- [ ] **Step 3:** Commit — `docs(repo-map): layout tenant editor pointers`

---

## Deliberate scope decisions (do not "fix" during implementation)

- **Single screen (`ppt/dashboard`)** — a screen picker joins when more ppt screens enter the layout system.
- **No mobile-platform tenant editing** — envelope carries the web manifest only (Task 1); mobile joins with the mobile slice.
- **Explicit-save only** — no autosave/optimistic preview; the dashboard reflects changes on next visit (resolved endpoint is cached per org).
- **Rails UI truthfully mirrors the server gate but does not duplicate its validation** — out-of-rails states are unreachable in the UI; the 422 path still renders verbatim as the safety net.
- **No preview bridge** — later slice.

## Out of scope (subsequent plans)

1. Preview bridge (admin + tenant), platform-override editing, per-tenant view in admin-web.
2. Mobile registries/renderers; publish webhook; `layout_editor_*` capability.
