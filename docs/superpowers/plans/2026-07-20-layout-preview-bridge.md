# Layout Preview Bridge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Live iframe preview in the admin layout editor (spec §6.1): the editor embeds the real site, pushes the resolved draft on every edit via an origin-validated postMessage bridge with handshake, the framed page re-renders optimistically, and clicking a section in the iframe highlights it in the editor's tree.

**Architecture:** (1) A stateless backend endpoint `POST /platform-admin/layout/preview-resolve` runs `layout_core::resolve` over a posted draft `ScreenConfig` (against the stored platform manifest + the screen's stored kills) so resolution logic is never duplicated in TS. (2) A tiny protocol module in `@ppt/shared` (`layout-preview.ts`: message types, type guards, origin-checked child/parent helpers — pure TS, no React). (3) Preview mode in both pilot pages: when the URL carries `layoutPreview=1&parentOrigin=<origin>`, the page performs the child handshake and renders the PUSHED `ResolvedScreen` instead of the fetched one; renderers tag each section wrapper with `data-layout-section=<type>` and report clicks to the parent. (4) A `PreviewPanel` in admin-web's `LayoutEditorPage`: preview-URL input (persisted per screen in localStorage), sandboxed iframe, parent handshake, debounced preview-resolve on draft change, config push, click→tree highlight.

**Tech Stack:** Rust/axum (one pure endpoint), TypeScript (`@ppt/shared` no-build module), React in ppt-web/reality-web/admin-web, Vitest.

## Global Constraints

- Spec §6.1: postMessage bridge with origin validation + handshake; `data-*` section tags (we own both sides — no stega); full draft config pushed on every change; draft-mode flag via URL. All previous slices merged on `dev` (#2424–#2429).
- **Branch:** `feature/layout-preview-bridge` from `dev`.
- **Protocol** (in `@ppt/shared/src/layout-preview.ts`, versioned constant `LAYOUT_PREVIEW_PROTOCOL = 1`):
  - child → parent: `{ kind: 'ppt-layout-preview', protocol: 1, type: 'ready', screen?: string }` (on mount and on `pageshow`), `{ …type: 'section-click', sectionType: string }`.
  - parent → child: `{ …type: 'config', resolved: ResolvedScreenLike }`.
  - Child accepts messages ONLY from `parentOrigin` (URL param) and only after it sent `ready`; parent accepts ONLY from the preview URL's origin. Both sides ignore non-matching `kind`/`protocol` silently. Type guards exported (`isPreviewMessage`, etc.). No `'*'` targetOrigin anywhere — always the validated counterpart origin.
- **Preview-resolve endpoint:** `POST /api/v1/platform-admin/layout/preview-resolve` body `{ config: <ScreenConfig JSON>, platform: 'web'|'mobile' }`, super-admin gated like its siblings, sanctioned public connection for manifest+kills reads; 422 `{errors}` when `config` doesn't parse as `ScreenConfig`; 404 `{errors}` when no manifest for the platform; 200 → `ResolvedScreen` JSON from `layout_core::resolve(&config, platform, None, &kills_for(config.screen), &manifest)` (tenant layer None — superadmin previews base config; per-tenant preview is a later slice).
- **Renderer tagging:** ppt-web `LayoutRenderer` and reality-web `LayoutSections` wrap each rendered section (including placeholders) in a `<div data-layout-section={type}>`; in preview mode an `onSectionClick(type)` callback fires from a capture-phase click listener on the wrapper (prevent navigation with `preventDefault` ONLY in preview mode). Tagging is unconditional (harmless data attribute); click reporting only in preview mode.
- **Preview mode must not disturb normal rendering**: pages read `layoutPreview` from the URL once; when absent, zero behavior change (no listeners, no bridge). Preview pushes replace ONLY the layout (`ResolvedScreen`); data/props still come from the page's normal sources.
- iframe sandbox: `sandbox="allow-scripts allow-same-origin allow-forms"` (the framed app needs same-origin for its own API calls); `referrerPolicy="no-referrer"`.
- Debounce preview-resolve calls at 400 ms; a failed resolve shows an inline note in the panel and keeps the last pushed config (never crashes the editor).
- Gates: backend `cargo check -p api-server && cargo clippy -p api-server --all-targets -- -D warnings` (+ RLS script 0 for layout files); frontend `pnpm check && pnpm typecheck` + package suites (`@ppt/shared` if it has tests, `@ppt/web`, `@ppt/reality-web`, `@ppt/admin-web`). Known pre-existing failures (FileDisputePage 3; reality CookieConsentBanner 6 + RealtorManagement 4; admin stale Capabilities assertion + 4 Playwright) untouched. NO jest-dom matchers in admin-web tests; ppt-web/reality-web may use them.
- Commit scopes: `feat(api-server)`, `feat(shared)`, `feat(ppt-web)`, `feat(reality-web)`, `feat(admin-web)`, `docs(...)`. ADAPT rule as before; report adaptations.

## File Structure

```
backend/servers/api-server/src/routes/layout/admin.rs     # + preview_resolve handler
backend/servers/api-server/src/routes/layout/types.rs     # + PreviewResolveRequest
backend/servers/api-server/src/routes/layout/mod.rs       # + route
frontend/packages/shared/src/layout-preview.ts            # protocol module
frontend/packages/shared/src/layout-preview.test.ts       # (if shared has vitest; else test via ppt-web)
frontend/packages/shared/src/index.ts                     # + export
frontend/apps/ppt-web/src/features/layout/LayoutRenderer.tsx        # + data tags + onSectionClick
frontend/apps/ppt-web/src/features/layout/usePreviewLayout.ts       # child-side hook
frontend/apps/ppt-web/src/features/layout/usePreviewLayout.test.tsx
frontend/apps/ppt-web/src/features/dashboard/pages/ManagerDashboardPage.tsx  # preview wiring
frontend/apps/reality-web/src/components/listings/LayoutSections.tsx # + data tags + onSectionClick
frontend/apps/reality-web/src/components/listings/ListingDetailContent.tsx   # preview wiring (client-side)
frontend/apps/reality-web/src/components/listings/useListingPreviewLayout.ts # child-side hook (or shared shape)
frontend/apps/admin-web/src/features/layout-editor/api.ts            # + previewResolve
frontend/apps/admin-web/src/features/layout-editor/PreviewPanel.tsx  # iframe + parent bridge
frontend/apps/admin-web/src/features/layout-editor/PreviewPanel.test.tsx
frontend/apps/admin-web/src/features/layout-editor/LayoutEditorPage.tsx      # mount panel + tree highlight
docs/repo-map.md
```

---

### Task 1: Backend preview-resolve endpoint

**Files:**
- Modify: `backend/servers/api-server/src/routes/layout/types.rs` (+ `PreviewResolveRequest { config: serde_json::Value, platform: String }`)
- Modify: `backend/servers/api-server/src/routes/layout/admin.rs` (+ `preview_resolve` handler)
- Modify: `backend/servers/api-server/src/routes/layout/mod.rs` (`admin_router()`: `.route("/preview-resolve", axum::routing::post(admin::preview_resolve))`)

**Interfaces:**
- Consumes: existing admin.rs idioms (`extract_super_admin_token`, `bad_request`, sanctioned public connection via `db::RlsPool::new(state.db.clone()).acquire_public()`), `LayoutRepository::{get_manifest, list_kills}`, `layout_core::{resolve, Platform, RegistryManifest, ScreenConfig, SectionType}`.
- Produces: handler behavior — 403 unauthenticated; 422 `{errors:[…]}` when `config` fails to parse as `ScreenConfig` or `platform` isn't web/mobile; 404 `{errors}` when no stored manifest for the platform (or stored manifest unparseable → 422 with a "stored manifest invalid" error, mirroring the publish handler); 200 `Json(layout_core::resolve(&config, platform, None, &kills, &manifest))` where kills = stored kill flags for `config.screen`. utoipa annotation like siblings; NOT in ApiDoc.

- [ ] **Step 1:** Implement handler mirroring `publish`'s load/parse/error idioms (same error-shape helpers, same public-connection acquisition); parse `platform` exactly like `resolved.rs`'s `parse_platform`.
- [ ] **Step 2:** Verify FOREGROUND: `cd backend && cargo check -p api-server && cargo clippy -p api-server --all-targets -- -D warnings` clean; `bash backend/scripts/check-rls-enforcement.sh` → 0 violations in layout files.
- [ ] **Step 3:** Commit — `feat(api-server): layout preview-resolve endpoint`

---

### Task 2: `@ppt/shared` protocol module (TDD)

**Files:**
- Create: `frontend/packages/shared/src/layout-preview.ts`
- Modify: `frontend/packages/shared/src/index.ts` (+ export, matching existing style)
- Test: `frontend/packages/shared/src/layout-preview.test.ts` if the package runs Vitest (check its package.json — ADAPT; if it has no test runner, place the tests in `frontend/apps/ppt-web/src/features/layout/layout-preview.shared.test.ts` importing from `@ppt/shared`, and say so).

**Interfaces:**
- Consumes: nothing (pure TS, zero deps, no React).
- Produces (exact names; Tasks 3–5 rely on them):

```ts
export const LAYOUT_PREVIEW_PROTOCOL = 1;
export const LAYOUT_PREVIEW_KIND = 'ppt-layout-preview';

export interface ResolvedSectionLike {
  type: string;
  mode?: string;
  props?: Record<string, unknown>;
  presentation: 'visible' | 'placeholder';
}
export interface ResolvedScreenLike {
  screen: string;
  version: number;
  sections: ResolvedSectionLike[];
}

export type PreviewChildMessage =
  | { kind: typeof LAYOUT_PREVIEW_KIND; protocol: 1; type: 'ready'; screen?: string }
  | { kind: typeof LAYOUT_PREVIEW_KIND; protocol: 1; type: 'section-click'; sectionType: string };
export type PreviewParentMessage =
  { kind: typeof LAYOUT_PREVIEW_KIND; protocol: 1; type: 'config'; resolved: ResolvedScreenLike };

export function isPreviewChildMessage(data: unknown): data is PreviewChildMessage;
export function isPreviewParentMessage(data: unknown): data is PreviewParentMessage;

/** Read preview params from a URL search string. Returns null when not in preview mode
 *  or when parentOrigin is missing/invalid (never half-on). */
export function readPreviewParams(search: string): { parentOrigin: string } | null;

/** Child side: validates origin against parentOrigin, sends `ready`, invokes
 *  onConfig for valid config messages. Returns { sendSectionClick, dispose }. */
export function connectPreviewChild(opts: {
  parentOrigin: string;
  screen?: string;
  onConfig: (resolved: ResolvedScreenLike) => void;
  win?: Window;             // default window (injectable for tests)
}): { sendSectionClick: (sectionType: string) => void; dispose: () => void };

/** Parent side: listens for child messages from childOrigin, invokes onReady/onSectionClick,
 *  returns { sendConfig, dispose }. sendConfig posts to the child window with the exact
 *  childOrigin as targetOrigin (never '*'). */
export function connectPreviewParent(opts: {
  childWindow: () => Window | null;   // lazy: iframe.contentWindow
  childOrigin: string;
  onReady: () => void;
  onSectionClick: (sectionType: string) => void;
  win?: Window;
}): { sendConfig: (resolved: ResolvedScreenLike) => void; dispose: () => void };
```

Implementation notes: guards check `kind`, `protocol`, and `type` shape strictly; `readPreviewParams` requires `layoutPreview=1` AND a `parentOrigin` that parses via `new URL(…).origin` round-trip; child ignores messages before its own `ready` was sent (order enforced by construction — listener registered, then ready sent); both helpers use `win.addEventListener('message', …)` and `dispose()` removes it.

- [ ] **Step 1: failing tests** — guards accept/reject shapes; `readPreviewParams` (absent flag → null; missing/garbage parentOrigin → null; valid → origin); child: delivers config only from parentOrigin (spoofed origin ignored), sends ready on connect, `sendSectionClick` posts to parentOrigin; parent: delivers ready/section-click only from childOrigin, `sendConfig` posts with childOrigin targetOrigin. Use two mock window objects (simple event-emitter stubs) — no jsdom iframes needed.
- [ ] **Step 2: implement + pass; Biome clean.**
- [ ] **Step 3: Commit** — `feat(shared): layout preview bridge protocol`

---

### Task 3: ppt-web child side (TDD)

**Files:**
- Modify: `frontend/apps/ppt-web/src/features/layout/LayoutRenderer.tsx` (wrap every rendered row — component AND placeholder — in `<div data-layout-section={type} onClickCapture={preview only}>`; new optional prop `onSectionClick?: (type: string) => void`)
- Create: `frontend/apps/ppt-web/src/features/layout/usePreviewLayout.ts`
- Modify: `frontend/apps/ppt-web/src/features/dashboard/pages/ManagerDashboardPage.tsx`
- Test: `frontend/apps/ppt-web/src/features/layout/usePreviewLayout.test.tsx` (+ extend LayoutRenderer tests for tagging/click)

**Interfaces:**
- Consumes: Task 2 module from `@ppt/shared`.
- Produces: `usePreviewLayout(screen: string): { previewLayout: ResolvedScreenLike | null; inPreview: boolean; sendSectionClick: (t: string) => void }` — reads `readPreviewParams(window.location.search)` once (useState initializer); when active, `connectPreviewChild` in an effect (dispose on unmount), storing pushed configs in state. Page usage:

```tsx
const { previewLayout, inPreview, sendSectionClick } = usePreviewLayout('ppt/dashboard');
const { data: layout } = useResolvedLayout('ppt/dashboard'); // unchanged
<LayoutRenderer
  layout={(previewLayout as ResolvedScreen | null) ?? layout ?? DEFAULT_DASHBOARD_LAYOUT}
  registry={dashboardRegistry}
  onSectionClick={inPreview ? sendSectionClick : undefined}
/>
```

`LayoutRenderer` click behavior: when `onSectionClick` is set, the wrapper's capture-phase click calls `e.preventDefault(); e.stopPropagation(); onSectionClick(type)`. When unset: no listener, no behavior change. `data-layout-section` is always present.

- [ ] **Step 1: failing tests** — (renderer) rows carry `data-layout-section`; with `onSectionClick`, clicking inside a section fires the callback with the type and prevents inner interaction; without it, clicks pass through (existing tests stay green). (hook) with a preview-param location + stubbed `connectPreviewChild` (vi.mock the shared module): returns pushed config after `onConfig` fires; `inPreview=false` and no connect call when params absent.
- [ ] **Step 2: implement + pass** — `pnpm -F @ppt/web test -- layout` + full suite (pre-existing failures only).
- [ ] **Step 3: Commit** — `feat(ppt-web): layout preview mode on the manager dashboard`

---

### Task 4: reality-web child side (TDD)

**Files:**
- Modify: `frontend/apps/reality-web/src/components/listings/LayoutSections.tsx` (same tagging + optional `onSectionClick` as Task 3's renderer — mirror it)
- Create: `frontend/apps/reality-web/src/components/listings/useListingPreviewLayout.ts` (same hook shape as ppt-web's, `'use client'`-safe: guard `typeof window !== 'undefined'`, read search from `window.location`)
- Modify: `frontend/apps/reality-web/src/components/listings/ListingDetailContent.tsx` (call the hook with `'reality/listing-detail'`; effective layout = `previewLayout ?? layout` prop; pass `onSectionClick` down; the sidebar agent-contact lookup uses the same effective layout)
- Test: extend `LayoutSections.test.tsx` + `useListingPreviewLayout.test.tsx`

**Interfaces:** mirror Task 3 exactly (the hook may be a near-copy — small enough that duplication beats a premature shared-react package; note it).

- [ ] **Step 1: failing tests** (tagging/click on LayoutSections; hook behavior with mocked shared module; ListingDetailContent uses pushed layout when in preview — extend its test with one case).
- [ ] **Step 2: implement + pass** — `pnpm -F @ppt/reality-web test` (pre-existing failures only).
- [ ] **Step 3: Commit** — `feat(reality-web): layout preview mode on listing detail`

---

### Task 5: admin-web PreviewPanel (TDD)

**Files:**
- Modify: `frontend/apps/admin-web/src/features/layout-editor/api.ts` (+ `previewResolve(token, config: ScreenConfig, platform): Promise<ResolvedScreenLike>` via the existing `request` helper, POST `/preview-resolve`, errors → `LayoutApiError`)
- Create: `frontend/apps/admin-web/src/features/layout-editor/PreviewPanel.tsx`
- Modify: `frontend/apps/admin-web/src/features/layout-editor/LayoutEditorPage.tsx` (mount `<PreviewPanel screen platform draft={localDraft} token …/>` in its own Card; wire `onSectionSelected` → highlighted section state passed to `SectionTreeEditor` as a new optional `highlightType?: string` prop that applies a highlight class/outline to the matching row)
- Modify: `frontend/apps/admin-web/src/features/layout-editor/SectionTreeEditor.tsx` (accept + render `highlightType` — outline style + `scrollIntoView` on change; no behavior change otherwise)
- Test: `PreviewPanel.test.tsx` (+ extend page/tree tests minimally)

**Interfaces:**
- Consumes: Tasks 1–2 (`previewResolve`, `connectPreviewParent` from `@ppt/shared`), existing editor state.
- Produces `PreviewPanel` behavior:
  - Props: `{ screen: string; platform: 'web'|'mobile'; draft: ScreenConfig; token: string | null; onSectionSelected(type: string): void }`.
  - URL input (persisted `localStorage['ppt.layoutPreview.url.' + screen]`); Load button sets the iframe `src` to the URL with `layoutPreview=1&parentOrigin=<window.location.origin>` appended (preserving existing query); invalid URL → inline error.
  - On iframe load + child `ready` (via `connectPreviewParent` with `childOrigin = new URL(previewUrl).origin`): panel state "connected"; every `draft`/`platform` change (and on ready) triggers a 400 ms-debounced `previewResolve`; success → `sendConfig(resolved)`; failure → inline note (keep last pushed).
  - `section-click` → `onSectionSelected(type)`.
  - Dispose the bridge on unmount/url change. Sandbox + referrerPolicy per Global Constraints. All strings `t('admin.layout.preview.…', { defaultValue })`.
- [ ] **Step 1: failing tests** — mock `./api` and `@ppt/shared`'s `connectPreviewParent`: (1) Load builds the iframe src with appended params, preserving the URL's own query; (2) on ready, previewResolve called with current draft and `sendConfig` receives its result; (3) draft change → debounced re-resolve (use fake timers); (4) resolve failure → inline note, no crash; (5) section-click invokes `onSectionSelected`; (6) invalid URL → inline error, no iframe.
- [ ] **Step 2: implement + pass** — `pnpm -F @ppt/admin-web test -- layout-editor` + full suite; NO jest-dom matchers; typecheck stays at 0 admin-web errors.
- [ ] **Step 3: Commit** — `feat(admin-web): live preview panel in the layout editor`

---

### Task 6: Gates + docs

- Modify: `docs/repo-map.md` (layout bullet: `Preview bridge: @ppt/shared layout-preview + admin-web PreviewPanel; preview-resolve endpoint`), screen-map Agent Log lines for ppt dashboard + reality listing-detail (`2026-07-20 — agent: layout preview mode (postMessage bridge) added.`).
- Gates FOREGROUND: backend check/clippy + RLS script; `cd frontend && pnpm check && pnpm typecheck` + the four package test suites. Known pre-existing failures only; `check:fix` our files.
- Commit — `docs(repo-map): layout preview bridge pointers`

---

## Deliberate scope decisions (do not "fix" during implementation)

- **Superadmin preview only, base config, tenant layer None** — per-tenant preview joins the per-tenant admin view slice.
- **Click-to-select is one-way (iframe → tree)**; hover sync and tree→iframe scroll are polish for later.
- **The framed ppt-web page requires its own login session** — acceptable; the panel shows whatever the iframe shows (login screen if unauthenticated). No token passing into the iframe (deliberate — never leak the admin JWT to the framed origin).
- **Preview pushes bypass the server draft** — the editor previews unsaved local drafts via preview-resolve; Save Draft/Publish flows unchanged.
- **No E2E**; the bridge is unit-tested with injected window stubs.

## Out of scope (subsequent plans)

1. Per-tenant preview + platform-override editing UX.
2. Mobile registries/renderers; publish→ISR webhook; `layout_editor_*` capability.
