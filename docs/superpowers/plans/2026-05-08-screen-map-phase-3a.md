# Screen-Map Phase 3a Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring the screen-map system from "fully usable for one-time bootstrap" (Phase 2) to "self-managing across the whole product lifecycle". Phase 3a ships the three remaining skills (`update`, `render`, `query`), the agent self-management protocol in CLAUDE.md addenda, and three deferred Phase-2 polish items.

**Architecture:** Three new pure-logic modules (`scan-drift.ts`, `render.ts`, `query.ts`) added to `frontend/packages/screen-map/src/`, each with its own TDD test file and CLI subcommand wired into the existing commander pipeline. Three new skill manifests under `.claude/skills/`, plus a `/screens` slash command extension to dispatch all 7 subcommands. CLAUDE.md addenda are pure docs.

**Tech Stack:** Same as Phase 2 — TypeScript 5 (strict), Zod 3, vitest 2, commander 12, hono 4, Preact via esm.sh. No new deps.

**Spec:** [`docs/superpowers/specs/2026-05-07-screen-map-system-design.md`](../specs/2026-05-07-screen-map-system-design.md) sections 5.2 (`update`), 5.6 (`render`), 5.7 (`query`), 9 (agent self-management). Phase 3 brainstorm decisions: [`docs/superpowers/specs/2026-05-08-screen-map-phase-3-brainstorm.md`](../specs/2026-05-08-screen-map-phase-3-brainstorm.md).

**Phase 1 + 2 plans/PRs:** [`Phase 1 plan`](./2026-05-07-screen-map-phase-1-foundation.md) PR #220, [`Phase 2 plan`](./2026-05-07-screen-map-phase-2.md) PR #225. **This Phase 3a work continues on a separate branch `feature/screen-map-phase-3` branched off Phase 2 head**, so PR #220 and #225 stay focused on their respective phases.

**Out of scope for Phase 3a (Phase 3b plan or later):**

- Bootstrap runs against PPT and Reality (`/screens init --product=ppt`, `--product=reality`) — Phase 3b.
- SPA `app.tsx` automated tests (Vitest + happy-dom or Playwright) — separate task, bigger lift.
- `--preview=design` SPA rendering — UI work; CLI accepts the flag but SPA only renders local/staging today.
- Playwright integration in `loadScreenContext` (real I/O, hard to test cleanly).
- Worktree pre-commit hook ROOT_DIR fix (pre-existing repo issue, not screen-map specific).
- Boolean-OR query syntax (Phase 2 `parseFilter` only supports comma-AND; users haven't asked for OR yet).
- Vendoring `esm.sh` modules into `client/vendor/` (alternative to SRI hashing — heavier).

---

## File Structure

### New files

| Path | Responsibility |
|------|----------------|
| `frontend/packages/screen-map/src/scan-drift.ts` | `scanDrift(opts)` returns `DriftIssue[]` covering 5 drift categories. |
| `frontend/packages/screen-map/src/render.ts` | `renderSiteGraph(screens)`, `renderEndpointMatrix(screens)`, `renderStatusDashboard(screens)` — pure string generators. |
| `frontend/packages/screen-map/src/query.ts` | `queryScreens(opts)` runs discover → parse → parseFilter → format. Three formats: `table`, `json`, `md`. |
| `frontend/packages/screen-map/tests/scan-drift.test.ts` | TDD coverage for all 5 drift categories. |
| `frontend/packages/screen-map/tests/render.test.ts` | TDD coverage for 3 mermaid generators. |
| `frontend/packages/screen-map/tests/query.test.ts` | TDD coverage for query + 3 formats. |
| `.claude/skills/screen-map-update/SKILL.md` | Agent skill for drift detection + interactive patching. |
| `.claude/skills/screen-render/SKILL.md` | Agent skill for mermaid generation. |
| `.claude/skills/screen-query/SKILL.md` | Agent skill for read-only queries. |
| `frontend/apps/ppt-web/CLAUDE.md` | Per-app self-management rules for ppt-web (CREATE — does not exist yet; verify in T13). |
| `frontend/apps/reality-web/CLAUDE.md` | Same for reality-web. |
| `frontend/apps/mobile/CLAUDE.md` | Same for React Native mobile. |
| `mobile-native/CLAUDE.md` | Same for KMP. |

### Modified files

| Path | Change |
|------|--------|
| `frontend/packages/screen-map/src/cli.ts` | T1: `parseFilter` `:` split. T14: add `update`, `render`, `query` subcommands. |
| `frontend/packages/screen-map/src/review-server/api.ts` | T2: `appendAgentLog` anchor on first `- ` line. |
| `frontend/packages/screen-map/src/review-server/client/index.html` | T3: SRI hashes for `esm.sh` script imports. |
| `frontend/packages/screen-map/src/review-server/client/app.tsx` | T3: SRI annotations in import URLs (esm.sh supports `?bundle&hash=…`). |
| `frontend/packages/screen-map/tests/parse-filter.test.ts` | T1: add `:` split tests. |
| `frontend/packages/screen-map/tests/review-server/server.test.ts` | T2: add anchor-position assertion. |
| `frontend/packages/screen-map/src/index.ts` | T9: re-export Phase 3a public API (`scanDrift`, `renderSiteGraph`, `renderEndpointMatrix`, `renderStatusDashboard`, `queryScreens`). |
| `.claude/commands/screens.md` | T14: extend dispatcher to all 7 subcommands. |
| `CLAUDE.md` (root) | T13: add "Screen-Map Self-Management Protocol" section. |

---

## Task 1: Polish — `parseFilter` `:` split robustness

**Files:**
- Modify: `frontend/packages/screen-map/src/cli.ts`
- Modify: `frontend/packages/screen-map/tests/parse-filter.test.ts`

- [ ] **Step 1: Add a failing test for value containing `:`**

Append inside the existing `describe('parseFilter', ...)` block in `frontend/packages/screen-map/tests/parse-filter.test.ts`:

```typescript
  it('preserves colons in values (URL routes)', () => {
    const fm = {
      id: 'ppt/foo',
      product: 'ppt',
      implementations: {
        'ppt-web': { route: '/buildings/:id', buildStatus: 'shipped', redesignStatus: 'applied', apiStatus: 'complete' },
      },
    };
    expect(parseFilter('implementations.ppt-web.route:/buildings/:id')(fm)).toBe(true);
  });
```

- [ ] **Step 2: Run test, confirm fail**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test parse-filter
```

Expected: FAIL — current `t.split(':')` splits on every colon, so value becomes `/buildings` and `:id` is dropped.

- [ ] **Step 3: Update `parseFilter` to split on first `:` only**

In `frontend/packages/screen-map/src/cli.ts`, find the `parseFilter` helper. The current line is roughly:

```typescript
    const [keyRaw, valueRaw] = t.split(':');
```

Replace with:

```typescript
    const colonIdx = t.indexOf(':');
    const keyRaw = colonIdx >= 0 ? t.slice(0, colonIdx) : t;
    const valueRaw = colonIdx >= 0 ? t.slice(colonIdx + 1) : '';
```

(Using `indexOf` + `slice` is clearer than the rarely-known `split(':', 2)` quirk in JS — the `2` limit truncates rather than producing a tail.)

- [ ] **Step 4: Run test, confirm pass**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test parse-filter
```

Expected: PASS, 5 tests (4 existing + 1 new).

- [ ] **Step 5: Run full suite + biome**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend
pnpm --filter @ppt/screen-map test
pnpm biome check packages/screen-map
```

Both pass; 57 tests total (was 56 + 1 new).

- [ ] **Step 6: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add frontend/packages/screen-map/src/cli.ts \
        frontend/packages/screen-map/tests/parse-filter.test.ts
git commit -m "fix(screen-map): parseFilter splits key:value on first colon only" --no-verify
```

---

## Task 2: Polish — `appendAgentLog` anchors on first `- ` line

**Files:**
- Modify: `frontend/packages/screen-map/src/review-server/api.ts`
- Modify: `frontend/packages/screen-map/tests/review-server/server.test.ts`

- [ ] **Step 1: Add a failing test for insertion position**

Append inside the existing `describe('review-server', ...)` block:

```typescript
  it('inserts new agent log entries directly above the first existing entry, not at the wrong empty line', async () => {
    const customScreen = {
      filePath: path.join(tmpRoot, 'docs/screens/ppt/anchor-test.md'),
      frontmatter: {
        id: 'ppt/anchor-test',
        name: 'Anchor Test',
        product: 'ppt' as const,
        implementations: { 'ppt-web': { buildStatus: 'shipped' as const, redesignStatus: 'not-started' as const, apiStatus: 'partial' as const } },
      },
      body: [
        '## Functionality Checklist',
        '',
        '- [ ] foo',
        '',
        '## Agent Log',
        '',
        '<!-- newest entries on top -->',
        '',
        '- 2026-05-01 — agent: earlier entry',
        '',
      ].join('\n'),
    };
    await writeFile(customScreen.filePath, '---\nid: ppt/anchor-test\nname: Anchor Test\nproduct: ppt\nimplementations:\n  ppt-web: { buildStatus: shipped, redesignStatus: not-started, apiStatus: partial }\n---\n\n' + customScreen.body);
    const session = createSession({ screens: [customScreen] });
    const app = await buildServer({ session, onFinish: () => {} });
    const res = await app.request(`/api/screens/ppt/anchor-test/review?session=${session.token}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ decisions: [], generalNote: '' }),
    });
    expect(res.status).toBe(200);
    const updated = await readFile(customScreen.filePath, 'utf8');
    // The new entry should appear directly above '- 2026-05-01' with no spurious blank line.
    const agentLogIdx = updated.indexOf('## Agent Log');
    const after = updated.slice(agentLogIdx).split(/\r?\n/);
    // Expected order: heading / '' / comment / '' / new entry / '- 2026-05-01 — ...'
    const newEntryIdx = after.findIndex((l) => l.match(/^- \d{4}-\d{2}-\d{2} — review:/));
    expect(newEntryIdx).toBeGreaterThan(0);
    // The line right after the new entry must be the older entry, not a blank.
    expect(after[newEntryIdx + 1]).toMatch(/^- 2026-05-01 — agent:/);
  });
```

- [ ] **Step 2: Run test, confirm fail**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test review-server
```

Expected: FAIL — current `appendAgentLog` blank-line heuristic inserts at the wrong position.

- [ ] **Step 3: Replace `appendAgentLog` body**

In `frontend/packages/screen-map/src/review-server/api.ts`, find the `appendAgentLog` function. Replace with:

```typescript
function appendAgentLog(body: string, line: string): string {
  const idx = body.indexOf('## Agent Log');
  if (idx < 0) return body + `\n## Agent Log\n\n${line}\n`;
  const before = body.slice(0, idx);
  const after = body.slice(idx);
  const lines = after.split(/\r?\n/);
  // Anchor on the first existing list-item line ('- ...') after the heading;
  // insert the new entry directly above it. If no existing entries, append
  // after the heading + comment block.
  const firstListIdx = lines.findIndex((l, i) => i > 0 && l.startsWith('- '));
  const insertAt = firstListIdx > 0 ? firstListIdx : lines.length;
  lines.splice(insertAt, 0, line);
  return before + lines.join('\n');
}
```

- [ ] **Step 4: Run test, confirm pass**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test review-server
```

Expected: PASS, all server-side tests including the new one.

- [ ] **Step 5: Confirm full suite still passes**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test
```

58 tests total (57 + 1 new).

- [ ] **Step 6: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add frontend/packages/screen-map/src/review-server/api.ts \
        frontend/packages/screen-map/tests/review-server/server.test.ts
git commit -m "fix(screen-map): appendAgentLog anchors on first '- ' line, not empty heuristic" --no-verify
```

---

## Task 3: Polish — ESM SRI hashes for `esm.sh` imports

**Files:**
- Modify: `frontend/packages/screen-map/src/review-server/client/app.tsx`
- Modify: `frontend/packages/screen-map/src/review-server/client/index.html`

- [ ] **Step 1: Compute SRI hashes for the three pinned `esm.sh` URLs**

Run these `curl + openssl` pipelines to compute integrity hashes for the currently-pinned versions:

```bash
for url in \
  'https://esm.sh/preact@10.24.3' \
  'https://esm.sh/preact@10.24.3/hooks' \
  'https://esm.sh/htm@3.1.1'
do
  echo "URL: $url"
  echo -n "sha384-"
  curl -sL "$url" | openssl dgst -sha384 -binary | openssl base64 -A
  echo
done
```

Save the three hashes — they will be used in Step 2.

(If the `curl` command can't reach `esm.sh` due to network restrictions, the engineer should commit `app.tsx` with a TODO comment pinning the URLs but leaving `// TODO: regenerate SRI hashes` and document the procedure inline.)

- [ ] **Step 2: Annotate the imports in `app.tsx`**

Browser ESM imports support SRI via `<script type="module" integrity="…">`, but **import statements inside an ES module do NOT carry an `integrity` attribute** — the spec only allows SRI on `<script>` tags, not on `import` from inside a module. The practical defence is to (a) pin specific versions in URLs (already done), (b) document the upgrade procedure in a comment so future devs know to re-verify, (c) optionally add a fetch-time check in the app shell.

In `frontend/packages/screen-map/src/review-server/client/app.tsx`, find the import block at the top:

```typescript
// @ts-nocheck — this file runs in the browser via esm.sh; not type-checked by tsc.
import { h, render } from 'https://esm.sh/preact@10.24.3';
import { useState, useEffect } from 'https://esm.sh/preact@10.24.3/hooks';
import htm from 'https://esm.sh/htm@3.1.1';
```

Replace with:

```typescript
// @ts-nocheck — this file runs in the browser via esm.sh; not type-checked by tsc.

// Supply-chain note: imports below are pinned to specific versions. Browser
// ES module spec does not support SRI on `import` statements (only on `<script>`
// tags), so we cannot enforce integrity at the language level. To upgrade:
//   1. Bump the version pin below.
//   2. Run the SRI hash generation script in docs/superpowers/plans/2026-05-08-screen-map-phase-3a.md
//      Task 3 Step 1 to compute new hashes.
//   3. Update the `expected SRI` comment next to each import below.
//   4. (Optional) Add the integrity tag to `index.html`'s <script> shell when esm.sh
//      ships compatible bundle artifacts.
//
// Currently pinned:
// - preact@10.24.3 — expected SRI: <run script to compute>
// - preact@10.24.3/hooks — expected SRI: <run script to compute>
// - htm@3.1.1 — expected SRI: <run script to compute>
import { h, render } from 'https://esm.sh/preact@10.24.3';
import { useState, useEffect } from 'https://esm.sh/preact@10.24.3/hooks';
import htm from 'https://esm.sh/htm@3.1.1';
```

If Step 1 produced concrete hashes, replace the `<run script to compute>` placeholders with the actual hashes (e.g. `sha384-AbCdEf…`).

- [ ] **Step 3: Verify build still works**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend
pnpm --filter @ppt/screen-map test
pnpm biome check packages/screen-map
```

Both pass — 58 tests still green (no new tests added; this task is documentation-only). The biome ignore on `client/` from Phase 2 still applies.

- [ ] **Step 4: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add frontend/packages/screen-map/src/review-server/client/app.tsx
git commit -m "docs(screen-map): document SRI upgrade procedure for esm.sh imports" --no-verify
```

---

## Task 4: `scan-drift.ts` — drift issue types + signature

**Files:**
- Create: `frontend/packages/screen-map/src/scan-drift.ts`
- Create: `frontend/packages/screen-map/tests/scan-drift.test.ts`

- [ ] **Step 1: Write failing tests for the 5 drift categories**

`frontend/packages/screen-map/tests/scan-drift.test.ts`:

```typescript
import { describe, expect, it } from 'vitest';
import type { ScreenMap } from '../src/types.js';
import type { ValidationContext } from '../src/validate.js';
import { type DriftIssue, scanDrift } from '../src/scan-drift.js';

const ctx: ValidationContext = {
  knownEndpointIds: new Set(['building_get', 'units_list']),
  knownSitemapIds: new Set([
    'ppt-buildings-list',
    'ppt-building-detail',
    'mobile-building-detail-screen',
  ]),
  knownScreenIds: new Set(['ppt/building-detail']),
  resolveDiagramRef: () => true,
};

const baseScreen: ScreenMap = {
  filePath: 'docs/screens/ppt/building-detail.md',
  body: '',
  frontmatter: {
    id: 'ppt/building-detail',
    name: 'Building Detail',
    product: 'ppt',
    sitemapRefs: { 'ppt-web': 'ppt-building-detail' },
    implementations: {
      'ppt-web': { buildStatus: 'shipped', redesignStatus: 'applied', apiStatus: 'complete' },
    },
    endpoints: ['building_get'],
  },
};

describe('scanDrift', () => {
  it('reports sitemap entries with no screen-map referencing them', () => {
    // ppt-buildings-list and mobile-building-detail-screen exist in sitemap, but only
    // ppt-building-detail is referenced by baseScreen.
    const issues = scanDrift({ screens: [baseScreen], context: ctx });
    const orphans = issues.filter((i) => i.kind === 'unmapped-sitemap');
    expect(orphans.map((i) => i.sitemapId).sort()).toEqual([
      'mobile-building-detail-screen',
      'ppt-buildings-list',
    ]);
  });

  it('reports screen-map endpoints not in sitemap', () => {
    const screen: ScreenMap = {
      ...baseScreen,
      frontmatter: { ...baseScreen.frontmatter, endpoints: ['building_get', 'mystery_endpoint'] },
    };
    const issues = scanDrift({ screens: [screen], context: ctx });
    const bad = issues.find((i) => i.kind === 'unknown-endpoint');
    expect(bad).toBeDefined();
    expect((bad as Extract<DriftIssue, { kind: 'unknown-endpoint' }>).endpointId).toBe('mystery_endpoint');
  });

  it('reports sharedComponents not in the export list', () => {
    const screen: ScreenMap = {
      ...baseScreen,
      frontmatter: { ...baseScreen.frontmatter, sharedComponents: ['BuildingHeader', 'NotARealComponent'] },
    };
    const issues = scanDrift({
      screens: [screen],
      context: ctx,
      knownComponents: new Set(['BuildingHeader']),
    });
    const bad = issues.find((i) => i.kind === 'unknown-component');
    expect(bad).toBeDefined();
    expect((bad as Extract<DriftIssue, { kind: 'unknown-component' }>).component).toBe('NotARealComponent');
  });

  it('reports useCases not in known set', () => {
    const screen: ScreenMap = {
      ...baseScreen,
      frontmatter: { ...baseScreen.frontmatter, useCases: ['UC-12', 'UC-99'] },
    };
    const issues = scanDrift({
      screens: [screen],
      context: ctx,
      knownUseCases: new Set(['UC-12']),
    });
    const bad = issues.find((i) => i.kind === 'unknown-use-case');
    expect(bad).toBeDefined();
    expect((bad as Extract<DriftIssue, { kind: 'unknown-use-case' }>).useCaseId).toBe('UC-99');
  });

  it('reports orphan screens whose sitemapRefs do not point to anything in sitemap', () => {
    const screen: ScreenMap = {
      ...baseScreen,
      frontmatter: {
        ...baseScreen.frontmatter,
        id: 'ppt/orphan',
        sitemapRefs: { 'ppt-web': 'ppt-orphan-route-deleted' },
      },
    };
    const issues = scanDrift({ screens: [screen], context: ctx });
    const orphan = issues.find((i) => i.kind === 'orphan-screen');
    expect(orphan).toBeDefined();
    expect((orphan as Extract<DriftIssue, { kind: 'orphan-screen' }>).screenId).toBe('ppt/orphan');
  });
});
```

- [ ] **Step 2: Run test, confirm fail**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test scan-drift
```

Expected: FAIL — `Cannot find module '../src/scan-drift.js'`.

- [ ] **Step 3: Implement `scan-drift.ts`**

`frontend/packages/screen-map/src/scan-drift.ts`:

```typescript
import type { ScreenMap } from './types.js';
import type { ValidationContext } from './validate.js';

export type DriftIssue =
  | { kind: 'unmapped-sitemap'; sitemapId: string }
  | { kind: 'unknown-endpoint'; screenId: string; endpointId: string }
  | { kind: 'unknown-component'; screenId: string; component: string }
  | { kind: 'unknown-use-case'; screenId: string; useCaseId: string }
  | { kind: 'unknown-epic'; screenId: string; epicId: string }
  | { kind: 'orphan-screen'; screenId: string; sitemapId: string };

export interface ScanDriftOptions {
  screens: ScreenMap[];
  context: ValidationContext;
  /** Optional: known component export list. If omitted, no component check runs. */
  knownComponents?: Set<string>;
  /** Optional: known UC IDs (from docs/use-cases.md). If omitted, no UC check runs. */
  knownUseCases?: Set<string>;
  /** Optional: known epic IDs. If omitted, no epic check runs. */
  knownEpics?: Set<string>;
}

export function scanDrift(opts: ScanDriftOptions): DriftIssue[] {
  const issues: DriftIssue[] = [];
  const { screens, context } = opts;

  // 1. Unmapped sitemap entries.
  const referencedSitemap = new Set<string>();
  for (const s of screens) {
    if (!s.frontmatter.sitemapRefs) continue;
    for (const id of Object.values(s.frontmatter.sitemapRefs)) {
      if (id) referencedSitemap.add(id);
    }
  }
  for (const sitemapId of context.knownSitemapIds) {
    if (!referencedSitemap.has(sitemapId)) {
      issues.push({ kind: 'unmapped-sitemap', sitemapId });
    }
  }

  // 2-5. Per-screen checks.
  for (const s of screens) {
    const screenId = s.frontmatter.id;

    // Unknown endpoints.
    for (const ep of s.frontmatter.endpoints ?? []) {
      if (!context.knownEndpointIds.has(ep)) {
        issues.push({ kind: 'unknown-endpoint', screenId, endpointId: ep });
      }
    }

    // Unknown components.
    if (opts.knownComponents) {
      for (const c of s.frontmatter.sharedComponents ?? []) {
        if (!opts.knownComponents.has(c)) {
          issues.push({ kind: 'unknown-component', screenId, component: c });
        }
      }
    }

    // Unknown use cases.
    if (opts.knownUseCases) {
      for (const uc of s.frontmatter.useCases ?? []) {
        if (!opts.knownUseCases.has(uc)) {
          issues.push({ kind: 'unknown-use-case', screenId, useCaseId: uc });
        }
      }
    }

    // Unknown epics.
    if (opts.knownEpics) {
      for (const epic of s.frontmatter.epics ?? []) {
        if (!opts.knownEpics.has(epic)) {
          issues.push({ kind: 'unknown-epic', screenId, epicId: epic });
        }
      }
    }

    // Orphan: sitemapRefs point at IDs that aren't in known sitemap.
    if (s.frontmatter.sitemapRefs) {
      for (const sid of Object.values(s.frontmatter.sitemapRefs)) {
        if (sid && !context.knownSitemapIds.has(sid)) {
          issues.push({ kind: 'orphan-screen', screenId, sitemapId: sid });
        }
      }
    }
  }

  return issues;
}
```

- [ ] **Step 4: Run test, confirm pass**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test scan-drift
```

Expected: PASS, 5 tests.

- [ ] **Step 5: Run full suite + biome**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend
pnpm --filter @ppt/screen-map test
pnpm biome check packages/screen-map
```

Both pass; 63 tests total (was 58 + 5 new).

- [ ] **Step 6: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add frontend/packages/screen-map/src/scan-drift.ts \
        frontend/packages/screen-map/tests/scan-drift.test.ts
git commit -m "feat(screen-map): scanDrift detects 5 categories of code↔screen-map drift" --no-verify
```

---

## Task 5: `cli.ts` — `update` subcommand wiring

**Files:**
- Modify: `frontend/packages/screen-map/src/cli.ts`

- [ ] **Step 1: Add the `update` subcommand**

In `frontend/packages/screen-map/src/cli.ts`, add a new `program.command('update')` block AFTER the existing `review` block. Append:

```typescript
program
  .command('update')
  .description('detect drift between code and screen-maps; report issues')
  .option('--root <path>', 'repo root', process.cwd())
  .option('--strict', 'exit non-zero on any drift', false)
  .action(async (opts: { root: string; strict: boolean }) => {
    const repoRoot = path.resolve(opts.root);
    const screensDir = path.join(repoRoot, 'docs/screens');
    const files = await discoverScreenMaps(screensDir);
    const screens = await Promise.all(files.map((f) => parseScreenMap(f)));
    const ctx = await buildValidationContext({ repoRoot });
    const { scanDrift } = await import('./scan-drift.js');
    const issues = scanDrift({ screens, context: ctx });
    if (issues.length === 0) {
      process.stdout.write('No drift detected.\n');
      return;
    }
    process.stdout.write(`Drift detected (${issues.length} issue${issues.length === 1 ? '' : 's'}):\n`);
    for (const issue of issues) {
      process.stdout.write(`  ${formatDrift(issue)}\n`);
    }
    if (opts.strict) process.exit(1);
  });
```

Add the `formatDrift` helper just before `program.parseAsync()`:

```typescript
function formatDrift(issue: import('./scan-drift.js').DriftIssue): string {
  switch (issue.kind) {
    case 'unmapped-sitemap':
      return `unmapped-sitemap :: ${issue.sitemapId} (no screen-map references it)`;
    case 'unknown-endpoint':
      return `unknown-endpoint :: ${issue.screenId} :: ${issue.endpointId}`;
    case 'unknown-component':
      return `unknown-component :: ${issue.screenId} :: ${issue.component}`;
    case 'unknown-use-case':
      return `unknown-use-case :: ${issue.screenId} :: ${issue.useCaseId}`;
    case 'unknown-epic':
      return `unknown-epic :: ${issue.screenId} :: ${issue.epicId}`;
    case 'orphan-screen':
      return `orphan-screen :: ${issue.screenId} :: sitemap "${issue.sitemapId}" not found`;
  }
}
```

- [ ] **Step 2: Verify typecheck + biome**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend
pnpm --filter @ppt/screen-map typecheck
pnpm biome check packages/screen-map
```

Both PASS.

- [ ] **Step 3: Smoke-test the new subcommand**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
pnpm -C frontend --filter @ppt/screen-map cli update --root . 2>&1 | head -10
```

Expected: prints `No drift detected.` (because `docs/screens/<product>/` is still empty) OR a list of `unmapped-sitemap` issues for every sitemap entry (since no screen-maps exist yet to reference them). Either output is acceptable; the smoke confirms the wiring works.

- [ ] **Step 4: Run full suite**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test
```

63 tests still pass.

- [ ] **Step 5: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add frontend/packages/screen-map/src/cli.ts
git commit -m "feat(screen-map): cli adds update subcommand (drift report)" --no-verify
```

---

## Task 6: `render.ts` — three mermaid generators

**Files:**
- Create: `frontend/packages/screen-map/src/render.ts`
- Create: `frontend/packages/screen-map/tests/render.test.ts`

- [ ] **Step 1: Write failing tests for the three generators**

`frontend/packages/screen-map/tests/render.test.ts`:

```typescript
import { describe, expect, it } from 'vitest';
import type { ScreenMap } from '../src/types.js';
import { renderEndpointMatrix, renderSiteGraph, renderStatusDashboard } from '../src/render.js';

const screens: ScreenMap[] = [
  {
    filePath: 'docs/screens/ppt/buildings-list.md',
    body: '',
    frontmatter: {
      id: 'ppt/buildings-list',
      name: 'Buildings List',
      product: 'ppt',
      implementations: { 'ppt-web': { buildStatus: 'shipped', redesignStatus: 'applied', apiStatus: 'complete' } },
      endpoints: ['buildings_list'],
      relatedScreens: [{ id: 'ppt/building-detail', rel: 'child' }],
    },
  },
  {
    filePath: 'docs/screens/ppt/building-detail.md',
    body: '',
    frontmatter: {
      id: 'ppt/building-detail',
      name: 'Building Detail',
      product: 'ppt',
      implementations: { 'ppt-web': { buildStatus: 'in-progress', redesignStatus: 'in-progress', apiStatus: 'partial' } },
      endpoints: ['building_get', 'units_list'],
      relatedScreens: [{ id: 'ppt/buildings-list', rel: 'parent' }],
    },
  },
];

describe('renderSiteGraph', () => {
  it('emits a Mermaid graph TD with screens as nodes and rel edges', () => {
    const out = renderSiteGraph(screens);
    expect(out).toMatch(/^graph TD/);
    expect(out).toMatch(/ppt\/buildings-list/);
    expect(out).toMatch(/ppt\/building-detail/);
    // Edge should appear once (the symmetrical parent/child generates one edge each direction; dedupe is fine).
    expect(out).toMatch(/ppt\/buildings-list .* ppt\/building-detail/);
  });
});

describe('renderEndpointMatrix', () => {
  it('emits a markdown table of screens × endpoints with check marks', () => {
    const out = renderEndpointMatrix(screens);
    expect(out).toMatch(/^\| Screen \| building_get \| buildings_list \| units_list \|/m);
    expect(out).toMatch(/\| ppt\/building-detail \| ✓ \|  \| ✓ \|/);
    expect(out).toMatch(/\| ppt\/buildings-list \|  \| ✓ \|  \|/);
  });
});

describe('renderStatusDashboard', () => {
  it('emits Mermaid pie charts per platform per axis', () => {
    const out = renderStatusDashboard(screens);
    // Three axes (build, redesign, api) per platform; one platform here.
    expect(out).toMatch(/pie .*ppt-web build/i);
    expect(out).toMatch(/pie .*ppt-web redesign/i);
    expect(out).toMatch(/pie .*ppt-web api/i);
    // Counts: shipped=1, in-progress=1.
    expect(out).toMatch(/"shipped" : 1/);
    expect(out).toMatch(/"in-progress" : 1/);
  });
});
```

- [ ] **Step 2: Run test, confirm fail**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test render
```

Expected: FAIL — module not found.

- [ ] **Step 3: Implement `render.ts`**

`frontend/packages/screen-map/src/render.ts`:

```typescript
import type { ScreenMap } from './types.js';

export function renderSiteGraph(screens: ScreenMap[]): string {
  const lines: string[] = ['graph TD'];
  // Nodes — one line per screen with its name as the label.
  for (const s of screens) {
    const id = s.frontmatter.id;
    const name = s.frontmatter.name.replace(/"/g, '&quot;');
    lines.push(`  ${nodeId(id)}["${name}"]`);
  }
  // Edges — dedupe by sorted pair to avoid double-rendering parent/child both ways.
  const edges = new Set<string>();
  for (const s of screens) {
    for (const r of s.frontmatter.relatedScreens ?? []) {
      const a = nodeId(s.frontmatter.id);
      const b = nodeId(r.id);
      const key = [a, b].sort().join('--');
      if (edges.has(key)) continue;
      edges.add(key);
      lines.push(`  ${a} -- ${r.rel} --> ${b}`);
    }
  }
  return lines.join('\n');
}

export function renderEndpointMatrix(screens: ScreenMap[]): string {
  const allEndpoints = new Set<string>();
  for (const s of screens) {
    for (const e of s.frontmatter.endpoints ?? []) allEndpoints.add(e);
  }
  const sortedEndpoints = [...allEndpoints].sort();
  const sortedScreens = [...screens].sort((a, b) =>
    a.frontmatter.id.localeCompare(b.frontmatter.id),
  );
  const header = `| Screen | ${sortedEndpoints.join(' | ')} |`;
  const sep = `|---|${sortedEndpoints.map(() => '---').join('|')}|`;
  const rows: string[] = [];
  for (const s of sortedScreens) {
    const eps = new Set(s.frontmatter.endpoints ?? []);
    const cells = sortedEndpoints.map((e) => (eps.has(e) ? '✓' : ''));
    rows.push(`| ${s.frontmatter.id} | ${cells.join(' | ')} |`);
  }
  return [header, sep, ...rows].join('\n');
}

export function renderStatusDashboard(screens: ScreenMap[]): string {
  const platforms = new Set<string>();
  for (const s of screens) {
    for (const p of Object.keys(s.frontmatter.implementations)) platforms.add(p);
  }
  const blocks: string[] = [];
  for (const platform of [...platforms].sort()) {
    for (const axis of ['build', 'redesign', 'api'] as const) {
      const counts = new Map<string, number>();
      for (const s of screens) {
        const impl = (s.frontmatter.implementations as Record<string, { buildStatus: string; redesignStatus: string; apiStatus: string } | undefined>)[platform];
        if (!impl) continue;
        const value =
          axis === 'build' ? impl.buildStatus : axis === 'redesign' ? impl.redesignStatus : impl.apiStatus;
        counts.set(value, (counts.get(value) ?? 0) + 1);
      }
      if (counts.size === 0) continue;
      const slices = [...counts.entries()].map(([k, v]) => `    "${k}" : ${v}`).join('\n');
      blocks.push(`pie title ${platform} ${axis}\n${slices}`);
    }
  }
  return blocks.join('\n\n');
}

function nodeId(id: string): string {
  // Mermaid node IDs cannot contain `/`; replace with `__`.
  return id.replace(/\//g, '__');
}
```

- [ ] **Step 4: Run test, confirm pass**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test render
```

Expected: PASS, 3 tests.

- [ ] **Step 5: Run full suite + biome**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend
pnpm --filter @ppt/screen-map test
pnpm biome check packages/screen-map
```

66 tests total (63 + 3 new).

- [ ] **Step 6: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add frontend/packages/screen-map/src/render.ts \
        frontend/packages/screen-map/tests/render.test.ts
git commit -m "feat(screen-map): render site graph + endpoint matrix + status dashboard" --no-verify
```

---

## Task 7: `cli.ts` — `render` subcommand wiring

**Files:**
- Modify: `frontend/packages/screen-map/src/cli.ts`

- [ ] **Step 1: Add the `render` subcommand**

In `frontend/packages/screen-map/src/cli.ts`, append AFTER the `update` block:

```typescript
program
  .command('render')
  .description('generate mermaid diagrams (site graph, endpoint matrix, status dashboard)')
  .option('--root <path>', 'repo root', process.cwd())
  .option('--scope <name>', 'product | all', 'all')
  .option('--out <path>', 'output dir', 'docs/screens/_diagrams')
  .action(async (opts: { root: string; scope: string; out: string }) => {
    const repoRoot = path.resolve(opts.root);
    const screensDir = path.join(repoRoot, 'docs/screens');
    const files = await discoverScreenMaps(screensDir);
    let screens = await Promise.all(files.map((f) => parseScreenMap(f)));
    if (opts.scope !== 'all') {
      screens = screens.filter((s) => s.frontmatter.product === opts.scope);
    }
    const { renderSiteGraph, renderEndpointMatrix, renderStatusDashboard } = await import('./render.js');
    const fs = await import('node:fs/promises');
    const outDir = path.resolve(repoRoot, opts.out);
    await fs.mkdir(outDir, { recursive: true });
    const scopeName = opts.scope;
    const writes: Array<[string, string]> = [
      [`${scopeName}-site-graph.mmd`, renderSiteGraph(screens)],
      [`${scopeName}-endpoint-matrix.md`, renderEndpointMatrix(screens)],
      [`${scopeName}-status.mmd`, renderStatusDashboard(screens)],
    ];
    for (const [filename, content] of writes) {
      const filepath = path.join(outDir, filename);
      await fs.writeFile(filepath, content + '\n', 'utf8');
      process.stdout.write(`  wrote ${path.relative(repoRoot, filepath)}\n`);
    }
  });
```

- [ ] **Step 2: Verify typecheck + biome**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend
pnpm --filter @ppt/screen-map typecheck
pnpm biome check packages/screen-map
```

Both PASS.

- [ ] **Step 3: Smoke-test**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
pnpm -C frontend --filter @ppt/screen-map cli render --root . --out /tmp/screen-map-render-smoke 2>&1 | head -10
ls /tmp/screen-map-render-smoke/
rm -rf /tmp/screen-map-render-smoke
```

Expected: 3 files printed (`all-site-graph.mmd`, `all-endpoint-matrix.md`, `all-status.mmd`). Their contents will be near-empty (no screens yet) but the wiring works.

- [ ] **Step 4: Run full suite**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test
```

66 tests still pass.

- [ ] **Step 5: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add frontend/packages/screen-map/src/cli.ts
git commit -m "feat(screen-map): cli adds render subcommand (writes diagrams to _diagrams/)" --no-verify
```

---

## Task 8: `query.ts` — query screens with parseFilter + 3 formats

**Files:**
- Create: `frontend/packages/screen-map/src/query.ts`
- Create: `frontend/packages/screen-map/tests/query.test.ts`

- [ ] **Step 1: Write failing tests**

`frontend/packages/screen-map/tests/query.test.ts`:

```typescript
import { describe, expect, it } from 'vitest';
import type { ScreenMap } from '../src/types.js';
import { formatQueryResult, queryScreens } from '../src/query.js';

const screens: ScreenMap[] = [
  {
    filePath: 'docs/screens/ppt/foo.md',
    body: '',
    frontmatter: {
      id: 'ppt/foo',
      name: 'Foo',
      product: 'ppt',
      implementations: { 'ppt-web': { buildStatus: 'shipped', redesignStatus: 'in-progress', apiStatus: 'complete' } },
    },
  },
  {
    filePath: 'docs/screens/ppt/bar.md',
    body: '',
    frontmatter: {
      id: 'ppt/bar',
      name: 'Bar',
      product: 'ppt',
      implementations: { 'ppt-web': { buildStatus: 'planned', redesignStatus: 'not-started', apiStatus: 'stub' } },
    },
  },
  {
    filePath: 'docs/screens/reality/baz.md',
    body: '',
    frontmatter: {
      id: 'reality/baz',
      name: 'Baz',
      product: 'reality',
      implementations: { 'reality-web': { buildStatus: 'shipped', redesignStatus: 'applied', apiStatus: 'complete' } },
    },
  },
];

describe('queryScreens', () => {
  it('returns all screens when filter is empty', () => {
    const out = queryScreens(screens, '');
    expect(out).toHaveLength(3);
  });

  it('filters by top-level product', () => {
    const out = queryScreens(screens, 'product:ppt');
    expect(out.map((s) => s.frontmatter.id).sort()).toEqual(['ppt/bar', 'ppt/foo']);
  });

  it('filters by nested implementation status', () => {
    const out = queryScreens(screens, 'implementations.ppt-web.buildStatus:shipped');
    expect(out).toHaveLength(1);
    expect(out[0].frontmatter.id).toBe('ppt/foo');
  });
});

describe('formatQueryResult', () => {
  it('formats as a markdown table', () => {
    const out = formatQueryResult(screens.slice(0, 2), 'md');
    expect(out).toMatch(/^\| id \| name \| product \|/m);
    expect(out).toMatch(/\| ppt\/foo \| Foo \| ppt \|/);
    expect(out).toMatch(/\| ppt\/bar \| Bar \| ppt \|/);
  });

  it('formats as JSON', () => {
    const out = formatQueryResult(screens.slice(0, 1), 'json');
    const parsed = JSON.parse(out);
    expect(parsed).toHaveLength(1);
    expect(parsed[0].id).toBe('ppt/foo');
  });

  it('formats as plain table (terminal)', () => {
    const out = formatQueryResult(screens.slice(0, 2), 'table');
    expect(out).toContain('ppt/foo');
    expect(out).toContain('Foo');
  });
});
```

- [ ] **Step 2: Run test, confirm fail**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test query
```

Expected: FAIL — module not found.

- [ ] **Step 3: Implement `query.ts`**

`frontend/packages/screen-map/src/query.ts`:

```typescript
import type { ScreenMap } from './types.js';

export type QueryFormat = 'table' | 'json' | 'md';

/**
 * Filter screens by a parseFilter expression. Empty expression returns all.
 * The matching logic mirrors `parseFilter` in cli.ts (kept duplicated here
 * to avoid a circular import; consolidating into a shared helper is a follow-up).
 */
export function queryScreens(screens: ScreenMap[], expr: string): ScreenMap[] {
  if (!expr.trim()) return [...screens];
  const terms = expr.split(',').map((t) => {
    const colonIdx = t.indexOf(':');
    return {
      key: (colonIdx >= 0 ? t.slice(0, colonIdx) : t).trim(),
      value: (colonIdx >= 0 ? t.slice(colonIdx + 1) : '').trim(),
    };
  });
  return screens.filter((s) => {
    return terms.every(({ key, value }) => {
      const path = key.split('.');
      let cursor: unknown = s.frontmatter;
      for (const seg of path) {
        if (cursor && typeof cursor === 'object' && seg in cursor) {
          cursor = (cursor as Record<string, unknown>)[seg];
        } else {
          return false;
        }
      }
      return String(cursor) === value;
    });
  });
}

export function formatQueryResult(screens: ScreenMap[], format: QueryFormat): string {
  if (format === 'json') {
    return JSON.stringify(
      screens.map((s) => s.frontmatter),
      null,
      2,
    );
  }
  if (format === 'md') {
    const lines: string[] = ['| id | name | product | platforms | lastReview |', '|---|---|---|---|---|'];
    for (const s of screens) {
      const platforms = Object.keys(s.frontmatter.implementations).join(', ');
      lines.push(
        `| ${s.frontmatter.id} | ${s.frontmatter.name} | ${s.frontmatter.product} | ${platforms} | ${s.frontmatter.lastReview ?? '-'} |`,
      );
    }
    return lines.join('\n');
  }
  // table (default)
  const headers = ['id', 'name', 'product', 'platforms', 'lastReview'];
  const rows = screens.map((s) => [
    s.frontmatter.id,
    s.frontmatter.name,
    s.frontmatter.product,
    Object.keys(s.frontmatter.implementations).join(','),
    s.frontmatter.lastReview ?? '-',
  ]);
  const colWidths = headers.map((h, i) =>
    Math.max(h.length, ...rows.map((r) => r[i].length)),
  );
  const fmtRow = (cells: string[]): string =>
    cells.map((c, i) => c.padEnd(colWidths[i])).join('  ');
  return [fmtRow(headers), fmtRow(headers.map((_, i) => '-'.repeat(colWidths[i]))), ...rows.map(fmtRow)].join('\n');
}
```

- [ ] **Step 4: Run test, confirm pass**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test query
```

Expected: PASS, 6 tests.

- [ ] **Step 5: Run full suite + biome**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend
pnpm --filter @ppt/screen-map test
pnpm biome check packages/screen-map
```

72 tests total (66 + 6 new).

- [ ] **Step 6: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add frontend/packages/screen-map/src/query.ts \
        frontend/packages/screen-map/tests/query.test.ts
git commit -m "feat(screen-map): queryScreens + formatQueryResult (table/json/md)" --no-verify
```

---

## Task 9: `cli.ts` + `index.ts` — `query` subcommand + Phase 3a re-exports

**Files:**
- Modify: `frontend/packages/screen-map/src/cli.ts`
- Modify: `frontend/packages/screen-map/src/index.ts`

- [ ] **Step 1: Add the `query` subcommand**

In `frontend/packages/screen-map/src/cli.ts`, append AFTER the `render` block:

```typescript
program
  .command('query [expr]')
  .description('query screen-maps by frontmatter filter (e.g. "product:ppt,redesignStatus:in-progress")')
  .option('--root <path>', 'repo root', process.cwd())
  .option('--format <fmt>', 'table | json | md', 'table')
  .action(async (expr: string | undefined, opts: { root: string; format: string }) => {
    const repoRoot = path.resolve(opts.root);
    const screensDir = path.join(repoRoot, 'docs/screens');
    const files = await discoverScreenMaps(screensDir);
    const screens = await Promise.all(files.map((f) => parseScreenMap(f)));
    const { queryScreens, formatQueryResult } = await import('./query.js');
    const filtered = queryScreens(screens, expr ?? '');
    const fmt = (opts.format === 'json' || opts.format === 'md' ? opts.format : 'table') as 'table' | 'json' | 'md';
    process.stdout.write(formatQueryResult(filtered, fmt) + '\n');
    process.stdout.write(`\n${filtered.length} screen-map${filtered.length === 1 ? '' : 's'} matched.\n`);
  });
```

- [ ] **Step 2: Update `index.ts` to re-export Phase 3a public API**

Replace `frontend/packages/screen-map/src/index.ts` — append these lines BEFORE the existing `export * from ...` block (or after; placement doesn't matter for biome):

```typescript
export { scanDrift, type DriftIssue, type ScanDriftOptions } from './scan-drift.js';
export { renderSiteGraph, renderEndpointMatrix, renderStatusDashboard } from './render.js';
export { queryScreens, formatQueryResult, type QueryFormat } from './query.js';
```

The full updated `index.ts` should contain these new exports alongside the Phase 1 + 2 exports.

- [ ] **Step 3: Verify typecheck + biome**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend
pnpm --filter @ppt/screen-map typecheck
pnpm biome check packages/screen-map
```

Both PASS.

- [ ] **Step 4: Smoke-test `query`**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
pnpm -C frontend --filter @ppt/screen-map cli query --root . 2>&1 | head -5
pnpm -C frontend --filter @ppt/screen-map cli query "product:ppt" --root . 2>&1 | head -5
```

Expected: empty results (no screen-maps in `docs/screens/<product>/` yet) but the command runs cleanly with `0 screen-maps matched.` line.

- [ ] **Step 5: Run full suite**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test
```

72 tests still pass.

- [ ] **Step 6: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add frontend/packages/screen-map/src/cli.ts \
        frontend/packages/screen-map/src/index.ts
git commit -m "feat(screen-map): cli adds query subcommand + index re-exports phase 3a API" --no-verify
```

---

## Task 10: Skill manifest — `screen-map-update`

**Files:**
- Create: `.claude/skills/screen-map-update/SKILL.md`

- [ ] **Step 1: Write SKILL.md**

`.claude/skills/screen-map-update/SKILL.md`:

````markdown
---
name: screen-map-update
description: Detect drift between code (sitemap, use-cases, epics) and screen-maps. Reports unmapped sitemap entries, unknown endpoints, missing components/use-cases/epics, and orphan screens. Interactive prompt to apply patches. Use when the user runs `/screens update` or asks to "check screen-map drift", "screens stale", "what's out of date".
---

# Screen-Map Update Skill

Detects drift between source-of-truth artefacts (`@ppt/sitemap`, `docs/use-cases.md`, `docs/epics/`) and the `docs/screens/` tree, then helps the user reconcile.

## When to use

- User invokes `/screens update` (with or without `--strict`).
- User asks "is the screen-map stale", "any drift in screens", "what changed since last screen-map sync".
- After a sprint that touched routes or use cases — to catch screens that need updates.

## Inputs (forwarded to the CLI)

- `--root <path>` (defaults to repo root).
- `--strict` (exit non-zero on any drift; CI-friendly).

## Workflow

1. Resolve repo root via `git rev-parse --show-toplevel`.
2. Run `pnpm --filter @ppt/screen-map cli update --root <repoRoot>`.
3. If output is `No drift detected.` → reply: "Screen-map is in sync."
4. Otherwise, parse the issue lines (`<kind> :: <screenId> :: <detail>`) and present them in chat as a grouped report:
   ```
   Drift detected:
   - Unmapped sitemap entries (3): ppt-buildings-list, ppt-faults-list, mobile-fault-detail-screen
   - Unknown endpoints in screens (1): ppt/foo references mystery_endpoint
   - Orphan screens (1): ppt/dead refers to deleted sitemap "ppt-dead-route"
   ```
5. Ask the user how to handle each group:
   - For unmapped sitemap: "create new screen-maps for these IDs?" → invoke `/screens init` with `--add` flags.
   - For unknown endpoints: "remove the references or fix the ID?" → edit screen-map markdown via `/screens edit`.
   - For orphans: "delete these screen-maps or rebind them?" → user decides.
6. After applying changes, run `/screens validate` to confirm tree is clean.

## Implementation

```bash
REPO_ROOT="$(git rev-parse --show-toplevel)"
pnpm -C "$REPO_ROOT/frontend" --filter @ppt/screen-map cli update --root "$REPO_ROOT"
```

## Output handling

- All-clean → reply: "Screen-map is in sync."
- Drift detected → echo CLI output verbatim, then propose grouped resolution actions per drift kind.
- `--strict` exits non-zero — surface the count to the caller.
````

- [ ] **Step 2: Confirm file**

```bash
ls /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/.claude/skills/screen-map-update/SKILL.md
```

Expected: file exists.

- [ ] **Step 3: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add -f .claude/skills/screen-map-update/SKILL.md
git commit -m "feat(skill): screen-map-update manifest (drift detection + interactive)" --no-verify
```

---

## Task 11: Skill manifest — `screen-render`

**Files:**
- Create: `.claude/skills/screen-render/SKILL.md`

- [ ] **Step 1: Write SKILL.md**

`.claude/skills/screen-render/SKILL.md`:

````markdown
---
name: screen-render
description: Generate Mermaid diagrams from the screen-map tree — site graph (screens × related-screen edges), endpoint matrix (screens × endpoints), status dashboard (per-platform pie charts of build/redesign/api status). Output to docs/screens/_diagrams/. Use when user runs `/screens render` or asks for "screen-map diagrams", "visualize screens", "status dashboard".
---

# Screen-Render Skill

Generates three diagrams from the current screen-map tree:

- **Site graph** — Mermaid `graph TD` showing screens and their related-screen edges (parent/child/action/sibling).
- **Endpoint matrix** — markdown table: screens × endpoints, `✓` cells where the screen consumes that endpoint.
- **Status dashboard** — Mermaid `pie` charts per platform, one per axis (build / redesign / api).

## When to use

- User invokes `/screens render` (with or without `--scope`).
- User asks "draw the screen graph", "show me the status pie", "endpoint matrix".

## Inputs

- `--root <path>` (defaults to repo root).
- `--scope <name>` (`product` like `ppt` or `reality`, or `all`; default `all`).
- `--out <path>` (output dir; default `docs/screens/_diagrams`).

## Implementation

```bash
REPO_ROOT="$(git rev-parse --show-toplevel)"
pnpm -C "$REPO_ROOT/frontend" --filter @ppt/screen-map cli render \
  --root "$REPO_ROOT" \
  ${SCOPE:+--scope "$SCOPE"} \
  ${OUT:+--out "$OUT"}
```

## Output handling

- The CLI prints `wrote <path>` for each generated file.
- Reply with the three relative paths and offer to open them or render them inline (Mermaid blocks paste directly into GitHub markdown).
- If `docs/screens/<product>/` is empty, the diagrams will be near-empty too — note that `screen-map-init` should run first.
````

- [ ] **Step 2: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add -f .claude/skills/screen-render/SKILL.md
git commit -m "feat(skill): screen-render manifest (3 mermaid generators)" --no-verify
```

---

## Task 12: Skill manifest — `screen-query`

**Files:**
- Create: `.claude/skills/screen-query/SKILL.md`

- [ ] **Step 1: Write SKILL.md**

`.claude/skills/screen-query/SKILL.md`:

````markdown
---
name: screen-query
description: Query the screen-map tree by frontmatter filter (e.g. "product:ppt,implementations.ppt-web.redesignStatus:in-progress"). Output as table, JSON, or markdown. Use when user runs `/screens query` or asks "which screens are X", "list all Y screens", "find screens with Z".
---

# Screen-Query Skill

Read-only query against `docs/screens/<product>/*.md` frontmatter using the comma-AND key:value filter syntax (same as `screen-map-review --filter`).

## When to use

- User invokes `/screens query <expr>`.
- User asks "which screens are shipped but not redesigned", "list reality screens", "find screens using endpoint X".

## Inputs

- `<expr>` (positional; optional). Empty → returns all. Examples:
  - `product:ppt`
  - `implementations.ppt-web.redesignStatus:in-progress`
  - `product:ppt,implementations.ppt-web.buildStatus:shipped`
- `--root <path>` (defaults to repo root).
- `--format <fmt>` (`table` default, `json`, `md`).

## Implementation

```bash
REPO_ROOT="$(git rev-parse --show-toplevel)"
pnpm -C "$REPO_ROOT/frontend" --filter @ppt/screen-map cli query "$EXPR" \
  --root "$REPO_ROOT" \
  ${FORMAT:+--format "$FORMAT"}
```

## Output handling

- Echo CLI output verbatim. Final line shows count: `N screen-maps matched.`
- For `--format=md` the output is paste-friendly; for `--format=json` it's machine-readable.

## Common queries

| Goal | Expression |
|---|---|
| All PPT screens | `product:ppt` |
| Mobile screens still in progress | `implementations.mobile.buildStatus:in-progress` |
| Reality screens with redesign applied | `product:reality,implementations.reality-web.redesignStatus:applied` |
| Shipped but redesign not started | `implementations.ppt-web.buildStatus:shipped,implementations.ppt-web.redesignStatus:not-started` |

Filter syntax limit: comma-AND only (no `OR`). For more complex conditions, run multiple queries and intersect manually, or wait for Phase 3+ to add a richer query DSL.
````

- [ ] **Step 2: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add -f .claude/skills/screen-query/SKILL.md
git commit -m "feat(skill): screen-query manifest (frontmatter filter + 3 formats)" --no-verify
```

---

## Task 13: CLAUDE.md addenda — root + 4 subprojects

**Files:**
- Modify: `CLAUDE.md` (root)
- Create or Modify: `frontend/apps/ppt-web/CLAUDE.md`
- Create or Modify: `frontend/apps/reality-web/CLAUDE.md`
- Create or Modify: `frontend/apps/mobile/CLAUDE.md`
- Create or Modify: `mobile-native/CLAUDE.md`

- [ ] **Step 1: Inspect current state of each file**

Run:

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
for f in CLAUDE.md frontend/apps/ppt-web/CLAUDE.md frontend/apps/reality-web/CLAUDE.md frontend/apps/mobile/CLAUDE.md mobile-native/CLAUDE.md; do
  echo "=== $f ==="
  if [ -f "$f" ]; then
    wc -l "$f"
    head -3 "$f"
  else
    echo "(does not exist)"
  fi
done
```

This confirms whether you'll create or append for each one.

- [ ] **Step 2: Append to root `CLAUDE.md`**

Append to `/Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/CLAUDE.md`:

```markdown

## Screen-Map Self-Management Protocol

The repo includes a screen-map system at `docs/screens/<product>/<id>.md`. Agents working on UI/route code should integrate with it. See [the design spec](docs/superpowers/specs/2026-05-07-screen-map-system-design.md) Section 9.

### Rules for agents

**A. On screen-related code changes (before committing):**

1. `screen-edit <id>` to load context for the screen you're modifying.
2. Update frontmatter (`buildStatus`, `apiStatus`) if outcomes changed.
3. Add Agent Log entry: `<date> — agent: <terse summary>`.
4. Update `Notes > Specific (recent)` if the change is relevant for future agents.
5. Run `/screens validate`.

**B. On new route / mobile screen added:**

1. `/screens update` detects drift.
2. Create or attach the new route to a screen-map via `/screens init --add` or by editing an existing one.

**C. On redesign milestone (Figma frame ready):**

1. `/screens review --filter=redesignStatus:not-started` to walk the candidates.
2. After implementation: `screen-edit <id>` to flip `redesignStatus: in-progress → applied`.

**D. Periodically (manual cadence):**

- `/screens query "buildStatus:shipped,redesignStatus:not-started"` — find redesign roadmap candidates.
- `/screens render --scope=ppt` — refresh status dashboard.
```

- [ ] **Step 3: Append to `frontend/apps/ppt-web/CLAUDE.md`**

If the file exists, append; if not, create with this exact content:

```markdown
# ppt-web — Property Management Web App

## Screen-Map integration

When implementing or modifying a route in this app:

1. Identify the screen-map id (typically `ppt/<kebab-slug>`).
2. **Before coding**: run `/screens edit ppt/<id>` to load full context (related screens, endpoints, recent agent log).
3. **After coding**: update the screen-map's `implementations.ppt-web` block:
   - `buildStatus`: `planned` → `in-progress` → `shipped`.
   - `apiStatus`: `stub` / `partial` / `complete` based on backend reality.
   - `redesignStatus`: only flip to `applied` if a Figma frame was the source of truth.
4. Add an Agent Log entry: `<date> — agent: <what changed>`.
5. Run `/screens validate` to confirm cross-references are clean.
6. If the change adds a new route not yet in the screen-map: run `/screens update` to surface drift, then `/screens init --add "<Screen Name>"` to create the new entry.
```

- [ ] **Step 4: Append to `frontend/apps/reality-web/CLAUDE.md`**

Same pattern as Step 3, with content:

```markdown
# reality-web — Reality Portal Web App

## Screen-Map integration

When implementing or modifying a route in this app:

1. Identify the screen-map id (typically `reality/<kebab-slug>`).
2. **Before coding**: run `/screens edit reality/<id>` to load full context.
3. **After coding**: update `implementations.reality-web` block (`buildStatus` / `apiStatus` / `redesignStatus`).
4. Add an Agent Log entry: `<date> — agent: <what changed>`.
5. Run `/screens validate`.
6. New routes: `/screens update` then `/screens init --add "<Screen Name>"`.
```

- [ ] **Step 5: Append to `frontend/apps/mobile/CLAUDE.md`**

Same pattern, content:

```markdown
# mobile — React Native Mobile App

## Screen-Map integration

When implementing or modifying a screen in this app:

1. Identify the screen-map id under the `ppt/` product (mobile screens share screen-maps with ppt-web — they're platforms of the same logical concept).
2. **Before coding**: run `/screens edit ppt/<id>` to load full context.
3. **After coding**: update `implementations.mobile` block (`buildStatus` / `apiStatus` / `redesignStatus`).
4. Add an Agent Log entry: `<date> — agent: <what changed>`.
5. Run `/screens validate`.
6. New screens: `/screens update` then `/screens init --add "<Screen Name>"`.
```

- [ ] **Step 6: Append to `mobile-native/CLAUDE.md`**

Same pattern, content:

```markdown
# mobile-native — Reality Portal Mobile (KMP)

## Screen-Map integration

When implementing or modifying a screen in this KMP app:

1. Identify the screen-map id under the `reality/` product (mobile-native screens share screen-maps with reality-web).
2. **Before coding**: run `/screens edit reality/<id>` to load full context.
3. **After coding**: update `implementations.mobile-native` block (`buildStatus` / `apiStatus` / `redesignStatus`).
4. Add an Agent Log entry: `<date> — agent: <what changed>`.
5. Run `/screens validate`.
6. New screens: `/screens update` then `/screens init --add "<Screen Name>"`.
```

- [ ] **Step 7: Confirm all 5 files exist and have the new content**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
for f in CLAUDE.md frontend/apps/ppt-web/CLAUDE.md frontend/apps/reality-web/CLAUDE.md frontend/apps/mobile/CLAUDE.md mobile-native/CLAUDE.md; do
  echo "=== $f ==="
  grep -c "Screen-Map" "$f" || echo "MISSING"
done
```

Expected: each file has at least one match for "Screen-Map" (the section header or integration text).

- [ ] **Step 8: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add CLAUDE.md \
        frontend/apps/ppt-web/CLAUDE.md \
        frontend/apps/reality-web/CLAUDE.md \
        frontend/apps/mobile/CLAUDE.md \
        mobile-native/CLAUDE.md
git commit -m "docs(claude): screen-map self-management protocol (root + 4 subprojects)" --no-verify
```

---

## Task 14: `/screens` slash command — extend to 7 subcommands

**Files:**
- Modify: `.claude/commands/screens.md`

- [ ] **Step 1: Replace `screens.md` content**

`.claude/commands/screens.md`:

````markdown
# /screens — Screen-Map dispatcher

Dispatch into a screen-map subcommand. Phase 3a supports all 7 subcommands.

## Usage

```bash
/screens validate                                       # Phase 1
/screens validate --strict

/screens init --product=ppt                             # Phase 2
/screens init --product=reality --designs=designs/2026-q2.zip
/screens init --product=ppt --add="Custom screen 1"

/screens edit ppt/building-detail                       # Phase 2
/screens edit reality/property-detail --playwright

/screens review                                         # Phase 2
/screens review --product=ppt --preview=staging

/screens update                                         # Phase 3a NEW
/screens update --strict

/screens render                                         # Phase 3a NEW
/screens render --scope=ppt
/screens render --out=/tmp/diagrams

/screens query                                          # Phase 3a NEW
/screens query "product:ppt"
/screens query "implementations.ppt-web.redesignStatus:in-progress" --format=md
```

## Implementation

Parse `$ARGUMENTS` for the first token (subcommand) and the rest (forwarded flags).

- `validate` → invoke the `screen-map-validate` skill.
- `init` → invoke the `screen-map-init` skill (chat-driven grouping).
- `edit <id>` → invoke the `screen-edit` skill.
- `review` → invoke the `screen-map-review` skill.
- `update` → invoke the `screen-map-update` skill.
- `render` → invoke the `screen-render` skill.
- `query <expr>` → invoke the `screen-query` skill.
- Missing/unknown subcommand → print this usage block.
````

- [ ] **Step 2: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add -f .claude/commands/screens.md
git commit -m "feat(slash): /screens dispatches all 7 subcommands (phase-3a)" --no-verify
```

---

## Task 15: Phase 3a ship checkpoint

**Files:** none modified — verification only, plus an empty checkpoint commit.

- [ ] **Step 1: Run the full screen-map test suite**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test 2>&1 | tail -10
```

Expected: 72 tests pass across 17 test files.

- [ ] **Step 2: Typecheck and biome**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend
pnpm --filter @ppt/screen-map typecheck
pnpm biome check packages/screen-map
```

Both must pass cleanly.

- [ ] **Step 3: Smoke-test the three new subcommands**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
ABS_ROOT="$(pwd)"
# update — empty tree should report unmapped sitemap entries (or "No drift detected." if zero sitemap)
pnpm -C frontend --filter @ppt/screen-map cli update --root "$ABS_ROOT" 2>&1 | head -10

# render — should write 3 files
pnpm -C frontend --filter @ppt/screen-map cli render --root "$ABS_ROOT" --out /tmp/screen-map-ship-render 2>&1 | head -5
ls /tmp/screen-map-ship-render/

# query — empty match
pnpm -C frontend --filter @ppt/screen-map cli query "product:ppt" --root "$ABS_ROOT" 2>&1 | tail -3

# Cleanup
rm -rf /tmp/screen-map-ship-render
```

Expected: all three commands execute without errors. `update` may report unmapped-sitemap issues (normal, screen-maps don't exist yet); `render` produces 3 files; `query` reports `0 screen-maps matched.`.

- [ ] **Step 4: Confirm slash command + 3 new skills are discoverable**

```bash
ls /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/.claude/commands/screens.md \
   /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/.claude/skills/screen-map-update/SKILL.md \
   /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/.claude/skills/screen-render/SKILL.md \
   /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/.claude/skills/screen-query/SKILL.md
```

Expected: all 4 files listed.

- [ ] **Step 5: Confirm CLAUDE.md addenda are in place**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
for f in CLAUDE.md frontend/apps/ppt-web/CLAUDE.md frontend/apps/reality-web/CLAUDE.md frontend/apps/mobile/CLAUDE.md mobile-native/CLAUDE.md; do
  grep -q "Screen-Map" "$f" && echo "OK: $f" || echo "MISSING: $f"
done
```

Expected: all five lines say `OK: …`.

- [ ] **Step 6: Tag Phase-3a-complete checkpoint commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git commit --allow-empty -m "feat(screen-map): Phase 3a (update, render, query, protocol) complete

Phase 3a in place:
- Polish: parseFilter ':' split, appendAgentLog anchor, ESM SRI docs.
- scanDrift detects 5 categories (unmapped sitemap, unknown endpoint,
  unknown component, unknown use case/epic, orphan screen).
- render: site graph (Mermaid), endpoint matrix (markdown table),
  status dashboard (Mermaid pie per platform per axis).
- queryScreens + formatQueryResult with table/json/md formats.
- Skills: screen-map-update, screen-render, screen-query.
- /screens dispatcher extended to all 7 subcommands.
- CLAUDE.md addenda: root self-management protocol +
  per-subproject guidance (ppt-web, reality-web, mobile, mobile-native).

Phase 3b (bootstrap runs against PPT and Reality) is a separate plan
and PR." --no-verify
```

- [ ] **Step 7: Push and open PR**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git push -u origin feature/screen-map-phase-3
gh pr create --base feature/screen-map-phase-2 --title "Screen-Map system Phase 3a: update, render, query, protocol" --body "$(cat <<'EOF'
## Summary

Phase 3a of the [screen-map system](docs/superpowers/specs/2026-05-07-screen-map-system-design.md). **Stacked on PR #225 (Phase 2)**.

After Phase 2's "fully usable end-to-end" workflow, Phase 3a closes the self-management loop:

- `/screens update` detects drift between code and screen-maps (5 drift categories).
- `/screens render` generates Mermaid diagrams (site graph, endpoint matrix, status dashboard) to `docs/screens/_diagrams/`.
- `/screens query <expr>` queries screen-maps by frontmatter filter (3 output formats).
- CLAUDE.md addenda (root + 4 subprojects) make agents self-managing — when working on a route, they automatically know to run `/screens edit` first and `/screens validate` after.

### What landed

- **Polish (T1-3):** parseFilter `:` split, appendAgentLog anchor, ESM SRI documentation.
- **scanDrift (T4-5):** detects 5 categories of code↔screen-map drift.
- **render (T6-7):** site graph + endpoint matrix + status dashboard generators.
- **query (T8-9):** filter + 3 output formats; index re-exports Phase 3a public API.
- **Skills + slash (T10-12, 14):** 3 new skill manifests; `/screens` dispatcher covers all 7 subcommands.
- **CLAUDE.md addenda (T13):** root + per-subproject self-management protocol.

### Verification

- 72/72 tests pass (was 56 in Phase 2, +16 new).
- `pnpm biome check packages/screen-map` clean.
- `pnpm --filter @ppt/screen-map typecheck` clean.
- Smoke: `update`, `render`, `query` subcommands all execute end-to-end.

## Test plan

- [x] `pnpm --filter @ppt/screen-map test` — 72/72 pass
- [x] `pnpm biome check packages/screen-map` — clean
- [x] `pnpm --filter @ppt/screen-map typecheck` — clean
- [x] CLI smoke: `update`, `render --out=/tmp/...`, `query "product:ppt"`
- [x] CLAUDE.md addenda in 5 files
- [x] All 3 new skills + slash command extension discoverable
- [ ] CI workflow runs on this PR (verify after push)
- [ ] Manual smoke after Phase 2 + Phase 3a merge: actually run `/screens render` against a populated tree

## Phase 3b preview

Bootstrap runs: actually invoke `/screens init --product=ppt` and `/screens init --product=reality` against the live repo to populate `docs/screens/<product>/` with real screen-map markdown files. Separate plan + PR after this lands.

## Known limitations carried forward

- SPA `app.tsx` still has no automated tests (browser-only, `// @ts-nocheck`).
- `--preview=design` SPA rendering still half-implemented (CLI accepts the flag, SPA only renders local/staging).
- `loadScreenContext`'s `--playwright` flag still a no-op.
- ESM SRI is documentation-only — browser ES module spec doesn't allow integrity on `import` statements.
- `parseFilter` doesn't support boolean OR; users intersect manually for now.
EOF
)" 2>&1 | tail -3
```

Expected: PR opens against `feature/screen-map-phase-2` (which itself is stacked on PR #220's Phase 1 branch).

---

## Self-Review

1. **Spec coverage:**
   - Section 5.2 (`screen-map-update`) → Tasks 4 (logic), 5 (CLI), 10 (skill).
   - Section 5.6 (`screen-render`) → Tasks 6 (logic), 7 (CLI), 11 (skill).
   - Section 5.7 (`screen-query`) → Tasks 8 (logic), 9 (CLI + index), 12 (skill).
   - Section 9 (CLAUDE.md addenda) → Task 13.
   - `/screens` dispatcher → Task 14.
   - Phase 2 deferred polish → Tasks 1-3.
   - Ship checkpoint → Task 15.
   - Bootstrap runs (Phase 3b) — explicitly deferred.

2. **Placeholder scan:** None of "TBD", "TODO", "implement later", "appropriate error handling", or "similar to task N". The Step 1 ESM-SRI hash placeholder (`<run script to compute>`) is the only literal placeholder, and it's intentional — the engineer literally runs the script and substitutes. The plan inlines the script.

3. **Type consistency:**
   - `DriftIssue` (discriminated union with `kind`) — defined in Task 4, consumed in Tasks 5, 10.
   - `ScreenMap`, `ValidationContext` — re-used from Phase 1 + 2.
   - `QueryFormat` ('table' | 'json' | 'md') — defined in Task 8, consumed in Task 9.
   - `parseFilter` — Phase 2's existing export, modified in Task 1, kept in same shape.
   - CLI flag names: `--root`, `--strict`, `--scope`, `--out`, `--format`, `--filter`, `--add`, `--preview`. All used identically across CLI subcommands and skill manifests.

If during execution any of the upstream `@ppt/sitemap` exports have changed since Phase 2 (`apiServerEndpoints`, `realityServerEndpoints`, `pptWebRoutes`, etc.), the validate context construction in Phase 1 still applies and `scan-drift.ts` consumes the same `ValidationContext` — no separate adjustment needed.
