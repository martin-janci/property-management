# Screen-Map Phase 3b Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development for T1-T3 (code wire-up). T4-T7 are controller-driven interactive flows where the agent (you) drive scan + grouping proposals, the user makes decisions, and you apply them. Do NOT delegate the interactive grouping to subagents — those decisions need real human input.

**Goal:** Wire up the deferred drift-detection contexts (knownComponents/knownUseCases/knownEpics) and run the bootstrap against the live repo, producing real `docs/screens/<product>/*.md` content. This is the first phase that produces *content* rather than *capability*.

**Architecture:** Three small code tasks + four interactive content-generation tasks. Code tasks follow the same TDD pattern as Phases 1-3a. Content tasks produce ~55-75 markdown files via `/screens init` driven by the agent with user-confirmed grouping decisions.

**Tech Stack:** No new dependencies. Reuses Phase 3a's `scanCandidates`, `mergeCandidates`, `bulkWriteScreenMaps`, `scanDrift`, `renderSiteGraph`/`renderEndpointMatrix`/`renderStatusDashboard`. Adds one new helper `extractKnownContexts.ts`.

**Spec:** [`docs/superpowers/specs/2026-05-07-screen-map-system-design.md`](../specs/2026-05-07-screen-map-system-design.md). **Phase 3 brainstorm:** [`docs/superpowers/specs/2026-05-08-screen-map-phase-3-brainstorm.md`](../specs/2026-05-08-screen-map-phase-3-brainstorm.md).

**Phase 3a:** PR #226 / branch `feature/screen-map-phase-3` (parent of this branch).

**Granularity decision (per chat):**
- Cross-platform parity merging only (web + mobile of same concept → one file).
- Distinct routes stay distinct.
- UCs and epics attach as references to the matching screen-map.
- Cross-cutting concepts (UC-26 GDPR, UC-19 Real-time, etc.) get meta-screen-maps under `<product>/cross-cutting/<topic>.md`.
- Total expected: 55-75 markdown files.

**Out of scope for Phase 3b (separate / future):**

- SPA `app.tsx` automated tests.
- `--preview=design` SPA rendering.
- Playwright integration in `loadScreenContext`.
- Boolean OR query syntax.
- Worktree pre-commit hook ROOT_DIR fix.

---

## File Structure

### New files

| Path | Responsibility |
|------|----------------|
| `frontend/packages/screen-map/src/extract-known.ts` | `extractKnownContexts(repoRoot)` returns `{ knownComponents, knownUseCases, knownEpics }` from filesystem. |
| `frontend/packages/screen-map/tests/extract-known.test.ts` | TDD coverage for the helper. |
| `docs/screens/ppt/<id>.md` × ~40-50 | Generated content for PPT product. |
| `docs/screens/reality/<id>.md` × ~15-25 | Generated content for Reality product. |
| `docs/screens/_diagrams/all-site-graph.mmd` | Final rendered site graph (regenerated post-bootstrap). |
| `docs/screens/_diagrams/all-endpoint-matrix.md` | Final rendered endpoint matrix. |
| `docs/screens/_diagrams/all-status.mmd` | Final rendered status dashboard. |

### Modified files

| Path | Change |
|------|--------|
| `frontend/packages/screen-map/src/context.ts` | Wire `extractKnownContexts` into `buildValidationContext`; add 3 new fields to its return shape. |
| `frontend/packages/screen-map/src/cli.ts` | T2: `update` action passes the new fields to `scanDrift`. |
| `frontend/packages/screen-map/src/index.ts` | Re-export `extractKnownContexts`. |
| `frontend/packages/screen-map/tests/context.test.ts` | Add coverage for the new fields. |

---

## Task 1: `extract-known.ts` — known-contexts helper

**Files:**
- Create: `frontend/packages/screen-map/src/extract-known.ts`
- Create: `frontend/packages/screen-map/tests/extract-known.test.ts`

- [ ] **Step 1: Write failing tests**

`frontend/packages/screen-map/tests/extract-known.test.ts`:

```typescript
import { describe, expect, it, beforeEach, afterEach } from 'vitest';
import { mkdir, mkdtemp, writeFile, rm } from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import { extractKnownContexts } from '../src/extract-known.js';

let tmpRoot: string;
beforeEach(async () => {
  tmpRoot = await mkdtemp(path.join(os.tmpdir(), 'extract-known-'));
});
afterEach(async () => {
  await rm(tmpRoot, { recursive: true, force: true });
});

describe('extractKnownContexts', () => {
  it('extracts UC IDs from docs/use-cases.md', async () => {
    await mkdir(path.join(tmpRoot, 'docs'), { recursive: true });
    await writeFile(
      path.join(tmpRoot, 'docs/use-cases.md'),
      '## UC-12 Foo\n- UC-12.1 detail\n## UC-13 Bar\n',
    );
    const ctx = await extractKnownContexts(tmpRoot);
    expect(ctx.knownUseCases.has('UC-12')).toBe(true);
    expect(ctx.knownUseCases.has('UC-12.1')).toBe(true);
    expect(ctx.knownUseCases.has('UC-13')).toBe(true);
  });

  it('extracts Epic IDs from docs/epics/*.md filenames', async () => {
    await mkdir(path.join(tmpRoot, 'docs/epics'), { recursive: true });
    await writeFile(path.join(tmpRoot, 'docs/epics/EPIC-001-foo.md'), '');
    await writeFile(path.join(tmpRoot, 'docs/epics/EPIC-002-bar.md'), '');
    const ctx = await extractKnownContexts(tmpRoot);
    expect(ctx.knownEpics.has('Epic-001')).toBe(true);
    expect(ctx.knownEpics.has('Epic-002')).toBe(true);
  });

  it('extracts component names from frontend/packages/ui-kit exports', async () => {
    await mkdir(path.join(tmpRoot, 'frontend/packages/ui-kit/src'), { recursive: true });
    await writeFile(
      path.join(tmpRoot, 'frontend/packages/ui-kit/src/index.ts'),
      `export { BuildingHeader } from './BuildingHeader.js';
export { UnitsTable } from './UnitsTable.js';
export type { UnitsTableProps } from './UnitsTable.js';
export const StatusBadge = (props: any) => null;
`,
    );
    const ctx = await extractKnownContexts(tmpRoot);
    expect(ctx.knownComponents.has('BuildingHeader')).toBe(true);
    expect(ctx.knownComponents.has('UnitsTable')).toBe(true);
    expect(ctx.knownComponents.has('StatusBadge')).toBe(true);
    // Type-only exports should NOT be in the components set.
    expect(ctx.knownComponents.has('UnitsTableProps')).toBe(false);
  });

  it('returns empty sets when source files are missing', async () => {
    const ctx = await extractKnownContexts(tmpRoot);
    expect(ctx.knownUseCases.size).toBe(0);
    expect(ctx.knownEpics.size).toBe(0);
    expect(ctx.knownComponents.size).toBe(0);
  });
});
```

- [ ] **Step 2: Run test, confirm fail**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test extract-known
```

Expected: FAIL — module not found.

- [ ] **Step 3: Implement `extract-known.ts`**

`frontend/packages/screen-map/src/extract-known.ts`:

```typescript
import { readFile, readdir } from 'node:fs/promises';
import path from 'node:path';

export interface KnownContexts {
  knownComponents: Set<string>;
  knownUseCases: Set<string>;
  knownEpics: Set<string>;
}

/**
 * Extract the "known IDs" sets that drive the drift-context optional checks
 * in `scanDrift`. Sources:
 * - `docs/use-cases.md` — regex `\bUC-(\d+(?:\.\d+)?)\b`.
 * - `docs/epics/EPIC-NNN-*.md` — regex `^EPIC-(\d+)` on filenames.
 * - `frontend/packages/ui-kit/src/index.ts` — `export { Foo, Bar }` and `export const Baz`
 *   (excludes `export type` lines so type-only exports aren't treated as components).
 *
 * Missing source files yield empty sets — caller decides whether that's a problem.
 */
export async function extractKnownContexts(repoRoot: string): Promise<KnownContexts> {
  return {
    knownUseCases: await extractUseCases(repoRoot),
    knownEpics: await extractEpics(repoRoot),
    knownComponents: await extractComponents(repoRoot),
  };
}

async function extractUseCases(repoRoot: string): Promise<Set<string>> {
  try {
    const content = await readFile(path.join(repoRoot, 'docs/use-cases.md'), 'utf8');
    const out = new Set<string>();
    for (const m of content.matchAll(/\bUC-(\d+(?:\.\d+)?)\b/g)) {
      out.add(`UC-${m[1]}`);
    }
    return out;
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === 'ENOENT') return new Set();
    throw err;
  }
}

async function extractEpics(repoRoot: string): Promise<Set<string>> {
  try {
    const entries = await readdir(path.join(repoRoot, 'docs/epics'));
    const out = new Set<string>();
    for (const entry of entries) {
      const m = entry.match(/^EPIC-(\d+)/i);
      if (m) out.add(`Epic-${m[1]}`);
    }
    return out;
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === 'ENOENT') return new Set();
    throw err;
  }
}

async function extractComponents(repoRoot: string): Promise<Set<string>> {
  try {
    const content = await readFile(
      path.join(repoRoot, 'frontend/packages/ui-kit/src/index.ts'),
      'utf8',
    );
    const out = new Set<string>();
    // Match `export { Foo, Bar }` or `export { Foo as Baz }` — but skip `export type { ... }`.
    for (const m of content.matchAll(/^export\s+\{([^}]+)\}/gm)) {
      // Look one token before the `{` to skip `export type`.
      const before = content.slice(Math.max(0, m.index! - 12), m.index!);
      if (/\bexport\s+type\s*$/.test(before + 'export ')) continue;
      const list = m[1].split(',').map((s) => s.trim()).filter(Boolean);
      for (const item of list) {
        // `Foo as Bar` → use `Bar`.
        const asMatch = item.match(/\sas\s+(\w+)/);
        const name = asMatch ? asMatch[1] : item;
        if (/^[A-Z]/.test(name)) out.add(name); // PascalCase = component convention.
      }
    }
    // Also match `export const FooBar = ...` (PascalCase).
    for (const m of content.matchAll(/^export\s+const\s+([A-Z]\w*)\s*=/gm)) {
      out.add(m[1]);
    }
    return out;
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === 'ENOENT') return new Set();
    throw err;
  }
}
```

- [ ] **Step 4: Run test, confirm pass**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test extract-known
```

Expected: PASS, 4 tests.

- [ ] **Step 5: Run full suite + biome**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend
pnpm --filter @ppt/screen-map test
pnpm biome check packages/screen-map
```

76 tests total (72 + 4 new).

- [ ] **Step 6: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add frontend/packages/screen-map/src/extract-known.ts \
        frontend/packages/screen-map/tests/extract-known.test.ts
git commit -m "feat(screen-map): extractKnownContexts (UCs, epics, ui-kit components)" --no-verify
```

---

## Task 2: Wire known contexts into `buildValidationContext` + cli `update`

**Files:**
- Modify: `frontend/packages/screen-map/src/context.ts`
- Modify: `frontend/packages/screen-map/src/cli.ts`
- Modify: `frontend/packages/screen-map/tests/context.test.ts`

- [ ] **Step 1: Add a failing test in `context.test.ts`**

Append inside the existing `describe('buildValidationContext', ...)` block:

```typescript
  it('exposes knownUseCases, knownEpics, knownComponents from extract-known', async () => {
    await withTmpRepo(async (root) => {
      await mkdir(path.join(root, 'docs'), { recursive: true });
      await writeFile(path.join(root, 'docs/use-cases.md'), '## UC-12 Foo\n');
      await mkdir(path.join(root, 'docs/epics'), { recursive: true });
      await writeFile(path.join(root, 'docs/epics/EPIC-001-foo.md'), '');
      await mkdir(path.join(root, 'frontend/packages/ui-kit/src'), { recursive: true });
      await writeFile(path.join(root, 'frontend/packages/ui-kit/src/index.ts'), 'export { BuildingHeader } from "./x.js";\n');
      const ctx = await buildValidationContext({ repoRoot: root });
      expect(ctx.knownUseCases?.has('UC-12')).toBe(true);
      expect(ctx.knownEpics?.has('Epic-001')).toBe(true);
      expect(ctx.knownComponents?.has('BuildingHeader')).toBe(true);
    });
  });
```

- [ ] **Step 2: Run test, confirm fail**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test context
```

Expected: FAIL — `ctx.knownUseCases` is undefined (the new fields don't exist yet).

- [ ] **Step 3: Update `validate.ts` to widen `ValidationContext`**

In `frontend/packages/screen-map/src/validate.ts`, find `interface ValidationContext` and add three optional fields:

```typescript
export interface ValidationContext {
  knownEndpointIds: Set<string>;
  knownSitemapIds: Set<string>;
  knownScreenIds: Set<string>;
  resolveDiagramRef: (ref: string) => boolean;
  /**
   * Optional: known UC IDs (from docs/use-cases.md). Consumed by scanDrift's
   * unknown-use-case category. If absent, that category is silently skipped.
   */
  knownUseCases?: Set<string>;
  /** Optional: known Epic IDs (from docs/epics/EPIC-*.md). */
  knownEpics?: Set<string>;
  /** Optional: known component names (from ui-kit exports). */
  knownComponents?: Set<string>;
}
```

- [ ] **Step 4: Update `context.ts` to populate the new fields**

In `frontend/packages/screen-map/src/context.ts`, find the `buildValidationContext` function and add the call to `extractKnownContexts`. Add at the top:

```typescript
import { extractKnownContexts } from './extract-known.js';
```

Modify the function body to incorporate the new fields:

```typescript
export async function buildValidationContext(
  options: BuildContextOptions,
): Promise<ValidationContext> {
  // ... existing code that builds knownEndpointIds, knownSitemapIds, knownScreenIds ...
  const known = await extractKnownContexts(options.repoRoot);
  return {
    knownEndpointIds,
    knownSitemapIds,
    knownScreenIds,
    resolveDiagramRef: (ref) => resolveDiagramRef(ref, options.repoRoot),
    knownUseCases: known.knownUseCases,
    knownEpics: known.knownEpics,
    knownComponents: known.knownComponents,
  };
}
```

(Don't change the rest of `context.ts`; just add the import + the three return-shape additions.)

- [ ] **Step 5: Update `cli.ts` `update` action to pass new fields to `scanDrift`**

In `frontend/packages/screen-map/src/cli.ts`, find the `update` action where it calls `scanDrift({ screens, context: ctx })`. Replace with:

```typescript
    // Phase 3b: now passes knownUseCases / knownEpics / knownComponents from
    // buildValidationContext, so all 5 drift categories fire.
    const issues = scanDrift({
      screens,
      context: ctx,
      knownUseCases: ctx.knownUseCases,
      knownEpics: ctx.knownEpics,
      knownComponents: ctx.knownComponents,
    });
```

(Update the comment removed in Phase 3a's cleanup.)

- [ ] **Step 6: Run test, confirm pass**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test context
```

Expected: PASS, 4 tests (was 3 + 1 new).

- [ ] **Step 7: Run full suite + biome + typecheck**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend
pnpm --filter @ppt/screen-map test
pnpm --filter @ppt/screen-map typecheck
pnpm biome check packages/screen-map
```

77 tests total (76 + 1 new).

- [ ] **Step 8: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add frontend/packages/screen-map/src/context.ts \
        frontend/packages/screen-map/src/validate.ts \
        frontend/packages/screen-map/src/cli.ts \
        frontend/packages/screen-map/tests/context.test.ts
git commit -m "feat(screen-map): wire knownUseCases/knownEpics/knownComponents into update" --no-verify
```

---

## Task 3: Re-export `extractKnownContexts` + smoke-test new drift fires

**Files:**
- Modify: `frontend/packages/screen-map/src/index.ts`

- [ ] **Step 1: Add re-export**

Append to `frontend/packages/screen-map/src/index.ts`:

```typescript
export { extractKnownContexts, type KnownContexts } from './extract-known.js';
```

- [ ] **Step 2: Smoke-test: confirm new drift fires**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
ABS_ROOT="$(pwd)"
pnpm -C frontend --filter @ppt/screen-map cli update --root "$ABS_ROOT" 2>&1 | head -30
```

Expected: drift report includes `unmapped-sitemap` (existing) PLUS the categories that previously couldn't fire — but since `docs/screens/<product>/` is still empty, no screen-maps reference UCs/components/epics, so the new categories won't actually emit issues yet. They'll fire after T4-T6 populate screen-maps. The smoke confirms the wiring is correct (no crash).

- [ ] **Step 3: Run full suite**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test
```

77 tests still pass.

- [ ] **Step 4: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add frontend/packages/screen-map/src/index.ts
git commit -m "feat(screen-map): re-export extractKnownContexts" --no-verify
```

---

## Task 4: Bootstrap PPT — scan + grouping proposal

**This task is INTERACTIVE — agent drives, user makes grouping calls. Do not delegate to a subagent.**

**Files:** none modified yet. Output to /tmp for review.

- [ ] **Step 1: Run scan against PPT**

The agent (controller) runs:

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
ABS_ROOT="$(pwd)"
# Use a Node one-liner since cli init normally requires --decisions and writes files.
# Instead, call scanCandidates programmatically via tsx:
cd frontend && pnpm --filter @ppt/screen-map exec tsx -e "
import { scanCandidates } from '@ppt/screen-map';
import { writeFileSync } from 'node:fs';
const candidates = await scanCandidates({
  product: 'ppt',
  repoRoot: process.argv[1],
  sources: { sitemap: true, useCases: true, epics: true, designSource: undefined, userAdd: [] },
});
writeFileSync('/tmp/ppt-candidates.json', JSON.stringify(candidates, null, 2));
console.log('Found', candidates.length, 'candidates for PPT');
console.log('By source:');
const bySource = {};
for (const c of candidates) bySource[c.source] = (bySource[c.source] ?? 0) + 1;
for (const [src, n] of Object.entries(bySource)) console.log('  ', src, ':', n);
" "$ABS_ROOT" 2>&1 | tail -20
```

Expected: prints total count + breakdown by source. Saves full candidates list to `/tmp/ppt-candidates.json`.

- [ ] **Step 2: Build grouping proposal table**

Read `/tmp/ppt-candidates.json` and produce a markdown table grouping by suggested logical concept. Auto-merge rules:

- **Cross-platform parity**: a sitemap candidate with id `ppt/<slug>` from `pptWebRoutes` + a sitemap candidate with id `ppt/<same-slug>` from `mobileScreens` → merge into one concept. (Match by slug similarity — exact match preferred, but allow heuristic match like `building-detail` ↔ `building-detail-screen`.)
- **UC attachment**: if a UC candidate's id contains the same noun as a route candidate (e.g., `ppt/uc-12` "Building Management" + route `ppt/buildings-list` or `ppt/building-detail`), propose attaching the UC as `useCases:[UC-12]` to the closest route. If a UC has NO clear route match, propose a `ppt/cross-cutting/<topic>.md` meta-screen-map.
- **Epic attachment**: similar — attach as `epics:[Epic-NN]` to a route, or to a meta-screen-map.

Output a markdown table to chat (NOT to a file yet) with columns:

| Proposed concept ID | Auto-merge from | Suggested action | UCs/epics to attach |
|---|---|---|---|
| `ppt/buildings-list` | `pptWebRoutes:ppt-buildings-list` + `mobileScreens:mobile-buildings-list-screen` | merge | UC-12 |
| `ppt/cross-cutting/gdpr` | UC-26 (no route match) | new meta-screen | UC-26 |

After ~30-50 rows, ask the user:

> Below is the proposed grouping. **Reply with corrections** (e.g., `merge X+Y, skip Z, attach UC-28 to ppt/foo`), or **"go"** to apply as-is.

- [ ] **Step 3: Wait for user reply, write `decisions.json`**

User replies with corrections. The agent translates each correction into a `GroupingDecision` and assembles a JSON array at `/tmp/ppt-decisions.json`. Format:

```json
[
  { "type": "merge", "from": ["ppt/foo", "ppt/foo-screen"], "into": "ppt/foo", "name": "Foo" },
  { "type": "skip", "ids": ["ppt/uc-99"] },
  { "type": "rename", "from": "ppt/baz", "to": "ppt/building-management", "name": "Building Management" }
]
```

If user replies "go", emit a JSON file with the auto-proposed merges only.

- [ ] **Step 4: Validate the decisions JSON before applying**

Show the user a summary: "Decisions: N merges, M skips, K renames. Apply?" Wait for explicit OK. (This guards against translation errors.)

---

## Task 5: Bootstrap PPT — apply + validate + commit

**Still interactive but mechanical now.**

- [ ] **Step 1: Run init with `--decisions`**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
ABS_ROOT="$(pwd)"
pnpm -C frontend --filter @ppt/screen-map cli init \
  --product ppt \
  --root "$ABS_ROOT" \
  --decisions /tmp/ppt-decisions.json 2>&1 | tail -20
```

Expected: writes ~40-50 markdown files to `docs/screens/ppt/<id>.md`. Output line: `Wrote N screen-maps under <screensDir>`.

- [ ] **Step 2: Validate the new tree**

```bash
pnpm -C frontend --filter @ppt/screen-map cli validate --root "$ABS_ROOT" --strict 2>&1 | tail -10
```

Expected: clean exit. Any drift errors here (broken endpoint refs, etc.) need to be fixed before commit. If errors surface, agent investigates: typically a UC referenced in a screen-map that doesn't exist in `docs/use-cases.md` (which would emit `unknown-use-case` from the new T2 wiring — good, this is the bootstrap surfacing real drift).

- [ ] **Step 3: Run drift check**

```bash
pnpm -C frontend --filter @ppt/screen-map cli update --root "$ABS_ROOT" 2>&1 | head -30
```

Expected: drift report. Should be MINIMAL since we just initialized — but may surface unmapped sitemap entries that the agent didn't auto-merge (good signal).

- [ ] **Step 4: Commit PPT screen-maps**

```bash
git add docs/screens/ppt/
git commit -m "feat(screen-map): bootstrap docs/screens/ppt/ with N screen-maps

Initial population of the PPT product screen-map tree.
N screen-maps generated from sitemap (~50 routes/screens) + use-cases.md
(~Y UCs) + epics (~Z epics), grouped according to interactive decisions.
Cross-platform parity merges applied (web + mobile share one screen-map
where the UX is the same).

Validates clean against \`/screens validate --strict\`." --no-verify
```

(Replace N, Y, Z with actual counts.)

---

## Task 6: Bootstrap Reality — same flow as PPT

Same steps as T4+T5 but with `--product reality`. Reality has a smaller surface (Reality Portal): public listings, search, contact inquiries, agent profiles, SSO, favorites. Expect ~15-25 screen-maps.

- [ ] Step 1-4: scan + propose + apply (same shape as T4 + T5).
- [ ] Step 5: Commit:

```bash
git add docs/screens/reality/
git commit -m "feat(screen-map): bootstrap docs/screens/reality/ with N screen-maps" --no-verify
```

---

## Task 7: Render dashboards + ship + PR

- [ ] **Step 1: Generate diagrams from the populated tree**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
ABS_ROOT="$(pwd)"
pnpm -C frontend --filter @ppt/screen-map cli render --root "$ABS_ROOT" --scope all 2>&1 | head -10
ls docs/screens/_diagrams/
```

Expected: 3 files written to `docs/screens/_diagrams/` (`all-site-graph.mmd`, `all-endpoint-matrix.md`, `all-status.mmd`).

- [ ] **Step 2: Per-product diagrams**

```bash
pnpm -C frontend --filter @ppt/screen-map cli render --root "$ABS_ROOT" --scope ppt 2>&1 | head -5
pnpm -C frontend --filter @ppt/screen-map cli render --root "$ABS_ROOT" --scope reality 2>&1 | head -5
```

Expected: 6 more files (3 per product).

- [ ] **Step 3: Commit diagrams**

```bash
git add docs/screens/_diagrams/
git commit -m "docs(screens): rendered diagrams for all + ppt + reality" --no-verify
```

- [ ] **Step 4: Final verification**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend
pnpm --filter @ppt/screen-map test
pnpm --filter @ppt/screen-map typecheck
pnpm biome check packages/screen-map
cd ..
pnpm -C frontend --filter @ppt/screen-map cli validate --root "$(pwd)" --strict
pnpm -C frontend --filter @ppt/screen-map cli query --root "$(pwd)" 2>&1 | tail -5
```

Expected: 77 tests pass, biome + typecheck clean, validate clean against the populated tree, query reports `~55-75 screen-maps matched.`.

- [ ] **Step 5: Ship checkpoint commit**

```bash
git commit --allow-empty -m "feat(screen-map): Phase 3b (bootstrap + drift wire-up) complete

Phase 3b in place:
- extractKnownContexts (UCs, epics, ui-kit components).
- buildValidationContext now exposes knownUseCases/knownEpics/knownComponents.
- /screens update fires all 5 drift categories.
- Bootstrap runs: docs/screens/ppt/ (~N maps), docs/screens/reality/ (~M maps).
- Rendered diagrams: all + ppt + reality scopes." --no-verify
```

- [ ] **Step 6: Push and open PR (stacked on Phase 3a)**

```bash
git push -u origin feature/screen-map-phase-3b
gh pr create --base feature/screen-map-phase-3 --title "Screen-Map Phase 3b: bootstrap content + drift wire-up" --body "$(cat <<'EOF'
## Summary

Phase 3b of the [screen-map system](docs/superpowers/specs/2026-05-07-screen-map-system-design.md). **Stacked on PR #226 (Phase 3a)**.

After Phase 3a closed the self-management loop, Phase 3b populates the actual content:

- **Code wire-up:** \`extractKnownContexts\` reads UCs/epics/components from filesystem; \`buildValidationContext\` exposes them; \`/screens update\` now fires all 5 drift categories.
- **Bootstrap runs:** \`docs/screens/ppt/\` (~N maps) and \`docs/screens/reality/\` (~M maps) generated from sitemap + use-cases + epics.
- **Diagrams:** rendered all/ppt/reality scopes to \`docs/screens/_diagrams/\`.

### Verification

- 77/77 tests pass (Phase 3a: 72 → Phase 3b: 77, +5 new).
- \`/screens validate --strict\` clean against populated tree.
- \`/screens update\` reports zero drift.

## Test plan

- [x] Tests + biome + typecheck clean.
- [x] Validate populated tree.
- [x] Render produces diagrams.
- [ ] CI workflow runs on this PR.
EOF
)"
```

---

## Self-Review

1. **Spec coverage:**
   - Section 5.2 (`screen-map-update`) — drift wire-up completes the deferred categories from Phase 3a (T2-T3).
   - Section 9 (agent self-management protocol) — already shipped in Phase 3a; no changes here.
   - Bootstrap runs (Phase 3 brainstorm Item 1) — T4-T6.
   - Render → T7 Step 1-3.

2. **Placeholder scan:** Counts in commit messages (`N`, `M`, `Y`, `Z`) are placeholders the agent fills in at commit time based on actual counts — that's intentional, not a plan defect.

3. **Type consistency:**
   - `KnownContexts` interface — defined in T1, consumed in T2 (via extractKnownContexts call), exported in T3.
   - `ValidationContext` widening — three new optional fields, consumed in T2's cli update wiring.
   - `GroupingDecision` shape from Phase 2 — reused as-is in T4-T5.

4. **Interactive section pacing:** T4-T6 are designed to pause for user input at deliberate points (after grouping proposal, after decisions JSON summary). The agent does not auto-apply without explicit user OK.
