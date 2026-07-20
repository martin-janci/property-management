# Layout Editor MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Superadmin layout editor in admin-web — screen list, per-screen section tree editing (visibility, order, modes, props), rails authoring, publish with validation-error surfacing, version history + rollback, kill-switch, and registry-manifest management — driving the existing `/api/v1/platform-admin/layout/*` endpoints. No iframe preview (that's the next slice).

**Architecture:** One admin-web feature module `src/features/layout-editor/` with a raw-fetch API layer (feature-flags idiom: bearer from `useAdminAuth`, TanStack Query for reads, `useMutation` for writes), a pure controlled `SectionTreeEditor` component (all editing is local state on a draft `ScreenConfig`; explicit Save Draft), a `RailsEditor` (structured per-section checkboxes + whitelist text), and two pages (`LayoutEditorPage`, `LayoutManifestsPage`) wired into `App.tsx` + `AdminLayout` nav behind `ProtectedRoute` (platform-principal gate; no new capability in MVP — endpoints enforce super-admin JWT server-side). Props and manifest JSON are edited via validated `<textarea>` (repo has no JSON-editor dependency; do not add one).

**Tech Stack:** React 19 + react-router-dom 6, TanStack Query 5, react-i18next (en/sk/cs with `defaultValue` fallbacks), `@ppt/ui-kit` (`Button`, `Badge`, `Card`, `Spinner`), local Toast system, Vitest + Testing Library.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-07-19-layout-content-manager-design.md` §5, §6 (editor minus preview bridge/§6.1), §7. Backend endpoints are merged on `dev` (#2426); frontend registries + checked-in manifests merged (#2427).
- **Branch:** `feature/layout-editor-mvp` from `dev`.
- **Contract shapes** (must match backend exactly): `ScreenConfig { screen, version, sections: [{ type, visible?, mode?, props?, overrides? }] }`; rails `{ hideable: string[], mode_editable: string[], reorderable: bool, prop_whitelist: Record<string,string[]> }`; manifest `{ platform: 'web'|'mobile', components: Record<string, { required?, supported_modes?, default_mode? }> }`; config row from GET /config: `{ config: { …row, draft, published, published_version, rails }, versions: [{ version, published_at, published_by }], kills: [{ section_type, killed_at }] }`. Endpoints: GET `/api/v1/platform-admin/layout/screens`, GET `…/config?screen=`, PUT `…/draft` `{screen, config}`, PUT `…/rails` `{screen, rails}`, POST `…/publish` `{screen}` (422 → `{errors: string[]}` — MUST be surfaced verbatim in the UI), POST `…/rollback` `{screen, version}`, POST `…/kill`/`…/unkill` `{screen, section_type}`, GET/PUT `…/manifests` (PUT `{platform, manifest}`).
- **Required sections** (per the selected platform manifest) get a lock badge and NO hide control (spec §4.2 / Gutenberg lesson — no disabled buttons); they can still be reordered. Sections not in the manifest render an "unknown type" warning badge.
- Editing is draft-only and explicit: local edits → "Save draft" (PUT /draft). Publish is a separate button; a 422 response renders the error list verbatim in an Alert — never a silent failure. Kill/unkill act immediately (they bypass drafts by design, spec §5) and must be visually distinct (danger styling + confirm).
- Fetch idiom: mirror `src/pages/feature-flags.tsx` (raw fetch, `Authorization: Bearer` when token, `credentials: 'include'`, `queryKey: ['admin','platform','layout',…]`, 404-on-list → empty, error toasts via local `useToast`). Mutations via `useMutation` (CapabilitiesAdminPage idiom) with query invalidation.
- i18n: keys under `admin.layout.*` in `messages/{en,sk,cs}.json`, always with inline `defaultValue` in `t()` calls (house style — pages must render without key coverage).
- Reorder controls are buttons (↑/↓) — no drag-and-drop dependency in MVP (a11y baseline per spec §5.2 of the design's editor section).
- Tests: Vitest co-located, Testing-Library, mock fetch via `vi.stubGlobal('fetch', …)` or msw (mirror `CapabilitiesAdminPage.test.tsx` wrapper stack: MemoryRouter + QueryClientProvider + AdminAuthProvider (+ mocked `@ppt/admin-ui` if needed)).
- Gates: `cd frontend && pnpm -F @ppt/admin-web test` (ADAPT: check the actual package name in `frontend/apps/admin-web/package.json`) + `pnpm check` + `pnpm typecheck`. Known pre-existing failures elsewhere in the workspace (FileDisputePage, CookieConsentBanner, RealtorManagement, dev-panel) must not be touched.
- Commit scope: `feat(admin-web)` / `docs(...)`. ADAPT rule as in prior plans (import paths/helper names to the named mirror files; logic and contracts fixed). Report adaptations.
- New-screen creation: the editor allows typing a new screen id (validated `^[a-z0-9-]+/[a-z0-9-]+$`) — PUT /draft upserts it server-side; seed its draft as `{ screen, version: 0, sections: [] }`.

## File Structure

```
frontend/apps/admin-web/src/features/layout-editor/
├── api.ts                    # types + fetch functions (token param)
├── api.test.ts
├── SectionTreeEditor.tsx     # pure controlled section-list editor
├── SectionTreeEditor.test.tsx
├── RailsEditor.tsx           # pure controlled rails editor
├── RailsEditor.test.tsx
├── LayoutEditorPage.tsx      # queries + mutations + composition
├── LayoutEditorPage.test.tsx
├── LayoutManifestsPage.tsx   # manifests list + upload
└── LayoutManifestsPage.test.tsx
frontend/apps/admin-web/src/App.tsx              # + 2 routes
frontend/apps/admin-web/src/components/AdminLayout.tsx  # + nav entries
frontend/apps/admin-web/messages/{en,sk,cs}.json # + admin.layout.* keys
docs/repo-map.md                                  # extend layout bullet
```

---

### Task 1: API layer

**Files:**
- Create: `frontend/apps/admin-web/src/features/layout-editor/api.ts`
- Test: `frontend/apps/admin-web/src/features/layout-editor/api.test.ts`

**Interfaces:**
- Consumes: nothing app-side (token passed in as a parameter — pages own the `useAdminAuth()` call).
- Produces (Tasks 2–5 rely on these exact names): types `SectionConfig { type: string; visible?: boolean; mode?: string; props?: Record<string, unknown>; overrides?: Record<string, unknown> }`, `ScreenConfig { screen: string; version: number; sections: SectionConfig[] }`, `Rails { hideable: string[]; mode_editable: string[]; reorderable: boolean; prop_whitelist: Record<string, string[]> }`, `ManifestComponent { required?: boolean; supported_modes?: string[]; default_mode?: string }`, `Manifest { platform: 'web' | 'mobile'; components: Record<string, ManifestComponent> }`, `ScreenSummary` (row of GET /screens), `ConfigEnvelope { config: ScreenRow; versions: VersionRow[]; kills: KillRow[] }` with `ScreenRow { screen: string; draft: ScreenConfig; published: ScreenConfig | null; published_version: number; rails: Rails | Record<string, never> }`, `VersionRow { version: number; published_at: string; published_by: string | null }`, `KillRow { section_type: string; killed_at: string }`; error class `LayoutApiError extends Error { status: number; errors: string[] }`; functions `listScreens(token)`, `getConfig(token, screen)`, `putDraft(token, screen, config)`, `putRails(token, screen, rails)`, `publish(token, screen)`, `rollback(token, screen, version)`, `kill(token, screen, sectionType)`, `unkill(token, screen, sectionType)`, `listManifests(token)`, `putManifest(token, platform, manifest)`.

- [ ] **Step 1: failing tests** — `api.test.ts` with `vi.stubGlobal('fetch', …)`:

```ts
import { afterEach, describe, expect, it, vi } from 'vitest';
import { LayoutApiError, listScreens, publish, putDraft } from './api';

function okJson(body: unknown) {
  return new Response(JSON.stringify(body), { status: 200 });
}

describe('layout-editor api', () => {
  afterEach(() => vi.unstubAllGlobals());

  it('listScreens sends bearer and returns rows; 404 → empty list', async () => {
    const fetchMock = vi.fn().mockResolvedValue(okJson([{ screen: 'ppt/dashboard' }]));
    vi.stubGlobal('fetch', fetchMock);
    const rows = await listScreens('tok');
    expect(rows).toEqual([{ screen: 'ppt/dashboard' }]);
    const [url, init] = fetchMock.mock.calls[0];
    expect(String(url)).toBe('/api/v1/platform-admin/layout/screens');
    expect((init.headers as Record<string, string>).Authorization).toBe('Bearer tok');
    expect(init.credentials).toBe('include');

    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response('', { status: 404 })));
    await expect(listScreens('tok')).resolves.toEqual([]);
  });

  it('putDraft PUTs the exact body shape', async () => {
    const fetchMock = vi.fn().mockResolvedValue(okJson({}));
    vi.stubGlobal('fetch', fetchMock);
    const config = { screen: 'ppt/dashboard', version: 0, sections: [] };
    await putDraft('tok', 'ppt/dashboard', config);
    const [url, init] = fetchMock.mock.calls[0];
    expect(String(url)).toBe('/api/v1/platform-admin/layout/draft');
    expect(init.method).toBe('PUT');
    expect(JSON.parse(init.body as string)).toEqual({ screen: 'ppt/dashboard', config });
  });

  it('publish surfaces 422 errors as LayoutApiError.errors', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ errors: ['required section gallery.v1 is hidden'] }), {
        status: 422,
      }),
    ));
    const err = await publish('tok', 'ppt/dashboard').catch((e: unknown) => e);
    expect(err).toBeInstanceOf(LayoutApiError);
    expect((err as LayoutApiError).status).toBe(422);
    expect((err as LayoutApiError).errors).toEqual(['required section gallery.v1 is hidden']);
  });
});
```

- [ ] **Step 2: implement `api.ts`** — the types above plus:

```ts
const BASE = '/api/v1/platform-admin/layout';

export class LayoutApiError extends Error {
  status: number;
  errors: string[];
  constructor(status: number, errors: string[]) {
    super(errors.join('; ') || `HTTP ${status}`);
    this.status = status;
    this.errors = errors;
  }
}

function headers(token: string | null): Record<string, string> {
  const h: Record<string, string> = { 'Content-Type': 'application/json' };
  if (token) h.Authorization = `Bearer ${token}`;
  return h;
}

async function request<T>(
  token: string | null,
  path: string,
  init: RequestInit = {},
  opts: { emptyOn404?: T } = {},
): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    ...init,
    headers: { ...headers(token), ...(init.headers as Record<string, string> | undefined) },
    credentials: 'include',
  });
  if (res.status === 404 && opts.emptyOn404 !== undefined) return opts.emptyOn404;
  if (!res.ok) {
    const body = await res.json().catch(() => ({}));
    const errors = Array.isArray((body as { errors?: unknown }).errors)
      ? ((body as { errors: string[] }).errors)
      : [];
    throw new LayoutApiError(res.status, errors);
  }
  if (res.status === 204) return undefined as T;
  return (await res.json()) as T;
}

export function listScreens(token: string | null): Promise<ScreenSummary[]> {
  return request(token, '/screens', {}, { emptyOn404: [] });
}
export function getConfig(token: string | null, screen: string): Promise<ConfigEnvelope> {
  return request(token, `/config?screen=${encodeURIComponent(screen)}`);
}
export function putDraft(token: string | null, screen: string, config: ScreenConfig) {
  return request(token, '/draft', { method: 'PUT', body: JSON.stringify({ screen, config }) });
}
export function putRails(token: string | null, screen: string, rails: Rails) {
  return request(token, '/rails', { method: 'PUT', body: JSON.stringify({ screen, rails }) });
}
export function publish(token: string | null, screen: string) {
  return request(token, '/publish', { method: 'POST', body: JSON.stringify({ screen }) });
}
export function rollback(token: string | null, screen: string, version: number) {
  return request(token, '/rollback', { method: 'POST', body: JSON.stringify({ screen, version }) });
}
export function kill(token: string | null, screen: string, sectionType: string) {
  return request<void>(token, '/kill', {
    method: 'POST',
    body: JSON.stringify({ screen, section_type: sectionType }),
  });
}
export function unkill(token: string | null, screen: string, sectionType: string) {
  return request<void>(token, '/unkill', {
    method: 'POST',
    body: JSON.stringify({ screen, section_type: sectionType }),
  });
}
export function listManifests(token: string | null): Promise<ManifestRow[]> {
  return request(token, '/manifests', {}, { emptyOn404: [] });
}
export function putManifest(token: string | null, platform: string, manifest: Manifest) {
  return request(token, '/manifests', {
    method: 'PUT',
    body: JSON.stringify({ platform, manifest }),
  });
}
```

(Add `ManifestRow { platform: string; manifest: Manifest; updated_at: string }`. ADAPT: exact GET /screens row shape — check the backend handler `list_screens` returns the `LayoutConfigRow` list; type `ScreenSummary` accordingly with at least `screen: string` and `published_version: number`.)

- [ ] **Step 3: run** `pnpm -F <admin-web pkg> test -- layout-editor` → PASS.
- [ ] **Step 4: Commit** — `feat(admin-web): layout editor API layer`

---

### Task 2: SectionTreeEditor (pure component, TDD)

**Files:**
- Create: `frontend/apps/admin-web/src/features/layout-editor/SectionTreeEditor.tsx`
- Test: `frontend/apps/admin-web/src/features/layout-editor/SectionTreeEditor.test.tsx`

**Interfaces:**
- Consumes: Task 1 types only. Fully controlled: `props { sections: SectionConfig[]; manifest: Manifest | null; kills: string[]; onChange(next: SectionConfig[]): void; onKill(type: string): void; onUnkill(type: string): void }`.
- Produces: the tree rows with, per section: type label; **required lock badge** (manifest `required`) and NO hide control for required sections; eye toggle (flips `visible`, default true) for optional ones; ↑/↓ buttons (disabled at edges) reordering via `onChange`; mode `<select>` populated from manifest `supported_modes` (hidden when none), changing `mode`; props `<textarea>` (JSON; invalid JSON shows inline error and does NOT call `onChange`); remove button (optional sections only); **killed badge** + kill/unkill button (confirm via `window.confirm`); unknown-type warning badge when the type is missing from the manifest. Below the list: add-section `<select>` of manifest types not yet present + Add button appending `{ type, visible: true }`.

- [ ] **Step 1: failing tests** — cover, with `user-event` and a spy `onChange`:
  1. required section (`gallery.v1` required in manifest) shows lock badge and no hide button; optional (`faq.v1`) shows the eye toggle, clicking it calls `onChange` with `visible: false`.
  2. ↓ on first section calls `onChange` with swapped order; ↑ disabled on first, ↓ disabled on last.
  3. mode select renders only for sections whose manifest entry has `supported_modes`, changing it patches `mode`.
  4. entering invalid JSON in props shows an error message and does NOT call `onChange`; valid JSON calls `onChange` with parsed `props`.
  5. killed section (type in `kills`) shows the killed badge and an Unkill button; un-killed shows Kill; clicking Kill (confirm mocked true) calls `onKill('faq.v1')`.
  6. add-select lists only manifest types not present; adding appends and calls `onChange`.
  7. a section whose type is absent from the manifest shows the unknown badge.

Write them with a local fixture manifest `{ platform:'web', components: { 'gallery.v1': { required: true }, 'faq.v1': { supported_modes: ['accordion','list'], default_mode: 'accordion' }, 'promo.v1': {} } }` and sections `[gallery.v1, faq.v1]`. Mock `react-i18next` with key-echo `t` (with defaultValue passthrough: `t: (_k, o) => o?.defaultValue ?? _k`).

- [ ] **Step 2: implement** — plain function component, no external state libs. Local state ONLY for the in-progress props text per section (`Record<string, string>` keyed by type, initialized from `JSON.stringify(section.props ?? {}, null, 2)`) and its error flag; everything else derives from props. Buttons are `@ppt/ui-kit` `Button` where convenient, native elements otherwise; badges via `Badge` or inline `<span className>` with `--ppt-*` token inline styles (house style). All strings via `t('admin.layout.…', { defaultValue: '…' })`.
- [ ] **Step 3: run to pass**; Biome clean.
- [ ] **Step 4: Commit** — `feat(admin-web): section tree editor component`

---

### Task 3: RailsEditor (pure component, TDD)

**Files:**
- Create: `frontend/apps/admin-web/src/features/layout-editor/RailsEditor.tsx`
- Test: `frontend/apps/admin-web/src/features/layout-editor/RailsEditor.test.tsx`

**Interfaces:**
- Consumes: Task 1 `Rails` type.
- Produces: controlled `props { rails: Rails; sectionTypes: string[]; onChange(next: Rails): void }` rendering: global `reorderable` checkbox; a row per section type with `hideable` + `mode_editable` checkboxes and a prop-whitelist text input (comma-separated names → `string[]`, trimmed, empties dropped; empty input removes the key from `prop_whitelist`).

- [ ] **Step 1: failing tests** — (1) toggling reorderable calls `onChange({...rails, reorderable:true})`; (2) checking hideable for a type adds it to `hideable` (and unchecking removes); (3) typing `title, limit` in the whitelist input calls `onChange` with `prop_whitelist: { 'faq.v1': ['title','limit'] }`; clearing it removes the key.
- [ ] **Step 2: implement + pass; Biome clean.**
- [ ] **Step 3: Commit** — `feat(admin-web): rails editor component`

---

### Task 4: LayoutEditorPage (wiring, TDD on behaviors)

**Files:**
- Create: `frontend/apps/admin-web/src/features/layout-editor/LayoutEditorPage.tsx`
- Test: `frontend/apps/admin-web/src/features/layout-editor/LayoutEditorPage.test.tsx`

**Interfaces:**
- Consumes: Tasks 1–3; `useAdminAuth` (`src/auth/AdminAuthContext.tsx`), `useToast` (`src/components/Toast.tsx`); TanStack Query.
- Produces: default-exported page component. Behavior contract:
  - Screen selector: `useQuery(['admin','platform','layout','screens'], listScreens)` → `<select>` + a "new screen" text input (pattern `^[a-z0-9-]+/[a-z0-9-]+$`, invalid → inline error, valid → selects it with an empty draft `{screen, version:0, sections:[]}`).
  - On screen selected: `useQuery(['admin','platform','layout','config',screen], () => getConfig(token, screen))` — 404 (LayoutApiError.status 404) treated as "new screen" empty envelope, not an error toast. Also `useQuery([… 'manifests'], listManifests)`; platform toggle web/mobile picks which manifest feeds the tree (default web).
  - Local draft state seeded from `envelope.config.draft` (deep-cloned) on load/screen change; a "dirty" flag when local ≠ loaded; Save Draft button → `useMutation(putDraft)` → success toast + invalidate config query; Rails likewise seeded from `envelope.config.rails` (defaulting to `{hideable:[],mode_editable:[],reorderable:false,prop_whitelist:{}}` when empty) with its own Save Rails mutation.
  - Publish button → `useMutation(publish)`; on `LayoutApiError` with `status===422` render the `errors` array verbatim in a persistent `Alert`/list (NOT just a toast); on success → success toast + invalidate.
  - Version history: table of `envelope.versions` (version, published_at) each with a Rollback button → confirm → `useMutation(rollback)` → invalidate.
  - Kill/unkill handlers passed to `SectionTreeEditor` → mutations → invalidate; kills list from `envelope.kills`.
  - Published-state summary line: `published_version` + whether a published config exists.

- [ ] **Step 1: failing tests** — wrapper stack per `CapabilitiesAdminPage.test.tsx` (MemoryRouter + QueryClientProvider + AdminAuthProvider with a seeded token — ADAPT: mirror how that test seeds auth; if it mocks the context, mock `useAdminAuth` directly). Mock the `./api` module with `vi.mock`. Cover:
  1. renders screen list from `listScreens` and loads config on selection (tree shows section types).
  2. eye-toggle → Save Draft calls `putDraft` with the edited sections array.
  3. publish rejection with `LayoutApiError(422, ['boom'])` renders the text `boom` (use `findByText`).
  4. rollback button (confirm mocked) calls `rollback(token, screen, version)`.
- [ ] **Step 2: implement + pass.** Layout: single column; `Card`-based groups (Sections / Rails / Publish & History / Danger zone). All strings `t('admin.layout.…', { defaultValue })`.
- [ ] **Step 3: Commit** — `feat(admin-web): layout editor page`

---

### Task 5: LayoutManifestsPage (TDD)

**Files:**
- Create: `frontend/apps/admin-web/src/features/layout-editor/LayoutManifestsPage.tsx`
- Test: `frontend/apps/admin-web/src/features/layout-editor/LayoutManifestsPage.test.tsx`

**Interfaces:**
- Consumes: Task 1 api (`listManifests`, `putManifest`), `useAdminAuth`, `useToast`.
- Produces: page listing stored manifests (platform, component count, updated_at, expandable `<pre>` of the JSON) + an upload form: platform `<select>` (web/mobile) + `<textarea>` for manifest JSON. Client-side gate before PUT: parses as JSON; has `components` object; `platform` field (if present) matches the selected platform — violations render inline errors, no request fired. Success → toast + invalidate. Include a hint line pointing at the checked-in app manifests (`frontend/apps/ppt-web/src/features/layout/manifest.json`, `frontend/apps/reality-web/src/lib/layout-manifest.json`) as canonical sources to paste from.

- [ ] **Step 1: failing tests** — (1) lists manifests from api; (2) invalid JSON in textarea → inline error, `putManifest` NOT called; (3) platform mismatch (body says web, select says mobile) → inline error, not called; (4) valid upload calls `putManifest('…','web', parsed)` and shows success toast.
- [ ] **Step 2: implement + pass.**
- [ ] **Step 3: Commit** — `feat(admin-web): layout manifests page`

---

### Task 6: Routing, nav, i18n, gates, docs

**Files:**
- Modify: `frontend/apps/admin-web/src/App.tsx` (+ routes `platform/layout` → `LayoutEditorPage`, `platform/layout/manifests` → `LayoutManifestsPage`, both wrapped in plain `<ProtectedRoute>` — platform-principal gate, NO `requiredCapability` (none exists for layout; adding one is a backend follow-up — note it in the report))
- Modify: `frontend/apps/admin-web/src/components/AdminLayout.tsx` (+ `NavItem`s in the platform `SidebarGroup`, un-gated)
- Modify: `frontend/apps/admin-web/messages/{en,sk,cs}.json` (add the `admin.layout.*` keys actually used — collect them from the components; sk/cs sensibly translated)
- Modify: `docs/repo-map.md` (extend the layout bullet: `Editor: admin-web/src/features/layout-editor/ (routes /platform/layout[, /manifests])`)
- Test: full gates

- [ ] **Step 1: routes + nav + i18n keys.**
- [ ] **Step 2: gates** — `cd frontend && pnpm check && pnpm typecheck && pnpm -F <admin-web pkg> test` (full admin-web suite; only pre-existing failures elsewhere in the workspace allowed — admin-web itself must be fully green). `pnpm check:fix` fallout on our files only.
- [ ] **Step 3: screen-map** — if `docs/screens/` has an admin product tree, add an Agent Log entry for a layout-editor screen doc; if none exists (likely), note the absence in the report — do NOT scaffold screen docs in this plan.
- [ ] **Step 4: Commit** — `feat(admin-web): wire layout editor routes, nav and i18n`

---

## Deliberate scope decisions (do not "fix" during implementation)

- **No iframe live preview / postMessage bridge** — next slice (spec §6.1).
- **No per-tenant override editing in admin-web MVP** — superadmin edits base config + rails only; the per-tenant view badge system comes with the tenant editor slice.
- **No drag-and-drop** — ↑/↓ buttons only.
- **No new admin capability** — platform-principal gate; capability registration (backend + `admin-ui/src/capabilities.ts`) is a follow-up.
- **No publish→ISR webhook yet** — reality-web still refreshes via `revalidate: 60`.
- **Platform overrides (`section.overrides`) are round-tripped untouched** — the tree edits base fields only; per-platform override editing UX arrives with the preview slice.
- **No E2E spec** — the admin-web E2E suite currently skips without a backend; a layout-editor spec joins when the dev-stack seeding lands.

## Out of scope (subsequent plans)

1. Preview bridge + platform-override editing + per-tenant view.
2. Tenant editor in ppt-web + rails-authoring polish.
3. Mobile registries/renderers; publish webhook + revalidateTag plumbing.
