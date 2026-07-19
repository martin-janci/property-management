# Layout Defensive Rendering Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the two pilot screens (`ppt/dashboard` in ppt-web, `reality/listing-detail` in reality-web) render from the resolved-layout endpoint through section registries — with placeholders for required sections, clean collapse for hidden ones, per-section error isolation, gap-owned spacing, and a hardcoded fallback layout whenever the endpoint is unavailable.

**Architecture:** Each app gets (a) a **section registry** (`Record<sectionType, ComponentType>` + per-section metadata), (b) a **LayoutRenderer** that takes a `ResolvedScreen` and renders registry components in order — placeholder for `presentation: "placeholder"`, skip-and-log for unknown types, each section wrapped in an error boundary, container owns spacing via `gap`, (c) a **DEFAULT_LAYOUT** constant used when the fetch fails or 404s (the endpoint is additive, never gating — spec §4), and (d) a **checked-in registry manifest** (JSON mirroring the registry) destined for `PUT /platform-admin/layout/manifests`. ppt-web fetches client-side via a new `@ppt/api-client` layout domain (TanStack Query); reality-web fetches server-side with ISR tags following its `getListing` pattern.

**Tech Stack:** React 19, TanStack Query 5 (ppt-web), Next.js App Router + next-intl (reality-web), Vitest + Testing Library, plain CSS with `--ppt-space-*` tokens (ppt-web) / styled-jsx (reality-web).

## Global Constraints

- Spec: `docs/superpowers/specs/2026-07-19-layout-content-manager-design.md` §4 (resilience rules 1–4), §9 step 2. Backend endpoints exist (PR #2426): api-server `GET /api/v1/layout/resolved/{screen}?platform=web` (auth), reality-server same path (public).
- **Branch:** `feature/layout-defensive-rendering` from `dev` (after #2426 merges; if not merged yet, branch from `feature/layout-control-plane` and note it).
- **Layout fetch NEVER breaks the page.** Any failure (network, non-2xx, parse) → `DEFAULT_LAYOUT`. reality-web must not `notFound()`/500 because of layout; ppt-web must render the default sections while loading and on error.
- **Spacing ownership (spec §4.3):** the LayoutRenderer container uses `gap`; section components ship zero external margin. In reality-web this means removing `margin-top` from `.section`/`.key-details`/`.listing-header` in the refactored composition and using a flex column with `gap: 24px` in the main column.
- **Unknown types:** render nothing, `console.warn` once per type (spec §4.1 client-side leg; server-side filtering comes later with app_version).
- **Placeholder:** neutral "section unavailable" block, i18n'd, reserving modest space (`min-height` ~96px), `role="status"`. Never a skeleton (spec: skeletons imply loading).
- **Section type names are the contract:** `ppt/dashboard` → `dashboard-stats.v1`, `action-queue.v1` (both required). `reality/listing-detail` → `gallery.v1` (required), `listing-header.v1` (required), `key-details.v1`, `description.v1`, `features.v1`, `additional-info.v1`, `resources.v1`, `agent-contact.v1`. Manifest JSONs must list exactly the registry's types — a unit test enforces registry↔manifest consistency in each app.
- Frontend gates: `cd frontend && pnpm install` first in a fresh worktree (memory: required), then `pnpm check && pnpm typecheck && pnpm test` (filter to touched packages while iterating: `pnpm -F @ppt/web test`, `pnpm -F @ppt/reality-web test`, `pnpm -F @ppt/api-client test` if it has tests configured — if a package has no test script, `pnpm -F <pkg> typecheck` is the gate; report which applied).
- i18n: ppt-web keys under `dashboard.layout.*` + `layout.*` in `frontend/apps/ppt-web/messages/*.json` (all locale files, English text as value placeholder-translated); reality-web namespace `layout` in `frontend/apps/reality-web/messages/*.json` (en, sk, cs, de, pl, hu).
- Commit scopes: `feat(api-client)`, `feat(ppt-web)`, `feat(reality-web)`, `docs(...)`. ADAPT rule as in prior plans: import paths/helper names may be adapted to the mirrored files named per task; logic and contracts may not. Report adaptations.
- Screen-map protocol (root CLAUDE.md section A): the final task adds Agent Log entries to `docs/screens/ppt/*dashboard*` and `docs/screens/reality/*listing-detail*` docs (locate by grep; if no matching screen doc exists, note it instead of creating one).

## File Structure

```
frontend/packages/api-client/src/layout/api.ts       # types + fetcher (hand-written domain, mirrors outages/)
frontend/packages/api-client/src/layout/hooks.ts     # layoutKeys + useResolvedLayout
frontend/packages/api-client/src/index.ts            # + exports
frontend/apps/ppt-web/src/features/layout/registry.tsx        # ppt-web section registry + DEFAULT_LAYOUT + manifest export
frontend/apps/ppt-web/src/features/layout/LayoutRenderer.tsx  # renderer + Placeholder + SectionBoundary
frontend/apps/ppt-web/src/features/layout/LayoutRenderer.css
frontend/apps/ppt-web/src/features/layout/manifest.json       # checked-in web manifest (ppt sections)
frontend/apps/ppt-web/src/features/layout/*.test.tsx          # renderer + registry/manifest tests
frontend/apps/ppt-web/src/features/dashboard/pages/ManagerDashboardPage.tsx   # refactor to sections
frontend/apps/ppt-web/src/features/dashboard/components/DashboardStats.tsx    # extracted stats section
frontend/apps/reality-web/src/lib/layout.ts                   # server fetch + DEFAULT_LAYOUT + types
frontend/apps/reality-web/src/lib/layout.test.ts
frontend/apps/reality-web/src/components/listings/sections/registry.tsx  # section registry (client)
frontend/apps/reality-web/src/components/listings/LayoutSections.tsx     # renderer + Placeholder + boundary
frontend/apps/reality-web/src/components/listings/ListingDetailContent.tsx  # refactor to registry-driven
frontend/apps/reality-web/src/components/listings/LayoutSections.test.tsx
frontend/apps/reality-web/src/lib/layout-manifest.json        # checked-in web manifest (reality sections)
frontend/apps/reality-web/src/app/[locale]/listings/[slug]/page.tsx  # + layout fetch
docs/superpowers/specs/…                              # (no spec change)
docs/repo-map.md                                      # extend layout bullet
```

---

### Task 1: `@ppt/api-client` layout domain

**Files:**
- Create: `frontend/packages/api-client/src/layout/api.ts`
- Create: `frontend/packages/api-client/src/layout/hooks.ts`
- Modify: `frontend/packages/api-client/src/index.ts` (add exports alongside the other hand-written domains)
- Test: `frontend/packages/api-client/src/layout/api.test.ts` (only if this package runs Vitest — check `package.json`; if it has no test setup, put the parse tests in ppt-web's test tree instead in Task 2 and note it)

**Interfaces:**
- Consumes: `authenticatedFetchJson` from `packages/api-client/src/lib/fetch.ts` (ADAPT: if its signature differs, mirror the newest domain module that uses it).
- Produces (used by Tasks 2–3): types `ResolvedSection { type: string; mode?: string; props?: Record<string, unknown>; presentation: 'visible' | 'placeholder' }`, `ResolvedScreen { screen: string; version: number; sections: ResolvedSection[] }`; `fetchResolvedLayout(screen: string, platform?: 'web' | 'mobile'): Promise<ResolvedScreen>`; `layoutKeys` factory; `useResolvedLayout(screen, options?)` hook.

- [ ] **Step 1: api.ts**

```ts
import { authenticatedFetchJson } from '../lib/fetch';

export interface ResolvedSection {
  type: string;
  mode?: string;
  props?: Record<string, unknown>;
  presentation: 'visible' | 'placeholder';
}

export interface ResolvedScreen {
  screen: string;
  version: number;
  sections: ResolvedSection[];
}

const API_BASE = '/api/v1/layout';

/** Fetch the resolved layout for a screen. Throws on any failure — callers
 *  are expected to fall back to their DEFAULT_LAYOUT (spec §4: the layout
 *  endpoint is additive, never gating). */
export async function fetchResolvedLayout(
  screen: string,
  platform: 'web' | 'mobile' = 'web',
): Promise<ResolvedScreen> {
  const data = await authenticatedFetchJson<ResolvedScreen>(
    `${API_BASE}/resolved/${screen}?platform=${platform}`,
  );
  if (!data || !Array.isArray(data.sections)) {
    throw new Error('layout: malformed ResolvedScreen payload');
  }
  return data;
}
```

(ADAPT: if `authenticatedFetchJson` takes `(url, options)` or returns a wrapped result, follow the exact call idiom of the most recently added domain module using it; the thrown-error-on-failure contract must hold.)

- [ ] **Step 2: hooks.ts**

```ts
import { useQuery } from '@tanstack/react-query';
import { fetchResolvedLayout, type ResolvedScreen } from './api';

export const layoutKeys = {
  all: ['layout'] as const,
  resolved: (screen: string, platform: string) =>
    [...layoutKeys.all, 'resolved', screen, platform] as const,
};

export function useResolvedLayout(screen: string, platform: 'web' | 'mobile' = 'web') {
  return useQuery<ResolvedScreen>({
    queryKey: layoutKeys.resolved(screen, platform),
    queryFn: () => fetchResolvedLayout(screen, platform),
    staleTime: 60_000,
    retry: 1,
  });
}
```

- [ ] **Step 3: export** — in `src/index.ts`, add exports next to the other domains: `export * from './layout/api'; export * from './layout/hooks';` (ADAPT to the file's existing export style — some domains use named re-exports).

- [ ] **Step 4: verify** — `cd frontend && pnpm -F @ppt/api-client typecheck` (or the package's actual check script) clean. If the package has a Vitest setup, add a test asserting `fetchResolvedLayout` throws on a malformed payload (mock `authenticatedFetchJson` with `vi.mock`); otherwise defer that assertion to Task 2's renderer tests and say so in the report.

- [ ] **Step 5: Commit** — `feat(api-client): layout domain — resolved-layout fetcher and query hook`

---

### Task 2: ppt-web registry + LayoutRenderer (TDD)

**Files:**
- Create: `frontend/apps/ppt-web/src/features/layout/registry.tsx`
- Create: `frontend/apps/ppt-web/src/features/layout/LayoutRenderer.tsx`
- Create: `frontend/apps/ppt-web/src/features/layout/LayoutRenderer.css`
- Create: `frontend/apps/ppt-web/src/features/layout/manifest.json`
- Test: `frontend/apps/ppt-web/src/features/layout/LayoutRenderer.test.tsx`, `frontend/apps/ppt-web/src/features/layout/manifest.test.ts`
- Modify: `frontend/apps/ppt-web/messages/en.json` (+ every other locale file in `messages/`): add under the existing style of nesting —
  `"layout": { "placeholderTitle": "Section unavailable", "placeholderBody": "This section is temporarily unavailable." }`

**Interfaces:**
- Consumes: `ResolvedScreen`/`ResolvedSection` from `@ppt/api-client`; the app's `ErrorBoundary` (`src/components/ErrorBoundary.tsx`, has `fallback` prop).
- Produces (Task 3 relies on): `SectionDef { component: React.ComponentType<SectionProps>; required: boolean; supportedModes: string[] }`, `SectionProps { mode?: string; props?: Record<string, unknown> }`, `registry: Record<string, SectionDef>` (populated in Task 3 — this task ships it EMPTY plus a `registerSections(map)` helper OR direct object spread; keep it a plain object literal populated in Task 3, with tests using their own local registries), `DEFAULT_DASHBOARD_LAYOUT: ResolvedScreen` (populated Task 3), `<LayoutRenderer layout={ResolvedScreen} registry={...} />`.

- [ ] **Step 1: registry.tsx (structure only; entries land in Task 3)**

```tsx
import type { ComponentType } from 'react';
import type { ResolvedScreen } from '@ppt/api-client';

export interface SectionProps {
  mode?: string;
  props?: Record<string, unknown>;
}

export interface SectionDef {
  component: ComponentType<SectionProps>;
  required: boolean;
  supportedModes: string[];
}

export type SectionRegistry = Record<string, SectionDef>;

/** ppt-web dashboard sections — populated by the dashboard feature (Task 3). */
export const dashboardRegistry: SectionRegistry = {};

/** Rendered when the layout endpoint is unavailable (spec §4: never gate the
 *  page on layout). Task 3 fills the real section list. */
export const DEFAULT_DASHBOARD_LAYOUT: ResolvedScreen = {
  screen: 'ppt/dashboard',
  version: 0,
  sections: [],
};

/** The registry manifest for upload to PUT /platform-admin/layout/manifests.
 *  Kept in manifest.json; the manifest.test.ts asserts it mirrors the registry. */
export function registryManifest(registry: SectionRegistry) {
  return {
    platform: 'web',
    components: Object.fromEntries(
      Object.entries(registry).map(([type, def]) => [
        type,
        {
          required: def.required,
          ...(def.supportedModes.length > 0
            ? { supported_modes: def.supportedModes, default_mode: def.supportedModes[0] }
            : {}),
        },
      ]),
    ),
  };
}
```

- [ ] **Step 2: failing renderer tests**

`LayoutRenderer.test.tsx` — use a local test registry; mock i18n per the app's convention (`vi.mock('react-i18next', …)` returning key-echo `t`):

```tsx
import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { ResolvedScreen } from '@ppt/api-client';
import { LayoutRenderer } from './LayoutRenderer';
import type { SectionRegistry } from './registry';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

const registry: SectionRegistry = {
  'alpha.v1': { component: () => <div>ALPHA</div>, required: true, supportedModes: [] },
  'beta.v1': {
    component: ({ mode }) => <div>BETA:{mode ?? 'none'}</div>,
    required: false,
    supportedModes: ['list', 'grid'],
  },
  'boom.v1': {
    component: () => {
      throw new Error('section crash');
    },
    required: false,
    supportedModes: [],
  },
};

function layoutOf(sections: ResolvedScreen['sections']): ResolvedScreen {
  return { screen: 'test/screen', version: 1, sections };
}

describe('LayoutRenderer', () => {
  it('renders sections in resolved order with mode passed through', () => {
    render(
      <LayoutRenderer
        registry={registry}
        layout={layoutOf([
          { type: 'beta.v1', mode: 'grid', presentation: 'visible' },
          { type: 'alpha.v1', presentation: 'visible' },
        ])}
      />,
    );
    const texts = screen.getAllByText(/ALPHA|BETA/).map((n) => n.textContent);
    expect(texts).toEqual(['BETA:grid', 'ALPHA']);
  });

  it('renders a placeholder for presentation=placeholder and no component output', () => {
    render(
      <LayoutRenderer
        registry={registry}
        layout={layoutOf([{ type: 'alpha.v1', presentation: 'placeholder' }])}
      />,
    );
    expect(screen.queryByText('ALPHA')).toBeNull();
    expect(screen.getByRole('status')).toHaveTextContent('layout.placeholderTitle');
  });

  it('skips unknown section types entirely and warns once per type', () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
    render(
      <LayoutRenderer
        registry={registry}
        layout={layoutOf([
          { type: 'ghost.v9', presentation: 'visible' },
          { type: 'ghost.v9', presentation: 'visible' },
          { type: 'alpha.v1', presentation: 'visible' },
        ])}
      />,
    );
    expect(screen.getByText('ALPHA')).toBeInTheDocument();
    expect(warn.mock.calls.filter((c) => String(c[0]).includes('ghost.v9'))).toHaveLength(1);
    warn.mockRestore();
  });

  it('isolates a crashing section: placeholder for it, siblings render', () => {
    const err = vi.spyOn(console, 'error').mockImplementation(() => {});
    render(
      <LayoutRenderer
        registry={registry}
        layout={layoutOf([
          { type: 'boom.v1', presentation: 'visible' },
          { type: 'alpha.v1', presentation: 'visible' },
        ])}
      />,
    );
    expect(screen.getByText('ALPHA')).toBeInTheDocument();
    expect(screen.getByRole('status')).toBeInTheDocument();
    err.mockRestore();
  });
});
```

`manifest.test.ts`:

```ts
import { describe, expect, it } from 'vitest';
import manifest from './manifest.json';
import { dashboardRegistry, registryManifest } from './registry';

describe('layout manifest', () => {
  it('mirrors the dashboard registry exactly', () => {
    expect(manifest).toEqual(registryManifest(dashboardRegistry));
  });
});
```

- [ ] **Step 3: run to verify failure** — `cd frontend && pnpm -F @ppt/web test -- layout` (ADAPT to the app's actual test filter syntax). Expected: FAIL (LayoutRenderer missing).

- [ ] **Step 4: implement LayoutRenderer.tsx**

```tsx
import { useTranslation } from 'react-i18next';
import type { ResolvedScreen } from '@ppt/api-client';
import { ErrorBoundary } from '../../components/ErrorBoundary';
import type { SectionRegistry } from './registry';
import './LayoutRenderer.css';

const warnedTypes = new Set<string>();

function Placeholder() {
  const { t } = useTranslation();
  return (
    <div className="layout-placeholder" role="status">
      <p className="layout-placeholder__title">{t('layout.placeholderTitle')}</p>
      <p className="layout-placeholder__body">{t('layout.placeholderBody')}</p>
    </div>
  );
}

export interface LayoutRendererProps {
  layout: ResolvedScreen;
  registry: SectionRegistry;
}

/** Renders a resolved layout defensively (spec §4): unknown type → skip +
 *  warn once; placeholder presentation → Placeholder; crashing section →
 *  Placeholder via boundary, siblings unaffected; container owns spacing. */
export function LayoutRenderer({ layout, registry }: LayoutRendererProps) {
  return (
    <div className="layout-sections">
      {layout.sections.map((section, i) => {
        const def = registry[section.type];
        if (!def) {
          if (!warnedTypes.has(section.type)) {
            warnedTypes.add(section.type);
            console.warn(`layout: unknown section type ${section.type} — skipped`);
          }
          return null;
        }
        if (section.presentation === 'placeholder') {
          return <Placeholder key={`${section.type}-${i}`} />;
        }
        const Component = def.component;
        return (
          <ErrorBoundary key={`${section.type}-${i}`} fallback={<Placeholder />}>
            <Component mode={section.mode} props={section.props} />
          </ErrorBoundary>
        );
      })}
    </div>
  );
}
```

`LayoutRenderer.css` (container owns spacing — spec §4.3):

```css
.layout-sections {
  display: flex;
  flex-direction: column;
  gap: var(--ppt-space-8);
}

.layout-placeholder {
  min-height: 96px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--ppt-space-2);
  border: 1px dashed var(--ppt-color-border, #d0d0d0);
  border-radius: 8px;
  color: var(--ppt-color-text-secondary, #666);
}

.layout-placeholder__title {
  font-weight: 600;
  margin: 0;
}

.layout-placeholder__body {
  margin: 0;
  font-size: var(--ppt-font-size-sm, 0.875rem);
}
```

For this task, `manifest.json` is the empty-registry manifest: `{ "platform": "web", "components": {} }` (Task 3 fills it).

- [ ] **Step 5: run to verify pass** — same test command; renderer tests + manifest test PASS. If the app's `ErrorBoundary` re-throws or its `fallback` prop behaves differently (ADAPT), wrap with a local minimal boundary class inside `LayoutRenderer.tsx` instead and report it.

- [ ] **Step 6: Commit** — `feat(ppt-web): layout section registry and defensive LayoutRenderer`

---

### Task 3: ppt-web dashboard refactor to sections

**Files:**
- Create: `frontend/apps/ppt-web/src/features/dashboard/components/DashboardStats.tsx` (extract the stats grid from `ManagerDashboardPage`)
- Modify: `frontend/apps/ppt-web/src/features/dashboard/pages/ManagerDashboardPage.tsx`
- Modify: `frontend/apps/ppt-web/src/features/layout/registry.tsx` (fill registry + DEFAULT_DASHBOARD_LAYOUT)
- Modify: `frontend/apps/ppt-web/src/features/layout/manifest.json`
- Test: `frontend/apps/ppt-web/src/features/dashboard/pages/ManagerDashboardPage.test.tsx`

**Interfaces:**
- Consumes: Task 1 `useResolvedLayout`, Task 2 renderer/registry types, existing `QuickStat`, `ActionQueue` components.
- Produces: registry entries `dashboard-stats.v1` (required, no modes) and `action-queue.v1` (required, no modes); `DEFAULT_DASHBOARD_LAYOUT` listing both visible; page renders `LayoutRenderer` fed by `useResolvedLayout('ppt/dashboard')` with fallback.

- [ ] **Step 1: extract DashboardStats**

Move the `<div className="dashboard-page__stats">…</div>` block (the four `QuickStat`s) from `ManagerDashboardPage.tsx` verbatim into `DashboardStats.tsx` as a component of type `ComponentType<SectionProps>` (ignore `mode`/`props` for now). Keep its CSS class names (the CSS file already styles them); remove the block's own `margin-bottom` if it has one (container gap owns spacing now — move any needed spacing tweak into the co-located CSS).

- [ ] **Step 2: fill the registry**

In `registry.tsx`:

```tsx
import { DashboardStats } from '../dashboard/components/DashboardStats';
import { ActionQueue } from '../dashboard/components/ActionQueue';

export const dashboardRegistry: SectionRegistry = {
  'dashboard-stats.v1': { component: DashboardStats, required: true, supportedModes: [] },
  'action-queue.v1': {
    component: () => <ActionQueue userRole="manager" />,
    required: true,
    supportedModes: [],
  },
};

export const DEFAULT_DASHBOARD_LAYOUT: ResolvedScreen = {
  screen: 'ppt/dashboard',
  version: 0,
  sections: [
    { type: 'dashboard-stats.v1', presentation: 'visible' },
    { type: 'action-queue.v1', presentation: 'visible' },
  ],
};
```

(ADAPT: if `ActionQueue`'s props differ, mirror its current usage in the page.) Update `manifest.json` to `registryManifest(dashboardRegistry)`'s output:

```json
{
  "platform": "web",
  "components": {
    "action-queue.v1": { "required": true },
    "dashboard-stats.v1": { "required": true }
  }
}
```

- [ ] **Step 3: failing page test**

```tsx
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ManagerDashboardPage } from './ManagerDashboardPage';

vi.mock('@ppt/api-client', async (importOriginal) => ({
  ...(await importOriginal<object>()),
  fetchResolvedLayout: vi.fn().mockRejectedValue(new Error('network down')),
}));

function renderPage() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <ManagerDashboardPage />
    </QueryClientProvider>,
  );
}

describe('ManagerDashboardPage (defensive layout)', () => {
  it('renders the default layout when the layout endpoint is down', async () => {
    renderPage();
    await waitFor(() => {
      // both required default sections present despite fetch failure
      expect(document.querySelector('.dashboard-page__stats')).toBeInTheDocument();
    });
  });
});
```

(ADAPT: the page may need Router/i18n wrappers per existing page tests — mirror `favorites/page.test.tsx`-style setup from the app's conventions; `ActionQueue` may need its hook mocked — `vi.mock` `../hooks/useActionQueue` returning empty mock data, mirroring however `ActionQueue` tests do it. Keep the assertion: stats section renders even though layout fetch rejected.)

- [ ] **Step 4: refactor the page**

`ManagerDashboardPage.tsx` keeps its header, then delegates the body:

```tsx
import { useResolvedLayout } from '@ppt/api-client';
import { LayoutRenderer } from '../../layout/LayoutRenderer';
import { dashboardRegistry, DEFAULT_DASHBOARD_LAYOUT } from '../../layout/registry';

// inside the component, after the header:
const { data: layout } = useResolvedLayout('ppt/dashboard');
// …
<LayoutRenderer layout={layout ?? DEFAULT_DASHBOARD_LAYOUT} registry={dashboardRegistry} />
```

While loading or on error, `data` is undefined → default layout renders (never a blank page). Remove the now-moved stats block and direct `<ActionQueue …/>` from the page JSX.

- [ ] **Step 5: run tests** — `pnpm -F @ppt/web test -- dashboard layout` → new page test + Task 2 tests + manifest test all PASS (manifest test now covers the filled registry).
- [ ] **Step 6: Commit** — `feat(ppt-web): dashboard renders through resolved layout with default fallback`

---

### Task 4: reality-web server-side layout lib

**Files:**
- Create: `frontend/apps/reality-web/src/lib/layout.ts`
- Create: `frontend/apps/reality-web/src/lib/layout-manifest.json`
- Test: `frontend/apps/reality-web/src/lib/layout.test.ts`

**Interfaces:**
- Consumes: the `resolveApiBase(host)` pattern from `src/app/[locale]/listings/[slug]/page.tsx` (ADAPT: if `resolveApiBase` is not exported, replicate the identical helper inside `layout.ts` and note it).
- Produces (Tasks 5–6): types `ResolvedSection`/`ResolvedScreen` (same shape as api-client's — duplicated here because reality-web does not depend on `@ppt/api-client`), `DEFAULT_LISTING_DETAIL_LAYOUT: ResolvedScreen` (all eight section types, visible, base order matching today's JSX order), `getResolvedLayout(host: string | null): Promise<ResolvedScreen>` — never throws, falls back to the default.

- [ ] **Step 1: failing tests**

```ts
import { afterEach, describe, expect, it, vi } from 'vitest';
import { DEFAULT_LISTING_DETAIL_LAYOUT, getResolvedLayout } from './layout';

describe('getResolvedLayout', () => {
  afterEach(() => vi.unstubAllGlobals());

  it('returns fetched layout on success', async () => {
    const payload = {
      screen: 'reality/listing-detail',
      version: 3,
      sections: [{ type: 'gallery.v1', presentation: 'visible' }],
    };
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(
      new Response(JSON.stringify(payload), { status: 200 }),
    ));
    await expect(getResolvedLayout(null)).resolves.toEqual(payload);
  });

  it('falls back to the default layout on non-2xx', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response('nope', { status: 404 })));
    await expect(getResolvedLayout(null)).resolves.toEqual(DEFAULT_LISTING_DETAIL_LAYOUT);
  });

  it('falls back to the default layout on network error', async () => {
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('down')));
    await expect(getResolvedLayout(null)).resolves.toEqual(DEFAULT_LISTING_DETAIL_LAYOUT);
  });

  it('falls back on malformed payload', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ nope: true }), { status: 200 }),
    ));
    await expect(getResolvedLayout(null)).resolves.toEqual(DEFAULT_LISTING_DETAIL_LAYOUT);
  });

  it('default layout lists all eight sections visible in base order', () => {
    expect(DEFAULT_LISTING_DETAIL_LAYOUT.sections.map((s) => s.type)).toEqual([
      'gallery.v1',
      'listing-header.v1',
      'key-details.v1',
      'description.v1',
      'features.v1',
      'additional-info.v1',
      'resources.v1',
      'agent-contact.v1',
    ]);
  });
});
```

- [ ] **Step 2: run to verify failure**, then **implement `layout.ts`**:

```ts
export interface ResolvedSection {
  type: string;
  mode?: string;
  props?: Record<string, unknown>;
  presentation: 'visible' | 'placeholder';
}

export interface ResolvedScreen {
  screen: string;
  version: number;
  sections: ResolvedSection[];
}

const SCREEN = 'reality/listing-detail';

export const DEFAULT_LISTING_DETAIL_LAYOUT: ResolvedScreen = {
  screen: SCREEN,
  version: 0,
  sections: [
    { type: 'gallery.v1', presentation: 'visible' },
    { type: 'listing-header.v1', presentation: 'visible' },
    { type: 'key-details.v1', presentation: 'visible' },
    { type: 'description.v1', presentation: 'visible' },
    { type: 'features.v1', presentation: 'visible' },
    { type: 'additional-info.v1', presentation: 'visible' },
    { type: 'resources.v1', presentation: 'visible' },
    { type: 'agent-contact.v1', presentation: 'visible' },
  ],
};

// ADAPT: mirror resolveApiBase from listings/[slug]/page.tsx (import it if
// exported; otherwise replicate it verbatim here and note the duplication).
declare function resolveApiBase(host: string | null): string;

/** Fetch the resolved layout for the listing-detail screen. NEVER throws and
 *  NEVER gates the page — any failure returns the default layout (spec §4). */
export async function getResolvedLayout(host: string | null): Promise<ResolvedScreen> {
  try {
    const tags = host ? [`host:${host}:layout:listing-detail`] : ['layout:listing-detail'];
    const response = await fetch(
      `${resolveApiBase(host)}/api/v1/layout/resolved/${SCREEN}?platform=web`,
      { headers: host ? { Host: host } : {}, next: { revalidate: 60, tags } },
    );
    if (!response.ok) return DEFAULT_LISTING_DETAIL_LAYOUT;
    const body = (await response.json()) as ResolvedScreen;
    if (!body || body.screen !== SCREEN || !Array.isArray(body.sections)) {
      return DEFAULT_LISTING_DETAIL_LAYOUT;
    }
    return body;
  } catch {
    return DEFAULT_LISTING_DETAIL_LAYOUT;
  }
}
```

(The `declare function` line is a stand-in for the ADAPT import/replication — the shipped file must contain a real implementation. In tests, `resolveApiBase` must be reachable without env vars — the localhost fallback branch covers that.)

- [ ] **Step 3: layout-manifest.json** — checked-in reality web manifest:

```json
{
  "platform": "web",
  "components": {
    "additional-info.v1": { "required": false },
    "agent-contact.v1": { "required": false },
    "description.v1": { "required": false },
    "features.v1": { "required": false },
    "gallery.v1": { "required": true },
    "key-details.v1": { "required": false },
    "listing-header.v1": { "required": true },
    "resources.v1": { "required": false }
  }
}
```

Add a consistency assertion to `layout.test.ts`: manifest component keys === default-layout section types (sorted).

- [ ] **Step 4: run tests to pass** — `pnpm -F @ppt/reality-web test -- layout`.
- [ ] **Step 5: Commit** — `feat(reality-web): server-side resolved-layout fetch with fail-safe default`

---

### Task 5: reality-web registry + LayoutSections renderer (TDD)

**Files:**
- Create: `frontend/apps/reality-web/src/components/listings/sections/registry.tsx`
- Create: `frontend/apps/reality-web/src/components/listings/LayoutSections.tsx`
- Test: `frontend/apps/reality-web/src/components/listings/LayoutSections.test.tsx`
- Modify: `frontend/apps/reality-web/messages/en.json` + sk/cs/de/pl/hu: add namespace `"layout": { "placeholderTitle": "…", "placeholderBody": "…" }` (English text; translate the sk/cs/de/pl/hu values sensibly).

**Interfaces:**
- Consumes: `ResolvedScreen` types from `../../lib/layout` (Task 4); `ListingDetail` type from `@ppt/reality-api-client`.
- Produces (Task 6 relies on): `ListingSectionProps { listing: ListingDetail; mode?: string; props?: Record<string, unknown> }`, `listingRegistry: Record<string, { component: ComponentType<ListingSectionProps>; required: boolean }>` (entries filled in Task 6), `<LayoutSections layout={ResolvedScreen} listing={ListingDetail} registry={...} />` — client component, flex column with `gap: 24px` (styled-jsx), placeholder (`role="status"`, next-intl `useTranslations('layout')`), unknown-type skip + warn-once, per-section error isolation via a small local class boundary (reality-web has no reusable ErrorBoundary component — write a minimal one inside `LayoutSections.tsx`).

- [ ] **Step 1: failing tests** — mirror Task 2's four renderer tests (order+mode passthrough, placeholder, unknown-skip-warn-once, crash-isolation) adapted to `LayoutSections` with a dummy `listing` object cast as `ListingDetail` and the global next-intl mock from `src/test/setup.tsx` (already key-echoes `useTranslations`). Same assertions, `role="status"` for placeholders.
- [ ] **Step 2: implement** — `LayoutSections.tsx` as `'use client'`; structure identical to ppt-web's `LayoutRenderer` but: props include `listing` forwarded to every section component; styling via styled-jsx:

```tsx
<div className="layout-sections">
  {/* …mapping identical to ppt-web LayoutRenderer, sections get
       <Component listing={listing} mode={section.mode} props={section.props} /> … */}
  <style jsx>{`
    .layout-sections {
      display: flex;
      flex-direction: column;
      gap: 24px;
    }
  `}</style>
</div>
```

plus a `Placeholder` (styled-jsx, `min-height: 96px`, dashed border, `role="status"`, `useTranslations('layout')`) and a minimal class `SectionBoundary extends Component` with `getDerivedStateFromError` rendering `<Placeholder />`. Registry file mirrors ppt-web's `SectionDef` shape minus modes metadata (add `supportedModes: string[]` anyway for manifest symmetry).

- [ ] **Step 3: run to pass** — `pnpm -F @ppt/reality-web test -- LayoutSections`.
- [ ] **Step 4: Commit** — `feat(reality-web): listing section registry and defensive LayoutSections renderer`

---

### Task 6: reality-web listing-detail refactor

**Files:**
- Modify: `frontend/apps/reality-web/src/components/listings/ListingDetailContent.tsx` (decompose the hardcoded main-column sections into registry-driven rendering)
- Create: section wrapper components under `frontend/apps/reality-web/src/components/listings/sections/` — `Gallery.tsx`, `ListingHeader.tsx`, `KeyDetails.tsx`, `Description.tsx`, `Features.tsx`, `AdditionalInfo.tsx`, `Resources.tsx`, `AgentContact.tsx` — each extracting the corresponding JSX block from `ListingDetailContent` VERBATIM (including its styled-jsx rules), typed `ComponentType<ListingSectionProps>`
- Modify: `frontend/apps/reality-web/src/components/listings/sections/registry.tsx` (fill entries: gallery + listing-header `required: true`, rest `required: false`)
- Modify: `frontend/apps/reality-web/src/app/[locale]/listings/[slug]/page.tsx` (fetch layout via `getResolvedLayout(host)` in parallel with `getListing` — `Promise.all`; pass `layout` prop through to `ListingDetailContent`)
- Test: extend `frontend/apps/reality-web/src/components/listings/ListingDetailContent.test.tsx` (it exists — keep its current assertions passing; add one: with a layout that hides `features.v1` and marks `gallery.v1` placeholder, the features section is absent and a `role="status"` placeholder renders)

**Constraints for the extraction:**
- Preserve today's conditional logic inside the section components (e.g. `Features` renders `null` when `activeFeatures.length === 0`; `AgentContact` renders `null` without `listing.agent`; `Resources` without URLs → `null`). A section returning `null` is equivalent to collapse — fine.
- Two-column grid stays: main column = `<LayoutSections layout … registry …/>` EXCEPT `agent-contact.v1`, which today lives in the sidebar. Resolution: keep the two-column shell in `ListingDetailContent`; the sidebar renders `agent-contact.v1` iff the resolved layout lists it visible (look it up from `layout.sections`); `LayoutSections` receives the layout with `agent-contact.v1` filtered out. Placeholder presentation for agent-contact renders the placeholder in the sidebar.
- Remove `margin-top` from the extracted sections' styled-jsx (`.section`, `.key-details`, `.listing-header` rules) — spacing now comes from the renderer's `gap: 24px` (spec §4.3). Header/breadcrumb/Footer stay outside the layout system.
- Keep JSON-LD, `Header`, breadcrumbs, `Footer` untouched.

- [ ] **Step 1: write the new failing test** (hide features, placeholder gallery). Run: existing + new tests fail only for the right reason (missing prop/renderer), not by breaking current assertions.
- [ ] **Step 2: extract sections + refactor content + page fetch** per constraints above. `page.tsx`: `const [listing, layout] = await Promise.all([getListing(...), getResolvedLayout(host)]);` — the layout fetch cannot reject (Task 4 contract).
- [ ] **Step 3: run the full reality-web suite** — `pnpm -F @ppt/reality-web test`. ALL pre-existing tests must stay green (the refactor is behavior-preserving under the default layout).
- [ ] **Step 4: Commit** — `feat(reality-web): listing detail renders through resolved layout sections`

---

### Task 7: Frontend workspace gates + docs

**Files:**
- Modify: `docs/repo-map.md` (extend the layout bullet with the frontend pointers)
- Modify: screen-map docs (Agent Log entries — see below)
- Test: full frontend gates

- [ ] **Step 1: repo-map** — extend the existing `layout-core` bullet with one more line:

```markdown
  Frontend: `ppt-web/src/features/layout/` + `reality-web/src/lib/layout.ts` +
  `reality-web/src/components/listings/LayoutSections.tsx` (registries, defensive
  renderers, checked-in web manifests for PUT /platform-admin/layout/manifests).
```

- [ ] **Step 2: screen-map protocol** — `grep -rl "dashboard" docs/screens/ppt/ | head` and `grep -rl "listing-detail\|listing_detail" docs/screens/reality/ | head`. For each matching screen doc: append to its Agent Log section: `2026-07-19 — agent: page now renders via resolved-layout section registry (defensive rendering, spec 2026-07-19-layout-content-manager-design).` Do not change frontmatter statuses. If no matching doc exists for a screen, skip it and note that in the report.

- [ ] **Step 3: full gates** — `cd frontend && pnpm install && pnpm check && pnpm typecheck && pnpm test`. All green; `pnpm check:fix` any Biome fallout on new files and include it.

- [ ] **Step 4: Commit** — `docs(repo-map): layout defensive-rendering pointers + screen-map log entries`

---

## Deliberate scope decisions (do not "fix" during implementation)

- **Mobile (RN + KMP) rendering is NOT in this plan** — spec §9 step 6.
- **No ISR webhook on publish yet.** reality-web relies on `revalidate: 60`; the on-publish `revalidateTag` webhook lands with the editor plan (which also adds the api-server→reality-web signing plumbing).
- **Manifests are checked-in JSON + a consistency test; uploading them to the backend (`PUT /platform-admin/layout/manifests`) is an ops step** for the editor plan / dev-stack seeding — not automated here.
- **`DashboardStats` keeps its mock values** — real stat wiring is the dashboard feature's own backlog, not layout work.
- **`props`/`mode` are passed through but unused by current sections** — display modes get exercised when a mode-bearing section (e.g. listings grid) joins the system.

## Out of scope (subsequent plans)

1. Superadmin editor MVP (admin-web) — includes manifest upload UX + publish webhook.
2. Tenant editor + rails authoring.
3. Preview bridge; mobile registries/renderers.
