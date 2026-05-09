# Screen-Map Phase 2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring the screen-map system from "validate only" (Phase 1) to "fully usable end-to-end". After Phase 2 a user can: bootstrap a fresh `docs/screens/<product>/` tree from sitemap + designs + user input (`/screens init`), inspect a single screen with full agent-friendly context (`/screens edit <id>`), and walk every screen interactively in a browser-based review UI capturing OK/Note feedback into the markdown (`/screens review`).

**Architecture:** Three new feature surfaces sitting on Phase 1's foundation. (1) `DesignSource` is a small adapter layer (interface + ZipAdapter + Claude Design stub) for design-image consumption. (2) The Visual Review server is a `hono`-based local HTTP service on `127.0.0.1` with a Preact-via-`esm.sh` SPA — no build step. (3) Three skills (`screen-map-init`, `screen-edit`, `screen-map-review`) glue everything together via the `/screens` slash dispatcher. First three tasks fold in Phase-1-review-flagged polish before adding new surface.

**Tech Stack:** TypeScript 5 (strict, ESM, ES2022 target), Zod 3 (frontmatter schema), gray-matter (markdown), `commander` (CLI), `hono` 4 (review server), Preact via ESM `esm.sh` (no-build SPA), `yauzl` 3 or `unzipper` 0.12 (zip extraction in ZipAdapter), vitest 2 (tests), pnpm 8 workspace, Biome (lint/format).

**Spec:** [`docs/superpowers/specs/2026-05-07-screen-map-system-design.md`](../specs/2026-05-07-screen-map-system-design.md) — sections 5.1, 5.3, 5.5, 6, 7. **Phase 2 brainstorm decisions:** [`docs/superpowers/specs/2026-05-07-screen-map-phase-2-brainstorm.md`](../specs/2026-05-07-screen-map-phase-2-brainstorm.md).

**Phase 1 plan & PR:** Plan at [`docs/superpowers/plans/2026-05-07-screen-map-phase-1-foundation.md`](./2026-05-07-screen-map-phase-1-foundation.md), shipped as PR #220 on branch `feature/vigilant-mirzakhani-4baf0e`. **This Phase 2 work continues on a separate branch `feature/screen-map-phase-2` branched off the Phase 1 head**, so PR #220 stays focused on Phase 1 review.

**Out of scope for Phase 2 (Phase 3 plan):**

- `screen-map-update` (drift detection between code and screen-maps).
- `screen-render` (mermaid generators).
- `screen-query` (read-only frontmatter queries).
- Root + per-subproject `CLAUDE.md` addenda for the agent self-management protocol (Section 9 of the spec).
- First bootstrap runs of `/screens init` against PPT and Reality.
- Periodic-loop / cron automation around `screen-query`.
- Worktree-vs-shared-hooks `ROOT_DIR` quirk fix (pre-existing repo issue).

---

## File Structure

### New files

| Path | Responsibility |
|------|----------------|
| `frontend/packages/screen-map/src/design-source/index.ts` | `DesignFrame`, `DesignSource` interfaces + helper to instantiate adapter from config. |
| `frontend/packages/screen-map/src/design-source/zip-adapter.ts` | `ZipAdapter` reads `manifest.json` from a ZIP file and lazy-extracts frame images on `get(id)`. |
| `frontend/packages/screen-map/src/design-source/claude-design.ts` | `ClaudeDesignAdapter` stub — `throw NotImplementedError`. |
| `frontend/packages/screen-map/src/scan.ts` | `scanCandidates(opts)` enumerates candidate screens from `@ppt/sitemap` + `docs/use-cases.md` + epic IDs + DesignSource frames + user-provided list. |
| `frontend/packages/screen-map/src/grouping.ts` | `mergeCandidates(candidates, decisions)` applies user grouping decisions (merge/split/skip) to produce final concept set. |
| `frontend/packages/screen-map/src/init-write.ts` | `bulkWriteScreenMaps(concepts, screensDir)` writes `<product>/<id>.md` files for each finalized concept. |
| `frontend/packages/screen-map/src/edit-context.ts` | `loadScreenContext(id, opts)` produces a markdown summary of a single screen + parents + children + related + sitemap entries + Playwright screenshot path. |
| `frontend/packages/screen-map/src/review-server/server.ts` | `hono` app: 127.0.0.1 bind, session token middleware, route mounting. |
| `frontend/packages/screen-map/src/review-server/api.ts` | Endpoint handlers: `/api/session`, `/api/screens/:id`, `/api/screens/:id/review`, `/api/session/finish`, `/api/designs/:adapter/:frame-id`. |
| `frontend/packages/screen-map/src/review-server/session.ts` | Per-process session state: token, screen list, current index, design caches. |
| `frontend/packages/screen-map/src/review-server/start.ts` | Server startup: pick free port, spawn server, open browser, register SIGINT handler. |
| `frontend/packages/screen-map/src/review-server/client/index.html` | SPA shell, imports `app.tsx` via ESM. |
| `frontend/packages/screen-map/src/review-server/client/app.tsx` | Preact entry; routes to `ScreenView` per screen id. |
| `frontend/packages/screen-map/src/review-server/client/styles.css` | Minimal CSS (grid layout, button/input styles). |
| `frontend/packages/screen-map/src/review-server/client/components/ScreenView.tsx` | Left pane: metadata + checklists + general-note textarea. |
| `frontend/packages/screen-map/src/review-server/client/components/PreviewPane.tsx` | Right pane: iframe (live/staging) or image (design) or split. |
| `frontend/packages/screen-map/src/review-server/client/components/ChecklistRow.tsx` | One feature row with OK/Note toggle and inline note textarea. |
| `frontend/packages/screen-map/src/review-server/client/components/NavBar.tsx` | Top bar: Prev / `screen X of N` / Next + filter info. |
| `frontend/packages/screen-map/tests/design-source/zip-adapter.test.ts` | Tests for ZipAdapter (manifest read, frame extract, missing frame). |
| `frontend/packages/screen-map/tests/scan.test.ts` | Tests for multi-source candidate enumeration. |
| `frontend/packages/screen-map/tests/grouping.test.ts` | Tests for merge/split/skip grouping decisions. |
| `frontend/packages/screen-map/tests/init-write.test.ts` | Tests for bulk markdown writing (frontmatter shape, idempotent re-write). |
| `frontend/packages/screen-map/tests/edit-context.test.ts` | Tests for `loadScreenContext` summary content. |
| `frontend/packages/screen-map/tests/review-server/server.test.ts` | Integration test: spawn server, hit endpoints, verify markdown mutation. |
| `frontend/packages/screen-map/tests/context.test.ts` | T3 NEW direct unit tests for `context.ts` (slugify, diagram-ref resolver). |
| `frontend/packages/screen-map/tests/fixtures/designs-2026-q2.zip` | Tiny test ZIP with a `manifest.json` and 2 sample frames. |
| `frontend/packages/screen-map/tests/fixtures/use-cases-sample.md` | Tiny `use-cases.md`-shaped fixture for scan tests. |
| `.claude/skills/screen-map-init/SKILL.md` | Init skill: scan + chat-driven grouping + bulk-write. |
| `.claude/skills/screen-map-review/SKILL.md` | Review skill: spawn server, manage browser, handle shutdown. |
| `.claude/skills/screen-edit/SKILL.md` | Per-screen context loader skill. |

### Modified files

| Path | Change |
|------|--------|
| `frontend/packages/screen-map/src/schema.ts` | T1: `IsoDateSchema` accepts both `string` and `Date` via `z.preprocess`. |
| `frontend/packages/screen-map/src/parse.ts` | T1: body normalization regex `^\r?\n` (Windows tolerance). |
| `frontend/packages/screen-map/src/discover.ts` | T2: narrow `catch {}` to ENOENT only; remove redundant `.gitkeep` check. |
| `frontend/packages/screen-map/src/validate.ts` | T2: add comment reserving `severity: 'warning'` for future advisories. |
| `frontend/packages/screen-map/src/context.ts` | T3: `slugify` Unicode-normalizes (`NFKD`) + strips combining marks before regex. |
| `frontend/packages/screen-map/src/cli.ts` | T13: add `init` subcommand. T15: add `edit` subcommand. T22: add `review` subcommand. |
| `frontend/packages/screen-map/src/index.ts` | Re-export new public API: `DesignSource`, `ZipAdapter`, `scanCandidates`, `loadScreenContext`, `bulkWriteScreenMaps`. |
| `frontend/packages/screen-map/package.json` | Add deps: `hono ^4.6.0`, `yauzl-promise ^4.0.0`. |
| `frontend/packages/screen-map/tests/discover.test.ts` | T2: add EACCES propagation test. |
| `frontend/packages/screen-map/tests/parse.test.ts` | T1: add `\r\n` body normalization test. |
| `frontend/packages/screen-map/tests/schema.test.ts` | T1: add `lastReview` accepts `Date` test. |
| `.claude/commands/screens.md` | T23: dispatch all four subcommands (validate, init, edit, review). |
| `scripts/install-hooks.sh` | T3: refresh summary list (current text says "eslint" — should say "Biome lint" plus include all 6 active checks). |

---

## Task 1: Polish — schema date coercion + parse `\r?\n` body normalization

**Files:**
- Modify: `frontend/packages/screen-map/src/schema.ts`
- Modify: `frontend/packages/screen-map/src/parse.ts`
- Modify: `frontend/packages/screen-map/tests/schema.test.ts`
- Modify: `frontend/packages/screen-map/tests/parse.test.ts`

- [ ] **Step 1: Add a failing test for `Date` input in `schema.test.ts`**

Append to `frontend/packages/screen-map/tests/schema.test.ts` inside the existing `describe('ScreenMapFrontmatterSchema', ...)` block:

```typescript
  it('accepts lastReview as a JS Date object (gray-matter coerces unquoted ISO dates)', () => {
    const result = ScreenMapFrontmatterSchema.safeParse({
      id: 'ppt/x',
      name: 'X',
      product: 'ppt',
      implementations: {},
      lastReview: new Date('2026-05-07T00:00:00.000Z'),
    });
    expect(result.success).toBe(true);
    expect(result.data?.lastReview).toBe('2026-05-07');
  });
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test schema
```

Expected: FAIL on the new test with "Expected string, received date" or similar.

- [ ] **Step 3: Replace `IsoDateSchema` in `schema.ts` with a `z.preprocess` form**

In `frontend/packages/screen-map/src/schema.ts`, find the existing internal `IsoDateSchema` declaration:

```typescript
const IsoDateSchema = z
  .string()
  .regex(/^\d{4}-\d{2}-\d{2}$/, { message: 'must be YYYY-MM-DD' });
```

Replace with:

```typescript
const IsoDateSchema = z.preprocess(
  (value) => {
    if (value instanceof Date) {
      // gray-matter (via js-yaml) auto-coerces unquoted ISO dates into Date.
      return value.toISOString().slice(0, 10);
    }
    return value;
  },
  z.string().regex(/^\d{4}-\d{2}-\d{2}$/, { message: 'must be YYYY-MM-DD' }),
);
```

- [ ] **Step 4: Run schema tests to verify they pass**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test schema
```

Expected: PASS, 6 tests (5 existing + 1 new).

- [ ] **Step 5: Add a failing test for `\r\n` body normalization in `parse.test.ts`**

Append to `frontend/packages/screen-map/tests/parse.test.ts` inside the existing `describe('parseScreenMapString', ...)` block:

```typescript
  it('strips a single leading CRLF as well as LF (Windows authoring)', () => {
    const sample = [
      '---',
      'id: ppt/x',
      'name: X',
      'product: ppt',
      'implementations: {}',
      '---',
      '',
      '## Heading',
      'body',
      '',
    ].join('\r\n');
    const screen = parseScreenMapString(sample, '<inline>');
    expect(screen.body.startsWith('## Heading')).toBe(true);
  });
```

- [ ] **Step 6: Run test to verify it fails**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test parse
```

Expected: FAIL because the body retains a leading `\r` (the existing `replace(/^\n/, '')` only matches LF).

- [ ] **Step 7: Update `parse.ts` body normalization regex**

In `frontend/packages/screen-map/src/parse.ts`, find:

```typescript
    body: parsed.content.replace(/^\n/, ''),
```

Replace with:

```typescript
    body: parsed.content.replace(/^\r?\n/, ''),
```

- [ ] **Step 8: Run parse tests to verify they pass**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test parse
```

Expected: PASS, 4 tests (3 existing + 1 new).

- [ ] **Step 9: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add frontend/packages/screen-map/src/schema.ts \
        frontend/packages/screen-map/src/parse.ts \
        frontend/packages/screen-map/tests/schema.test.ts \
        frontend/packages/screen-map/tests/parse.test.ts
git commit -m "fix(screen-map): coerce Date → ISO in IsoDateSchema; tolerate CRLF body"
```

---

## Task 2: Polish — discover ENOENT narrowing, redundant check removal, validate warning comment

**Files:**
- Modify: `frontend/packages/screen-map/src/discover.ts`
- Modify: `frontend/packages/screen-map/src/validate.ts`
- Modify: `frontend/packages/screen-map/tests/discover.test.ts`

- [ ] **Step 1: Add a failing test for non-ENOENT error propagation**

Append to `frontend/packages/screen-map/tests/discover.test.ts` inside the existing `describe('discoverScreenMaps', ...)` block:

```typescript
  it('propagates non-ENOENT readdir errors instead of silently skipping', async () => {
    // chmod 000 the product dir to provoke EACCES on readdir.
    const dir = path.join(tmpRoot, 'ppt');
    await mkdir(dir, { recursive: true });
    const { chmod } = await import('node:fs/promises');
    await chmod(dir, 0o000);
    try {
      await expect(discoverScreenMaps(tmpRoot)).rejects.toThrow();
    } finally {
      await chmod(dir, 0o755);
    }
  });
```

Note: the test self-restores permissions in `finally` so cleanup in `afterEach` works.

- [ ] **Step 2: Run test to verify it fails**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test discover
```

Expected: FAIL — current `catch {}` swallows EACCES, function returns `[]` instead of throwing.

- [ ] **Step 3: Update `discover.ts` to narrow the catch + remove redundant `.gitkeep` check**

In `frontend/packages/screen-map/src/discover.ts`, replace the function body. Current:

```typescript
export async function discoverScreenMaps(rootDir: string): Promise<string[]> {
  const out: string[] = [];

  for (const product of PRODUCT_DIRS) {
    const dir = path.join(rootDir, product);
    let entries;
    try {
      entries = await readdir(dir);
    } catch {
      continue;
    }
    for (const entry of entries) {
      if (IGNORED.has(entry)) continue;
      if (entry === '.gitkeep') continue;
      if (!entry.endsWith('.md')) continue;
      const full = path.join(dir, entry);
      const s = await stat(full);
      if (s.isFile()) out.push(full);
    }
  }
  return out.sort();
}
```

Replace with:

```typescript
export async function discoverScreenMaps(rootDir: string): Promise<string[]> {
  const out: string[] = [];

  for (const product of PRODUCT_DIRS) {
    const dir = path.join(rootDir, product);
    let entries: string[];
    try {
      entries = await readdir(dir);
    } catch (err) {
      if ((err as NodeJS.ErrnoException).code === 'ENOENT') continue;
      throw err;
    }
    for (const entry of entries) {
      if (IGNORED.has(entry)) continue;
      // .gitkeep is filtered by the .md suffix check below; no separate case.
      if (!entry.endsWith('.md')) continue;
      const full = path.join(dir, entry);
      const s = await stat(full);
      if (s.isFile()) out.push(full);
    }
  }
  return out.sort();
}
```

- [ ] **Step 4: Add comment reserving `severity: 'warning'` in validate.ts**

In `frontend/packages/screen-map/src/validate.ts`, find the `ValidationIssue` interface:

```typescript
export interface ValidationIssue {
  severity: 'error' | 'warning';
  path: string;
  message: string;
}
```

Replace with:

```typescript
export interface ValidationIssue {
  /**
   * `'warning'` is reserved for non-blocking advisories (e.g. dead links,
   * missing-but-not-required fields). No current rule produces a warning;
   * the lane exists so future rules can be added without an interface
   * change. CLI prints `warn ` for warnings; pre-commit hook only fails
   * on errors.
   */
  severity: 'error' | 'warning';
  path: string;
  message: string;
}
```

- [ ] **Step 5: Run all package tests to verify they pass**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test
```

Expected: PASS, 26 tests (was 21 in Phase 1; +1 from T1, +4 from T2 — wait, T2 only adds 1 test. After T1 it's 22. After T2 it should be 23). Adjust expectation: roughly 23 tests after Tasks 1 and 2.

- [ ] **Step 6: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add frontend/packages/screen-map/src/discover.ts \
        frontend/packages/screen-map/src/validate.ts \
        frontend/packages/screen-map/tests/discover.test.ts
git commit -m "fix(screen-map): narrow discover catch to ENOENT; document warning lane"
```

---

## Task 3: Polish — context.ts slugify Unicode + new context.test.ts + install-hooks.sh refresh

**Files:**
- Modify: `frontend/packages/screen-map/src/context.ts`
- Create: `frontend/packages/screen-map/tests/context.test.ts`
- Modify: `scripts/install-hooks.sh`

- [ ] **Step 1: Write a failing test for `context.test.ts`**

`frontend/packages/screen-map/tests/context.test.ts`:

```typescript
import { describe, expect, it } from 'vitest';
import { mkdtemp, rm, writeFile, mkdir } from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import { buildValidationContext } from '../src/context.js';

async function withTmpRepo(
  fn: (root: string) => Promise<void>,
): Promise<void> {
  const root = await mkdtemp(path.join(os.tmpdir(), 'screen-map-context-'));
  try {
    await fn(root);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
}

describe('buildValidationContext', () => {
  it('resolves diagram refs that include diacritics in the anchor', async () => {
    await withTmpRepo(async (root) => {
      await mkdir(path.join(root, 'docs'), { recursive: true });
      await writeFile(
        path.join(root, 'docs/seq.md'),
        '## Časový plán\n\nbody.\n',
      );
      const ctx = await buildValidationContext({ repoRoot: root });
      expect(ctx.resolveDiagramRef('docs/seq.md#časový-plán')).toBe(true);
      expect(ctx.resolveDiagramRef('docs/seq.md#casovy-plan')).toBe(true); // ASCII fallback
      expect(ctx.resolveDiagramRef('docs/seq.md#missing')).toBe(false);
    });
  });

  it('returns false when the file does not exist', async () => {
    await withTmpRepo(async (root) => {
      const ctx = await buildValidationContext({ repoRoot: root });
      expect(ctx.resolveDiagramRef('docs/nope.md#anchor')).toBe(false);
    });
  });

  it('returns true for an anchor-less ref pointing to an existing file', async () => {
    await withTmpRepo(async (root) => {
      await mkdir(path.join(root, 'docs'), { recursive: true });
      await writeFile(path.join(root, 'docs/x.md'), '# Heading\n');
      const ctx = await buildValidationContext({ repoRoot: root });
      expect(ctx.resolveDiagramRef('docs/x.md')).toBe(true);
    });
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test context
```

Expected: FAIL on the diacritic-anchor test (current slugify strips `č` to nothing → `časový-plán` becomes `asov-pln`, which doesn't match either form in the test).

- [ ] **Step 3: Replace the `slugify` function in `context.ts`**

In `frontend/packages/screen-map/src/context.ts`, find:

```typescript
function slugify(s: string): string {
  return s
    .toLowerCase()
    .replace(/[^a-z0-9\s-]/g, '')
    .trim()
    .replace(/\s+/g, '-');
}
```

Replace with:

```typescript
function slugify(s: string): string {
  return s
    .toLowerCase()
    .normalize('NFKD')
    // Strip combining marks (diacritics) — preserves base letter.
    .replace(/[̀-ͯ]/g, '')
    .replace(/[^a-z0-9\s-]/g, '')
    .trim()
    .replace(/\s+/g, '-');
}
```

- [ ] **Step 4: Update `extractHeadingSlugs` to also yield the original (non-normalized) slug**

In `frontend/packages/screen-map/src/context.ts`, find:

```typescript
function extractHeadingSlugs(markdown: string): Set<string> {
  const slugs = new Set<string>();
  const headingRe = /^#{1,6}\s+(.+?)\s*$/gm;
  let m: RegExpExecArray | null;
  while ((m = headingRe.exec(markdown))) {
    slugs.add(slugify(m[1]));
  }
  return slugs;
}
```

Replace with:

```typescript
function extractHeadingSlugs(markdown: string): Set<string> {
  const slugs = new Set<string>();
  const headingRe = /^#{1,6}\s+(.+?)\s*$/gm;
  // biome-ignore lint/suspicious/noAssignInExpressions: idiomatic exec loop
  for (let m = headingRe.exec(markdown); m !== null; m = headingRe.exec(markdown)) {
    const text = m[1];
    // ASCII slug (current behavior) AND a Unicode-preserving slug (lowercased,
    // with spaces hyphenated but diacritics intact). Both forms resolve.
    slugs.add(slugify(text));
    slugs.add(text.toLowerCase().trim().replace(/\s+/g, '-'));
  }
  return slugs;
}
```

- [ ] **Step 5: Run context tests to verify they pass**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test context
```

Expected: PASS, 3 tests in `context.test.ts`.

- [ ] **Step 6: Refresh `scripts/install-hooks.sh` summary block**

In `scripts/install-hooks.sh`, find the summary block (around lines 50–56) that lists the active hook checks. The current text mentions "eslint" but the project uses Biome, and the list is missing checks. Replace the block:

```bash
echo "  Pre-commit hook runs:"
echo "    1. Rust formatting check     (cargo fmt --check)"
echo "    2. Kotlin formatting check   (spotless)"
echo "    3. TypeScript/JS lint        (eslint)"
echo "    4. Auto version bump         (patch version)"
```

with:

```bash
echo "  Pre-commit hook runs:"
echo "    1. Rust formatting check        (cargo fmt --check)"
echo "    2. Kotlin formatting check      (spotless)"
echo "    3. TypeScript/JS lint           (Biome)"
echo "    4. TypeScript type check        (tsc --noEmit)"
echo "    5. Screen-map validation        (frontmatter + sitemap + relations)"
echo "    6. Auto version bump            (patch version)"
```

- [ ] **Step 7: Run install-hooks.sh to confirm the summary updates**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
./scripts/install-hooks.sh
```

Expected: prints the new 6-line summary including "Screen-map validation". The hook itself was already installed in Phase 1 Task 12; this only updates the printed summary.

- [ ] **Step 8: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add frontend/packages/screen-map/src/context.ts \
        frontend/packages/screen-map/tests/context.test.ts \
        scripts/install-hooks.sh
git commit -m "fix(screen-map): unicode-preserving slugify; context unit tests; refresh hook summary"
```

---

## Task 4: DesignSource interface

**Files:**
- Create: `frontend/packages/screen-map/src/design-source/index.ts`

- [ ] **Step 1: Write the interface module**

`frontend/packages/screen-map/src/design-source/index.ts`:

```typescript
export interface DesignFrame {
  id: string;
  /** Human-readable name shown in the review UI. */
  name: string;
  /** URL the SPA fetches to render the image. Server resolves to a stream. */
  imageUrl: string;
  /** Pixel dimensions for layout calculations. */
  width: number;
  height: number;
  /** Adapter-specific extras (e.g. ZIP file path, Figma node id). */
  metadata?: Record<string, unknown>;
}

export interface DesignSource {
  /** Stable identifier — used as the `:adapter` segment of /api/designs/. */
  readonly name: string;
  list(): Promise<DesignFrame[]>;
  get(id: string): Promise<DesignFrame | null>;
  /** Optional: stream raw bytes for a frame. Server uses this to proxy images. */
  readBytes?(id: string): Promise<Uint8Array | null>;
}

export interface DesignSourceConfig {
  adapter: 'zip' | 'claude-design';
  /** ZipAdapter: path to .zip file (relative to repoRoot or absolute). */
  file?: string;
  [key: string]: unknown;
}

/**
 * Build a DesignSource from a config record, typically read from frontmatter
 * `designSources[]` or from a screen-map config.
 *
 * Throws on unknown adapter names so misconfiguration is loud.
 */
export async function createDesignSource(
  config: DesignSourceConfig,
  context: { repoRoot: string },
): Promise<DesignSource> {
  switch (config.adapter) {
    case 'zip': {
      if (!config.file) {
        throw new Error('zip adapter requires a "file" config key');
      }
      const { ZipAdapter } = await import('./zip-adapter.js');
      return ZipAdapter.fromFile(config.file, context.repoRoot);
    }
    case 'claude-design': {
      const { ClaudeDesignAdapter } = await import('./claude-design.js');
      return new ClaudeDesignAdapter();
    }
    default: {
      const adapter: never = config.adapter;
      throw new Error(`unknown DesignSource adapter: ${String(adapter)}`);
    }
  }
}
```

- [ ] **Step 2: Confirm typecheck (interface only — no test until ZipAdapter lands)**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map typecheck
```

Expected: FAIL — the dynamic imports reference `./zip-adapter.js` and `./claude-design.js` which don't exist yet. This is intentional; Tasks 5 and 6 add them. Confirm errors are ONLY "Cannot find module" for those two paths.

- [ ] **Step 3: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add frontend/packages/screen-map/src/design-source/index.ts
git commit -m "feat(screen-map): DesignSource interface + adapter factory"
```

---

## Task 5: ZipAdapter

**Files:**
- Modify: `frontend/packages/screen-map/package.json`
- Create: `frontend/packages/screen-map/src/design-source/zip-adapter.ts`
- Create: `frontend/packages/screen-map/tests/design-source/zip-adapter.test.ts`
- Create: `frontend/packages/screen-map/tests/fixtures/designs-2026-q2.zip` (binary fixture, generated by a one-off Node script in Step 1)

- [ ] **Step 1: Generate the test fixture ZIP**

Run this one-off script from the repo root to produce the test fixture:

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
node -e '
import("yauzl-promise").catch(() => {
  console.error("yauzl-promise not yet installed; run pnpm install first");
  process.exit(1);
});
import("node:fs").then(async (fs) => {
  const archiver = (await import("archiver")).default;
  const out = fs.createWriteStream("frontend/packages/screen-map/tests/fixtures/designs-2026-q2.zip");
  const a = archiver("zip", { zlib: { level: 9 } });
  a.pipe(out);
  // 2 fake frames + manifest.
  const manifest = JSON.stringify({
    frames: [
      { id: "building-detail-v3-web", name: "Building Detail (Web v3)", file: "frames/building-detail-v3-web.png", width: 1440, height: 900 },
      { id: "building-detail-v3-mobile", name: "Building Detail (Mobile v3)", file: "frames/building-detail-v3-mobile.png", width: 375, height: 812 }
    ]
  }, null, 2);
  a.append(manifest, { name: "manifest.json" });
  // Tiny 1x1 PNG for each frame (8 bytes header + minimal IHDR/IDAT/IEND ~67 bytes).
  const pngBytes = Buffer.from("89504e470d0a1a0a0000000d49484452000000010000000108020000009077532de0000000174944415478da636060606060606060606060606060000000050001b04a40d40000000049454e44ae426082", "hex");
  a.append(pngBytes, { name: "frames/building-detail-v3-web.png" });
  a.append(pngBytes, { name: "frames/building-detail-v3-mobile.png" });
  await a.finalize();
  await new Promise(r => out.on("close", r));
  console.log("fixture written");
});
' 2>&1 | tail -5
```

If `archiver` is not installed: `cd frontend && pnpm add -D -w archiver` first, then retry. Or hand-craft the ZIP via the `zip` CLI:

```bash
mkdir -p /tmp/zip-fixture/frames
echo '{"frames":[{"id":"building-detail-v3-web","name":"Building Detail (Web v3)","file":"frames/building-detail-v3-web.png","width":1440,"height":900},{"id":"building-detail-v3-mobile","name":"Building Detail (Mobile v3)","file":"frames/building-detail-v3-mobile.png","width":375,"height":812}]}' > /tmp/zip-fixture/manifest.json
# 67-byte 1x1 PNG:
printf '\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x02\x00\x00\x00\x90wS\xde\x00\x00\x00\x17IDATx\xdaccccccccccccccccccc\x00\x00\x00\x05\x00\x01\xb0J@\xd4\x00\x00\x00\x00IEND\xaeB`\x82' > /tmp/zip-fixture/frames/building-detail-v3-web.png
cp /tmp/zip-fixture/frames/building-detail-v3-web.png /tmp/zip-fixture/frames/building-detail-v3-mobile.png
( cd /tmp/zip-fixture && zip -r /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend/packages/screen-map/tests/fixtures/designs-2026-q2.zip manifest.json frames/ )
```

Verify:

```bash
unzip -l /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend/packages/screen-map/tests/fixtures/designs-2026-q2.zip
```

Expected: lists `manifest.json`, `frames/building-detail-v3-web.png`, `frames/building-detail-v3-mobile.png`.

- [ ] **Step 2: Add `yauzl-promise` to package.json deps**

In `frontend/packages/screen-map/package.json`, add `"yauzl-promise": "^4.0.0"` to `dependencies` (alphabetical):

```json
  "dependencies": {
    "@ppt/sitemap": "workspace:*",
    "commander": "^12.1.0",
    "gray-matter": "^4.0.3",
    "yauzl-promise": "^4.0.0",
    "zod": "^3.23.8"
  },
```

Run `pnpm install`:

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm install
```

Expected: yauzl-promise added.

- [ ] **Step 3: Write a failing test for ZipAdapter**

`frontend/packages/screen-map/tests/design-source/zip-adapter.test.ts`:

```typescript
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { describe, expect, it } from 'vitest';
import { ZipAdapter } from '../../src/design-source/zip-adapter.js';

const here = path.dirname(fileURLToPath(import.meta.url));
const fixture = path.join(here, '..', 'fixtures', 'designs-2026-q2.zip');

describe('ZipAdapter', () => {
  it('reads manifest.json and lists frames', async () => {
    const adapter = await ZipAdapter.fromFile(fixture, '/');
    const frames = await adapter.list();
    expect(frames).toHaveLength(2);
    const webFrame = frames.find((f) => f.id === 'building-detail-v3-web');
    expect(webFrame).toBeDefined();
    expect(webFrame?.width).toBe(1440);
    expect(webFrame?.imageUrl).toContain('zip/building-detail-v3-web');
  });

  it('returns null for an unknown frame id', async () => {
    const adapter = await ZipAdapter.fromFile(fixture, '/');
    const frame = await adapter.get('does-not-exist');
    expect(frame).toBeNull();
  });

  it('streams frame bytes via readBytes', async () => {
    const adapter = await ZipAdapter.fromFile(fixture, '/');
    const bytes = await adapter.readBytes!('building-detail-v3-web');
    expect(bytes).toBeInstanceOf(Uint8Array);
    // PNG signature.
    expect(bytes![0]).toBe(0x89);
    expect(bytes![1]).toBe(0x50);
  });

  it('returns null bytes for unknown frame', async () => {
    const adapter = await ZipAdapter.fromFile(fixture, '/');
    const bytes = await adapter.readBytes!('does-not-exist');
    expect(bytes).toBeNull();
  });
});
```

- [ ] **Step 4: Run test to verify it fails**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test zip-adapter
```

Expected: FAIL with "Cannot find module '../../src/design-source/zip-adapter.js'".

- [ ] **Step 5: Implement `zip-adapter.ts`**

`frontend/packages/screen-map/src/design-source/zip-adapter.ts`:

```typescript
import path from 'node:path';
import { Readable } from 'node:stream';
import { open as openZip } from 'yauzl-promise';
import type { DesignFrame, DesignSource } from './index.js';

interface ManifestEntry {
  id: string;
  name: string;
  file: string;
  width: number;
  height: number;
}

interface Manifest {
  frames: ManifestEntry[];
}

export class ZipAdapter implements DesignSource {
  readonly name = 'zip';

  private constructor(
    private readonly zipPath: string,
    private readonly manifest: Manifest,
    /** in-memory byte cache; first read populates, later reads hit. */
    private readonly cache: Map<string, Uint8Array>,
  ) {}

  static async fromFile(filePart: string, repoRoot: string): Promise<ZipAdapter> {
    const zipPath = path.isAbsolute(filePart)
      ? filePart
      : path.join(repoRoot, filePart);
    const zip = await openZip(zipPath);
    let manifest: Manifest | null = null;
    try {
      for await (const entry of zip) {
        if (entry.filename === 'manifest.json') {
          const stream = await entry.openReadStream();
          const buf = await streamToBuffer(stream);
          manifest = JSON.parse(buf.toString('utf8')) as Manifest;
          break;
        }
      }
    } finally {
      await zip.close();
    }
    if (!manifest) {
      throw new Error(`ZipAdapter: ${zipPath} has no manifest.json at the root`);
    }
    if (!Array.isArray(manifest.frames)) {
      throw new Error(`ZipAdapter: manifest.frames must be an array`);
    }
    return new ZipAdapter(zipPath, manifest, new Map());
  }

  async list(): Promise<DesignFrame[]> {
    return this.manifest.frames.map((f) => this.toFrame(f));
  }

  async get(id: string): Promise<DesignFrame | null> {
    const entry = this.manifest.frames.find((f) => f.id === id);
    return entry ? this.toFrame(entry) : null;
  }

  async readBytes(id: string): Promise<Uint8Array | null> {
    const cached = this.cache.get(id);
    if (cached) return cached;
    const entry = this.manifest.frames.find((f) => f.id === id);
    if (!entry) return null;

    const zip = await openZip(this.zipPath);
    try {
      for await (const e of zip) {
        if (e.filename === entry.file) {
          const stream = await e.openReadStream();
          const buf = await streamToBuffer(stream);
          const bytes = new Uint8Array(buf);
          this.cache.set(id, bytes);
          return bytes;
        }
      }
    } finally {
      await zip.close();
    }
    return null;
  }

  private toFrame(entry: ManifestEntry): DesignFrame {
    return {
      id: entry.id,
      name: entry.name,
      imageUrl: `/api/designs/zip/${encodeURIComponent(entry.id)}`,
      width: entry.width,
      height: entry.height,
      metadata: { sourceFile: this.zipPath, frameFile: entry.file },
    };
  }
}

async function streamToBuffer(stream: Readable): Promise<Buffer> {
  const chunks: Buffer[] = [];
  for await (const chunk of stream) {
    chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk));
  }
  return Buffer.concat(chunks);
}
```

- [ ] **Step 6: Run zip-adapter tests to verify they pass**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test zip-adapter
```

Expected: PASS, 4 tests.

- [ ] **Step 7: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add frontend/packages/screen-map/src/design-source/zip-adapter.ts \
        frontend/packages/screen-map/tests/design-source/zip-adapter.test.ts \
        frontend/packages/screen-map/tests/fixtures/designs-2026-q2.zip \
        frontend/packages/screen-map/package.json \
        frontend/pnpm-lock.yaml
git commit -m "feat(screen-map): ZipAdapter reads manifest.json + lazy-extracts frames"
```

---

## Task 6: ClaudeDesignAdapter stub

**Files:**
- Create: `frontend/packages/screen-map/src/design-source/claude-design.ts`

- [ ] **Step 1: Write the stub**

`frontend/packages/screen-map/src/design-source/claude-design.ts`:

```typescript
import type { DesignFrame, DesignSource } from './index.js';

class NotImplementedError extends Error {
  constructor(method: string) {
    super(
      `ClaudeDesignAdapter.${method}() is a Phase-2 stub. Implementation deferred until the Claude Design API contract is finalised. See docs/superpowers/specs/2026-05-07-screen-map-system-design.md Section 7.2.`,
    );
    this.name = 'NotImplementedError';
  }
}

export class ClaudeDesignAdapter implements DesignSource {
  readonly name = 'claude-design';

  list(): Promise<DesignFrame[]> {
    throw new NotImplementedError('list');
  }

  get(_id: string): Promise<DesignFrame | null> {
    throw new NotImplementedError('get');
  }
}
```

- [ ] **Step 2: Verify typecheck**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map typecheck
```

Expected: PASS, 0 errors. After Tasks 4 + 5 + 6 the package fully compiles again.

- [ ] **Step 3: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add frontend/packages/screen-map/src/design-source/claude-design.ts
git commit -m "feat(screen-map): ClaudeDesignAdapter stub (NotImplementedError)"
```

---

## Task 7: scan.ts — sitemap + use-cases + epics sources

**Files:**
- Create: `frontend/packages/screen-map/src/scan.ts`
- Create: `frontend/packages/screen-map/tests/scan.test.ts`
- Create: `frontend/packages/screen-map/tests/fixtures/use-cases-sample.md`
- Create: `frontend/packages/screen-map/tests/fixtures/epic-sample/EPIC-001-user-mgmt.md`

- [ ] **Step 1: Create the use-cases fixture**

`frontend/packages/screen-map/tests/fixtures/use-cases-sample.md`:

```markdown
# Use Cases (test fixture)

## UC-12 Building Management
- UC-12.1 Create building
- UC-12.2 Edit building
- UC-12.3 Delete building

## UC-13 Unit Management
- UC-13.1 List units
- UC-13.2 Edit unit
```

- [ ] **Step 2: Create the epic fixture**

`frontend/packages/screen-map/tests/fixtures/epic-sample/EPIC-001-user-mgmt.md`:

```markdown
# Epic-001: User Management

## Stories
- STORY-001-001 Register
- STORY-001-002 Login
```

- [ ] **Step 3: Write a failing test for `scan.ts`**

`frontend/packages/screen-map/tests/scan.test.ts`:

```typescript
import { describe, expect, it } from 'vitest';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { scanCandidates } from '../src/scan.js';

const here = path.dirname(fileURLToPath(import.meta.url));
const fixturesDir = path.join(here, 'fixtures');

describe('scanCandidates', () => {
  it('returns sitemap routes/screens for the requested product', async () => {
    const candidates = await scanCandidates({
      product: 'ppt',
      repoRoot: '/',  // not used for sitemap source
      sources: { sitemap: true, useCases: false, epics: false, designSource: undefined, userAdd: [] },
    });
    expect(candidates.length).toBeGreaterThan(0);
    expect(candidates.every((c) => c.product === 'ppt')).toBe(true);
    expect(candidates.some((c) => c.source === 'sitemap')).toBe(true);
  });

  it('extracts UC IDs from a use-cases fixture', async () => {
    const candidates = await scanCandidates({
      product: 'ppt',
      repoRoot: fixturesDir,
      useCasesFile: path.join(fixturesDir, 'use-cases-sample.md'),
      sources: { sitemap: false, useCases: true, epics: false, designSource: undefined, userAdd: [] },
    });
    const ucIds = candidates.flatMap((c) => c.useCases ?? []);
    expect(ucIds).toContain('UC-12');
    expect(ucIds).toContain('UC-12.1');
    expect(ucIds).toContain('UC-13');
  });

  it('extracts Epic IDs from a directory of EPIC-*.md files', async () => {
    const candidates = await scanCandidates({
      product: 'ppt',
      repoRoot: fixturesDir,
      epicsDir: path.join(fixturesDir, 'epic-sample'),
      sources: { sitemap: false, useCases: false, epics: true, designSource: undefined, userAdd: [] },
    });
    const epics = candidates.flatMap((c) => c.epics ?? []);
    expect(epics).toContain('Epic-001');
  });

  it('includes user-add entries with source: "user"', async () => {
    const candidates = await scanCandidates({
      product: 'ppt',
      repoRoot: '/',
      sources: { sitemap: false, useCases: false, epics: false, designSource: undefined, userAdd: ['Faults assignment modal', 'Inventory dashboard'] },
    });
    expect(candidates).toHaveLength(2);
    expect(candidates.every((c) => c.source === 'user')).toBe(true);
    expect(candidates[0].name).toBe('Faults assignment modal');
  });
});
```

- [ ] **Step 4: Run test to verify it fails**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test scan
```

Expected: FAIL with "Cannot find module '../src/scan.js'".

- [ ] **Step 5: Implement `scan.ts`**

`frontend/packages/screen-map/src/scan.ts`:

```typescript
import { readFile, readdir } from 'node:fs/promises';
import path from 'node:path';
import {
  pptWebRoutes,
  realityWebRoutes,
  mobileScreens,
} from '@ppt/sitemap';
import type { DesignSource } from './design-source/index.js';
import type { Platform, Product } from './types.js';

export interface CandidateScreen {
  /** Heuristic id; user can override during interactive grouping. */
  id: string;
  name: string;
  product: Product;
  source: 'sitemap' | 'use-cases' | 'epics' | 'design' | 'user';
  /** Sitemap-side IDs that this candidate ties to (for sitemap sources). */
  sitemapRefs?: Partial<Record<Platform, string>>;
  useCases?: string[];
  epics?: string[];
  /** DesignSource frame id, if source === 'design'. */
  frameId?: string;
}

export interface ScanOptions {
  product: Product;
  repoRoot: string;
  /** Path to docs/use-cases.md; defaults to `<repoRoot>/docs/use-cases.md`. */
  useCasesFile?: string;
  /** Path to epic dir; defaults to `<repoRoot>/docs/epics`. */
  epicsDir?: string;
  /** DesignSource instance to enumerate frames from (caller constructs it). */
  designSource?: DesignSource;
  sources: {
    sitemap: boolean;
    useCases: boolean;
    epics: boolean;
    designSource: DesignSource | undefined;
    userAdd: string[];
  };
}

export async function scanCandidates(
  opts: ScanOptions,
): Promise<CandidateScreen[]> {
  const out: CandidateScreen[] = [];
  const product = opts.product;

  if (opts.sources.sitemap) {
    out.push(...scanSitemap(product));
  }
  if (opts.sources.useCases) {
    const file =
      opts.useCasesFile ?? path.join(opts.repoRoot, 'docs/use-cases.md');
    out.push(...(await scanUseCases(file, product)));
  }
  if (opts.sources.epics) {
    const dir = opts.epicsDir ?? path.join(opts.repoRoot, 'docs/epics');
    out.push(...(await scanEpics(dir, product)));
  }
  if (opts.sources.designSource) {
    out.push(...(await scanDesignSource(opts.sources.designSource, product)));
  }
  for (const name of opts.sources.userAdd) {
    out.push({
      id: `${product}/${slugifyName(name)}`,
      name,
      product,
      source: 'user',
    });
  }
  return out;
}

function scanSitemap(product: Product): CandidateScreen[] {
  const out: CandidateScreen[] = [];
  if (product === 'ppt') {
    for (const r of pptWebRoutes) {
      out.push({
        id: `ppt/${slugifyName(r.name ?? r.id)}`,
        name: r.name ?? r.id,
        product: 'ppt',
        source: 'sitemap',
        sitemapRefs: { 'ppt-web': r.id },
      });
    }
    for (const s of mobileScreens) {
      out.push({
        id: `ppt/${slugifyName(s.name ?? s.screenName ?? s.id)}`,
        name: s.name ?? s.screenName ?? s.id,
        product: 'ppt',
        source: 'sitemap',
        sitemapRefs: { mobile: s.id },
      });
    }
  } else {
    for (const r of realityWebRoutes) {
      out.push({
        id: `reality/${slugifyName(r.name ?? r.id)}`,
        name: r.name ?? r.id,
        product: 'reality',
        source: 'sitemap',
        sitemapRefs: { 'reality-web': r.id },
      });
    }
    // mobile-native (KMP) is not in @ppt/sitemap as routes; defer to user-add.
  }
  return out;
}

async function scanUseCases(
  file: string,
  product: Product,
): Promise<CandidateScreen[]> {
  let content: string;
  try {
    content = await readFile(file, 'utf8');
  } catch {
    return [];
  }
  // Match both `UC-NN` (category) and `UC-NN.M` (story).
  const matches = content.matchAll(/\bUC-(\d+(?:\.\d+)?)\b/g);
  const ucIds = new Set<string>();
  for (const m of matches) {
    ucIds.add(`UC-${m[1]}`);
  }
  // One synthetic candidate per UC id; user merges into concepts during grouping.
  return [...ucIds].map((id) => ({
    id: `${product}/${slugifyName(id)}`,
    name: id,
    product,
    source: 'use-cases' as const,
    useCases: [id],
  }));
}

async function scanEpics(
  dir: string,
  product: Product,
): Promise<CandidateScreen[]> {
  let entries: string[];
  try {
    entries = await readdir(dir);
  } catch {
    return [];
  }
  const epics = entries
    .map((entry) => entry.match(/^EPIC-(\d+)/i)?.[1])
    .filter((id): id is string => Boolean(id));
  return [...new Set(epics)].map((num) => ({
    id: `${product}/epic-${num}`,
    name: `Epic-${num}`,
    product,
    source: 'epics' as const,
    epics: [`Epic-${num}`],
  }));
}

async function scanDesignSource(
  source: DesignSource,
  product: Product,
): Promise<CandidateScreen[]> {
  const frames = await source.list();
  return frames.map((frame) => ({
    id: `${product}/${slugifyName(frame.name)}`,
    name: frame.name,
    product,
    source: 'design' as const,
    frameId: frame.id,
  }));
}

function slugifyName(s: string): string {
  return s
    .toLowerCase()
    .normalize('NFKD')
    .replace(/[̀-ͯ]/g, '')
    .replace(/[^a-z0-9\s-]/g, '')
    .trim()
    .replace(/\s+/g, '-')
    .replace(/-+/g, '-');
}
```

- [ ] **Step 6: Run scan tests to verify they pass**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test scan
```

Expected: PASS, 4 tests.

- [ ] **Step 7: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add frontend/packages/screen-map/src/scan.ts \
        frontend/packages/screen-map/tests/scan.test.ts \
        frontend/packages/screen-map/tests/fixtures/use-cases-sample.md \
        frontend/packages/screen-map/tests/fixtures/epic-sample/
git commit -m "feat(screen-map): scanCandidates (sitemap + use-cases + epics + design + user)"
```

---

## Task 8: grouping.ts — apply user grouping decisions

**Files:**
- Create: `frontend/packages/screen-map/src/grouping.ts`
- Create: `frontend/packages/screen-map/tests/grouping.test.ts`

- [ ] **Step 1: Write a failing test**

`frontend/packages/screen-map/tests/grouping.test.ts`:

```typescript
import { describe, expect, it } from 'vitest';
import { mergeCandidates, type GroupingDecision } from '../src/grouping.js';
import type { CandidateScreen } from '../src/scan.js';

const candidates: CandidateScreen[] = [
  { id: 'ppt/foo', name: 'Foo', product: 'ppt', source: 'sitemap' },
  { id: 'ppt/bar', name: 'Bar', product: 'ppt', source: 'sitemap' },
  { id: 'ppt/baz', name: 'Baz', product: 'ppt', source: 'use-cases', useCases: ['UC-12'] },
];

describe('mergeCandidates', () => {
  it('passes candidates through unchanged when there are no decisions', () => {
    const result = mergeCandidates(candidates, []);
    expect(result).toHaveLength(3);
  });

  it('merges multiple candidates into one concept by id', () => {
    const decisions: GroupingDecision[] = [
      { type: 'merge', from: ['ppt/foo', 'ppt/bar'], into: 'ppt/foo-bar', name: 'Foo Bar' },
    ];
    const result = mergeCandidates(candidates, decisions);
    expect(result).toHaveLength(2);
    const merged = result.find((c) => c.id === 'ppt/foo-bar');
    expect(merged?.name).toBe('Foo Bar');
  });

  it('skips candidates listed in skip decisions', () => {
    const decisions: GroupingDecision[] = [{ type: 'skip', ids: ['ppt/baz'] }];
    const result = mergeCandidates(candidates, decisions);
    expect(result).toHaveLength(2);
    expect(result.find((c) => c.id === 'ppt/baz')).toBeUndefined();
  });

  it('renames a candidate via decision', () => {
    const decisions: GroupingDecision[] = [
      { type: 'rename', from: 'ppt/baz', to: 'ppt/building-management', name: 'Building Management' },
    ];
    const result = mergeCandidates(candidates, decisions);
    const renamed = result.find((c) => c.id === 'ppt/building-management');
    expect(renamed?.name).toBe('Building Management');
    expect(renamed?.useCases).toEqual(['UC-12']); // preserved from baz
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test grouping
```

Expected: FAIL with "Cannot find module '../src/grouping.js'".

- [ ] **Step 3: Implement `grouping.ts`**

`frontend/packages/screen-map/src/grouping.ts`:

```typescript
import type { CandidateScreen } from './scan.js';

export type GroupingDecision =
  | { type: 'merge'; from: string[]; into: string; name?: string }
  | { type: 'skip'; ids: string[] }
  | { type: 'rename'; from: string; to: string; name?: string };

export function mergeCandidates(
  candidates: CandidateScreen[],
  decisions: GroupingDecision[],
): CandidateScreen[] {
  let working = [...candidates];

  for (const decision of decisions) {
    if (decision.type === 'skip') {
      const skipSet = new Set(decision.ids);
      working = working.filter((c) => !skipSet.has(c.id));
      continue;
    }
    if (decision.type === 'rename') {
      const target = working.find((c) => c.id === decision.from);
      if (!target) continue;
      target.id = decision.to;
      if (decision.name) target.name = decision.name;
      continue;
    }
    if (decision.type === 'merge') {
      const fromSet = new Set(decision.from);
      const merged = working.filter((c) => fromSet.has(c.id));
      if (merged.length === 0) continue;
      const combined: CandidateScreen = {
        id: decision.into,
        name: decision.name ?? merged[0].name,
        product: merged[0].product,
        source: merged[0].source,
        sitemapRefs: mergeSitemapRefs(merged),
        useCases: dedupe(merged.flatMap((m) => m.useCases ?? [])),
        epics: dedupe(merged.flatMap((m) => m.epics ?? [])),
        frameId: merged.find((m) => m.frameId)?.frameId,
      };
      working = working.filter((c) => !fromSet.has(c.id));
      working.push(combined);
    }
  }

  return working;
}

function mergeSitemapRefs(
  merged: CandidateScreen[],
): CandidateScreen['sitemapRefs'] {
  const result: NonNullable<CandidateScreen['sitemapRefs']> = {};
  for (const c of merged) {
    if (!c.sitemapRefs) continue;
    for (const [platform, id] of Object.entries(c.sitemapRefs)) {
      if (id) result[platform as keyof typeof result] = id;
    }
  }
  return Object.keys(result).length > 0 ? result : undefined;
}

function dedupe<T>(arr: T[]): T[] | undefined {
  if (arr.length === 0) return undefined;
  return [...new Set(arr)];
}
```

- [ ] **Step 4: Run grouping tests to verify they pass**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test grouping
```

Expected: PASS, 4 tests.

- [ ] **Step 5: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add frontend/packages/screen-map/src/grouping.ts \
        frontend/packages/screen-map/tests/grouping.test.ts
git commit -m "feat(screen-map): mergeCandidates applies user merge/skip/rename decisions"
```

---

## Task 9: init-write.ts — bulk markdown writer

**Files:**
- Create: `frontend/packages/screen-map/src/init-write.ts`
- Create: `frontend/packages/screen-map/tests/init-write.test.ts`

- [ ] **Step 1: Write a failing test**

`frontend/packages/screen-map/tests/init-write.test.ts`:

```typescript
import { describe, expect, it, beforeEach, afterEach } from 'vitest';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import { bulkWriteScreenMaps } from '../src/init-write.js';
import { parseScreenMap } from '../src/parse.js';
import type { CandidateScreen } from '../src/scan.js';

let tmpRoot: string;
beforeEach(async () => {
  tmpRoot = await mkdtemp(path.join(os.tmpdir(), 'screen-map-init-'));
});
afterEach(async () => {
  await rm(tmpRoot, { recursive: true, force: true });
});

describe('bulkWriteScreenMaps', () => {
  it('writes one markdown file per concept under <screensDir>/<product>/', async () => {
    const concepts: CandidateScreen[] = [
      {
        id: 'ppt/building-detail',
        name: 'Building Detail',
        product: 'ppt',
        source: 'sitemap',
        sitemapRefs: { 'ppt-web': 'ppt-building-detail' },
        useCases: ['UC-12'],
      },
      {
        id: 'reality/property-detail',
        name: 'Property Detail',
        product: 'reality',
        source: 'sitemap',
        sitemapRefs: { 'reality-web': 'reality-property-detail' },
      },
    ];
    const written = await bulkWriteScreenMaps(concepts, tmpRoot);
    expect(written).toHaveLength(2);

    const ppt = path.join(tmpRoot, 'ppt/building-detail.md');
    const screen = await parseScreenMap(ppt);
    expect(screen.frontmatter.id).toBe('ppt/building-detail');
    expect(screen.frontmatter.name).toBe('Building Detail');
    expect(screen.frontmatter.product).toBe('ppt');
    expect(screen.frontmatter.sitemapRefs?.['ppt-web']).toBe('ppt-building-detail');
    expect(screen.frontmatter.implementations['ppt-web']?.buildStatus).toBe('shipped');
    expect(screen.frontmatter.useCases).toEqual(['UC-12']);
    expect(screen.body).toContain('## Functionality Checklist');
    expect(screen.body).toContain('## Agent Log');
    expect(screen.body).toContain('init: created from scan');
  });

  it('marks design-sourced concepts as planned (not shipped)', async () => {
    const concepts: CandidateScreen[] = [
      {
        id: 'ppt/redesign-foo',
        name: 'Redesign Foo',
        product: 'ppt',
        source: 'design',
        frameId: 'foo-v3',
      },
    ];
    await bulkWriteScreenMaps(concepts, tmpRoot);
    const ppt = path.join(tmpRoot, 'ppt/redesign-foo.md');
    const screen = await parseScreenMap(ppt);
    expect(screen.frontmatter.implementations['ppt-web']?.buildStatus).toBe('planned');
    expect(screen.frontmatter.designSources).toBeDefined();
    expect(screen.frontmatter.designSources?.[0].frame).toBe('foo-v3');
  });

  it('refuses to overwrite an existing file unless force=true', async () => {
    const c: CandidateScreen[] = [
      { id: 'ppt/foo', name: 'Foo', product: 'ppt', source: 'user' },
    ];
    await bulkWriteScreenMaps(c, tmpRoot);
    await expect(bulkWriteScreenMaps(c, tmpRoot)).rejects.toThrow(/already exists/);
    // With force: succeed.
    const written = await bulkWriteScreenMaps(c, tmpRoot, { force: true });
    expect(written).toHaveLength(1);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test init-write
```

Expected: FAIL with "Cannot find module '../src/init-write.js'".

- [ ] **Step 3: Implement `init-write.ts`**

`frontend/packages/screen-map/src/init-write.ts`:

```typescript
import { existsSync } from 'node:fs';
import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import type { CandidateScreen } from './scan.js';
import type {
  Platform,
  Product,
  ScreenMapFrontmatter,
} from './types.js';
import { writeScreenMapString } from './write.js';

export interface BulkWriteOptions {
  force?: boolean;
}

export async function bulkWriteScreenMaps(
  concepts: CandidateScreen[],
  screensDir: string,
  options: BulkWriteOptions = {},
): Promise<string[]> {
  const written: string[] = [];
  for (const concept of concepts) {
    const slug = concept.id.split('/')[1];
    if (!slug) {
      throw new Error(`invalid concept id "${concept.id}" (no slug)`);
    }
    const dir = path.join(screensDir, concept.product);
    await mkdir(dir, { recursive: true });
    const file = path.join(dir, `${slug}.md`);
    if (existsSync(file) && !options.force) {
      throw new Error(`${file} already exists; pass force=true to overwrite`);
    }
    const screen = {
      filePath: file,
      frontmatter: buildFrontmatter(concept),
      body: buildBody(concept),
    };
    const serialized = writeScreenMapString(screen);
    await writeFile(file, serialized, 'utf8');
    written.push(file);
  }
  return written;
}

function buildFrontmatter(c: CandidateScreen): ScreenMapFrontmatter {
  const isDesigned = c.source === 'design';
  const buildStatus = isDesigned ? 'planned' : 'shipped';
  const apiStatus = isDesigned ? 'stub' : 'partial';
  const platforms = platformsForProduct(c.product);
  const implementations: ScreenMapFrontmatter['implementations'] = {};
  for (const p of platforms) {
    implementations[p] = {
      buildStatus,
      redesignStatus: isDesigned ? 'in-progress' : 'not-started',
      apiStatus,
    };
  }
  return {
    id: c.id,
    name: c.name,
    product: c.product,
    sitemapRefs: c.sitemapRefs,
    implementations,
    useCases: c.useCases,
    epics: c.epics,
    designSources: c.frameId
      ? [{ adapter: 'zip', frame: c.frameId }]
      : undefined,
  };
}

function buildBody(c: CandidateScreen): string {
  const today = new Date().toISOString().slice(0, 10);
  return [
    '## Functionality Checklist',
    '',
    '<!-- tag with [w] / [m] / [w,m] / [-] -->',
    '- [ ] [w,m] (none yet)',
    '',
    '## States',
    '',
    '- **Empty**:',
    '- **Loading**:',
    '- **Error**:',
    '',
    '## Notes',
    '',
    '### Broader context',
    '',
    '### Specific (recent)',
    '',
    '## Agent Log',
    '',
    '<!-- newest entries on top -->',
    '',
    `- ${today} — init: created from scan (source: ${c.source})`,
    '',
  ].join('\n');
}

function platformsForProduct(product: Product): Platform[] {
  return product === 'ppt'
    ? ['ppt-web', 'mobile']
    : ['reality-web', 'mobile-native'];
}
```

- [ ] **Step 4: Run init-write tests to verify they pass**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test init-write
```

Expected: PASS, 3 tests.

- [ ] **Step 5: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add frontend/packages/screen-map/src/init-write.ts \
        frontend/packages/screen-map/tests/init-write.test.ts
git commit -m "feat(screen-map): bulkWriteScreenMaps generates per-concept markdown files"
```

---

## Task 10: edit-context.ts — single-screen context loader

**Files:**
- Create: `frontend/packages/screen-map/src/edit-context.ts`
- Create: `frontend/packages/screen-map/tests/edit-context.test.ts`

- [ ] **Step 1: Write a failing test**

`frontend/packages/screen-map/tests/edit-context.test.ts`:

```typescript
import { describe, expect, it, beforeEach, afterEach } from 'vitest';
import { mkdtemp, mkdir, rm } from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import { bulkWriteScreenMaps } from '../src/init-write.js';
import { loadScreenContext } from '../src/edit-context.js';

let tmpRoot: string;
beforeEach(async () => {
  tmpRoot = await mkdtemp(path.join(os.tmpdir(), 'edit-ctx-'));
  await mkdir(path.join(tmpRoot, 'docs/screens'), { recursive: true });
  await bulkWriteScreenMaps(
    [
      { id: 'ppt/buildings-list', name: 'Buildings List', product: 'ppt', source: 'sitemap', sitemapRefs: { 'ppt-web': 'ppt-buildings-list' } },
      { id: 'ppt/building-detail', name: 'Building Detail', product: 'ppt', source: 'sitemap', sitemapRefs: { 'ppt-web': 'ppt-building-detail' } },
    ],
    path.join(tmpRoot, 'docs/screens'),
  );
});
afterEach(async () => {
  await rm(tmpRoot, { recursive: true, force: true });
});

describe('loadScreenContext', () => {
  it('returns a markdown summary including status, related screens, recent agent log', async () => {
    const summary = await loadScreenContext('ppt/building-detail', {
      repoRoot: tmpRoot,
      includePlaywright: false,
    });
    expect(summary).toContain('# ppt/building-detail');
    expect(summary).toContain('Building Detail');
    expect(summary).toContain('buildStatus: shipped');
    expect(summary).toContain('## Recent Agent Log');
    expect(summary).toContain('init: created from scan');
  });

  it('throws when the screen id does not exist', async () => {
    await expect(
      loadScreenContext('ppt/nope', { repoRoot: tmpRoot, includePlaywright: false }),
    ).rejects.toThrow(/not found/);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test edit-context
```

Expected: FAIL with "Cannot find module '../src/edit-context.js'".

- [ ] **Step 3: Implement `edit-context.ts`**

`frontend/packages/screen-map/src/edit-context.ts`:

```typescript
import path from 'node:path';
import { discoverScreenMaps } from './discover.js';
import { parseScreenMap } from './parse.js';
import type { ScreenMap } from './types.js';

export interface LoadScreenContextOptions {
  repoRoot: string;
  /** Run Playwright on a known route to capture a screenshot path. */
  includePlaywright: boolean;
  /** Override default screens dir. */
  screensDir?: string;
}

export async function loadScreenContext(
  id: string,
  options: LoadScreenContextOptions,
): Promise<string> {
  const screensDir =
    options.screensDir ?? path.join(options.repoRoot, 'docs/screens');
  const all = await discoverScreenMaps(screensDir);
  const parsed = await Promise.all(all.map((f) => parseScreenMap(f).catch(() => null)));
  const screens = parsed.filter((s): s is ScreenMap => s !== null);
  const target = screens.find((s) => s.frontmatter.id === id);
  if (!target) {
    throw new Error(`screen "${id}" not found under ${screensDir}`);
  }
  const related = (target.frontmatter.relatedScreens ?? []).map((r) => {
    const found = screens.find((s) => s.frontmatter.id === r.id);
    return { ...r, name: found?.frontmatter.name };
  });
  return formatSummary(target, related);
}

function formatSummary(
  screen: ScreenMap,
  related: { id: string; rel: string; name?: string }[],
): string {
  const fm = screen.frontmatter;
  const lines: string[] = [
    `# ${fm.id}`,
    '',
    `**Name:** ${fm.name}`,
    `**Product:** ${fm.product}`,
    '',
    '## Implementations',
    '',
  ];
  for (const [platform, impl] of Object.entries(fm.implementations)) {
    if (!impl) continue;
    lines.push(
      `- **${platform}**: buildStatus: ${impl.buildStatus}, redesignStatus: ${impl.redesignStatus}, apiStatus: ${impl.apiStatus}` +
        (impl.route ? `, route: ${impl.route}` : '') +
        (impl.screen ? `, screen: ${impl.screen}` : '') +
        (impl.component ? `, component: ${impl.component}` : ''),
    );
  }
  lines.push('');
  if (fm.endpoints?.length) {
    lines.push('## Endpoints');
    lines.push('');
    for (const ep of fm.endpoints) lines.push(`- ${ep}`);
    lines.push('');
  }
  if (related.length > 0) {
    lines.push('## Related Screens');
    lines.push('');
    for (const r of related) {
      lines.push(`- (${r.rel}) ${r.id}${r.name ? ` — ${r.name}` : ''}`);
    }
    lines.push('');
  }
  if (fm.useCases?.length) {
    lines.push(`**Use Cases:** ${fm.useCases.join(', ')}`);
  }
  if (fm.epics?.length) {
    lines.push(`**Epics:** ${fm.epics.join(', ')}`);
  }
  lines.push('');
  // Recent agent log: pull the last 5 list items from the body's "## Agent Log" section.
  const agentLog = extractAgentLog(screen.body);
  if (agentLog.length > 0) {
    lines.push('## Recent Agent Log');
    lines.push('');
    for (const entry of agentLog.slice(0, 5)) lines.push(entry);
    lines.push('');
  }
  return lines.join('\n');
}

function extractAgentLog(body: string): string[] {
  const idx = body.indexOf('## Agent Log');
  if (idx < 0) return [];
  const after = body.slice(idx);
  return after
    .split(/\r?\n/)
    .filter((l) => l.startsWith('- '));
}
```

- [ ] **Step 4: Run edit-context tests to verify they pass**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test edit-context
```

Expected: PASS, 2 tests.

- [ ] **Step 5: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add frontend/packages/screen-map/src/edit-context.ts \
        frontend/packages/screen-map/tests/edit-context.test.ts
git commit -m "feat(screen-map): loadScreenContext markdown summary for screen-edit skill"
```

---

## Task 11: Visual Review server — session + server.ts skeleton

**Files:**
- Modify: `frontend/packages/screen-map/package.json` (add `hono` dep)
- Create: `frontend/packages/screen-map/src/review-server/session.ts`
- Create: `frontend/packages/screen-map/src/review-server/server.ts`

- [ ] **Step 1: Add `hono` dependency**

In `frontend/packages/screen-map/package.json`, add to `dependencies`:

```json
    "hono": "^4.6.0",
```

Run `pnpm install`:

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm install
```

- [ ] **Step 2: Implement `session.ts`**

`frontend/packages/screen-map/src/review-server/session.ts`:

```typescript
import { randomBytes } from 'node:crypto';
import type { ScreenMap } from '../types.js';
import type { DesignSource } from '../design-source/index.js';

export interface ReviewSession {
  /** 32-hex-char token; required as `?session=<token>` for all API calls. */
  readonly token: string;
  /** Filtered, ordered list of screens for this review walk. */
  screens: ScreenMap[];
  /** Index into `screens`; mutated as user navigates. */
  currentIdx: number;
  /** Resolved DesignSource per `name` (zip / claude-design / ...). */
  designSources: Map<string, DesignSource>;
  /** `--preview` flag passthrough: where the right pane points by default. */
  defaultPreview: 'local' | 'staging' | 'design';
}

export function createSession(args: {
  screens: ScreenMap[];
  designSources?: DesignSource[];
  defaultPreview?: ReviewSession['defaultPreview'];
}): ReviewSession {
  const sources = new Map<string, DesignSource>();
  for (const ds of args.designSources ?? []) sources.set(ds.name, ds);
  return {
    token: randomBytes(16).toString('hex'),
    screens: args.screens,
    currentIdx: 0,
    designSources: sources,
    defaultPreview: args.defaultPreview ?? 'local',
  };
}
```

- [ ] **Step 3: Implement `server.ts`**

`frontend/packages/screen-map/src/review-server/server.ts`:

```typescript
import { Hono, type Context, type Next } from 'hono';
import path from 'node:path';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import type { ReviewSession } from './session.js';

export interface ServerOptions {
  session: ReviewSession;
  /** Hook for graceful shutdown — typically calls server.close(). */
  onFinish: () => void;
}

export function buildServer(opts: ServerOptions): Hono {
  const app = new Hono();
  const here = path.dirname(fileURLToPath(import.meta.url));
  const clientDir = path.join(here, 'client');

  // Token gate — applies to every /api/* route.
  app.use('/api/*', async (c, next) => {
    const provided = c.req.query('session');
    if (provided !== opts.session.token) {
      return c.json({ error: 'invalid session token' }, 401);
    }
    await next();
  });

  // Static client (HTML, JS, CSS).
  app.get('/', async (c) => {
    const html = await readFile(path.join(clientDir, 'index.html'), 'utf8');
    return c.html(html.replace('__SESSION_TOKEN__', opts.session.token));
  });
  app.get('/styles.css', async (c) => {
    const css = await readFile(path.join(clientDir, 'styles.css'), 'utf8');
    return c.body(css, 200, { 'Content-Type': 'text/css' });
  });
  app.get('/app.tsx', async (c) => {
    const js = await readFile(path.join(clientDir, 'app.tsx'), 'utf8');
    return c.body(js, 200, { 'Content-Type': 'application/javascript' });
  });

  // API routes are wired in api.ts — Task 12 attaches them.
  return app;
}
```

- [ ] **Step 4: Verify typecheck (no test for this task — wired up in Task 13)**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map typecheck
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add frontend/packages/screen-map/src/review-server/session.ts \
        frontend/packages/screen-map/src/review-server/server.ts \
        frontend/packages/screen-map/package.json \
        frontend/pnpm-lock.yaml
git commit -m "feat(screen-map): review-server session + Hono shell with token gate"
```

---

## Task 12: Visual Review server — api.ts endpoint handlers

**Files:**
- Create: `frontend/packages/screen-map/src/review-server/api.ts`
- Modify: `frontend/packages/screen-map/src/review-server/server.ts` (mount `attachApi`)

- [ ] **Step 1: Implement `api.ts`**

`frontend/packages/screen-map/src/review-server/api.ts`:

```typescript
import path from 'node:path';
import type { Hono } from 'hono';
import { writeScreenMapString } from '../write.js';
import { writeFile } from 'node:fs/promises';
import type { ReviewSession } from './session.js';

interface ReviewBody {
  decisions?: Array<{ itemKey: string; ok: boolean; note?: string }>;
  generalNote?: string;
}

export function attachApi(
  app: Hono,
  opts: {
    session: ReviewSession;
    onFinish: () => void;
  },
): void {
  app.get('/api/session', (c) => {
    const summaries = opts.session.screens.map((s) => ({
      id: s.frontmatter.id,
      name: s.frontmatter.name,
      product: s.frontmatter.product,
    }));
    return c.json({
      product: opts.session.screens[0]?.frontmatter.product ?? null,
      screens: summaries,
      currentIdx: opts.session.currentIdx,
      sessionToken: opts.session.token,
      defaultPreview: opts.session.defaultPreview,
    });
  });

  app.get('/api/screens/:id', (c) => {
    const id = c.req.param('id');
    const screen = opts.session.screens.find((s) => s.frontmatter.id === id);
    if (!screen) return c.json({ error: 'not found' }, 404);
    return c.json({
      frontmatter: screen.frontmatter,
      body: screen.body,
      previewUrls: buildPreviewUrls(screen, opts.session),
    });
  });

  app.post('/api/screens/:id/review', async (c) => {
    const id = c.req.param('id');
    const idx = opts.session.screens.findIndex((s) => s.frontmatter.id === id);
    if (idx < 0) return c.json({ error: 'not found' }, 404);
    const screen = opts.session.screens[idx];
    const body = (await c.req.json()) as ReviewBody;

    // Mutate body: append Agent Log entry; optionally append general note.
    const today = new Date().toISOString().slice(0, 10);
    const numOk = (body.decisions ?? []).filter((d) => d.ok).length;
    const numNotes = (body.decisions ?? []).filter((d) => d.note).length;
    const summary = `${today} — review: ${numOk} OK, ${numNotes} note${numNotes === 1 ? '' : 's'}`;
    const newBody = appendAgentLog(screen.body, `- ${summary}`);
    const finalBody = body.generalNote
      ? appendSpecificNote(newBody, today, body.generalNote)
      : newBody;
    // Update lastReview only.
    screen.frontmatter.lastReview = today;
    screen.body = finalBody;
    const serialized = writeScreenMapString(screen);
    await writeFile(screen.filePath, serialized, 'utf8');

    const next = opts.session.screens[idx + 1];
    opts.session.currentIdx = idx + 1;
    return c.json(next ? { nextScreenId: next.frontmatter.id } : { done: true });
  });

  app.post('/api/session/finish', (c) => {
    setTimeout(opts.onFinish, 100); // let response flush
    return c.json({ ok: true });
  });

  app.get('/api/designs/:adapter/:frameId', async (c) => {
    const adapterName = c.req.param('adapter');
    const frameId = c.req.param('frameId');
    const adapter = opts.session.designSources.get(adapterName);
    if (!adapter || !adapter.readBytes) {
      return c.json({ error: 'unknown adapter or no readBytes support' }, 404);
    }
    const bytes = await adapter.readBytes(frameId);
    if (!bytes) return c.json({ error: 'frame not found' }, 404);
    return c.body(bytes, 200, { 'Content-Type': 'image/png' });
  });
}

function buildPreviewUrls(
  screen: { frontmatter: { product: string; implementations: Record<string, unknown> } },
  session: ReviewSession,
): { local: string | null; staging: string | null } {
  const impl = (screen.frontmatter.implementations as Record<string, { route?: string }>);
  const route = impl['ppt-web']?.route ?? impl['reality-web']?.route ?? null;
  if (!route) return { local: null, staging: null };
  const local = screen.frontmatter.product === 'ppt' ? `http://localhost:5173${route}` : `http://localhost:3000${route}`;
  const stagingHost = screen.frontmatter.product === 'ppt' ? 'ppt.rlt.sk' : 'www.rlt.sk';
  const staging = `https://${stagingHost}${route}`;
  return { local, staging };
}

function appendAgentLog(body: string, line: string): string {
  const idx = body.indexOf('## Agent Log');
  if (idx < 0) return body + `\n## Agent Log\n\n${line}\n`;
  // Insert under the heading + comment, before any other entries.
  const before = body.slice(0, idx);
  const after = body.slice(idx);
  const lines = after.split(/\r?\n/);
  const insertIdx = lines.findIndex((l, i) => i > 0 && (l.startsWith('- ') || (l === '' && i > 2)));
  const insertAt = insertIdx > 0 ? insertIdx : lines.length;
  lines.splice(insertAt, 0, line);
  return before + lines.join('\n');
}

function appendSpecificNote(body: string, date: string, note: string): string {
  const heading = '### Specific (recent)';
  const idx = body.indexOf(heading);
  if (idx < 0) return body;
  const before = body.slice(0, idx + heading.length);
  const after = body.slice(idx + heading.length);
  return before + `\n\n- ${date}: ${note}` + after;
}
```

- [ ] **Step 2: Mount `attachApi` in `server.ts`**

In `frontend/packages/screen-map/src/review-server/server.ts`, find the line `// API routes are wired in api.ts — Task 12 attaches them.` and replace with:

```typescript
  // API routes wired here.
  // (imported lazily to keep the surface inspectable.)
  // eslint-disable-next-line
  const { attachApi } = await import('./api.js');
  attachApi(app, opts);
```

The function signature must change to async:

```typescript
export async function buildServer(opts: ServerOptions): Promise<Hono> {
```

- [ ] **Step 3: Verify typecheck**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map typecheck
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add frontend/packages/screen-map/src/review-server/api.ts \
        frontend/packages/screen-map/src/review-server/server.ts
git commit -m "feat(screen-map): review-server API endpoints (session, screens, finish, designs)"
```

---

## Task 13: Visual Review server — integration test

**Files:**
- Create: `frontend/packages/screen-map/tests/review-server/server.test.ts`

- [ ] **Step 1: Write the integration test**

`frontend/packages/screen-map/tests/review-server/server.test.ts`:

```typescript
import { describe, expect, it, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, readFile } from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import { bulkWriteScreenMaps } from '../../src/init-write.js';
import { discoverScreenMaps } from '../../src/discover.js';
import { parseScreenMap } from '../../src/parse.js';
import { createSession } from '../../src/review-server/session.js';
import { buildServer } from '../../src/review-server/server.js';

let tmpRoot: string;
beforeEach(async () => {
  tmpRoot = await mkdtemp(path.join(os.tmpdir(), 'review-srv-'));
  await mkdir(path.join(tmpRoot, 'docs/screens'), { recursive: true });
  await bulkWriteScreenMaps(
    [
      { id: 'ppt/foo', name: 'Foo', product: 'ppt', source: 'sitemap' },
      { id: 'ppt/bar', name: 'Bar', product: 'ppt', source: 'sitemap' },
    ],
    path.join(tmpRoot, 'docs/screens'),
  );
});
afterEach(async () => {
  await rm(tmpRoot, { recursive: true, force: true });
});

describe('review-server', () => {
  it('serves session metadata and walks screens, persisting reviews to markdown', async () => {
    const files = await discoverScreenMaps(path.join(tmpRoot, 'docs/screens'));
    const screens = await Promise.all(files.map((f) => parseScreenMap(f)));
    const session = createSession({ screens, defaultPreview: 'local' });
    const app = await buildServer({ session, onFinish: () => {} });

    const sessRes = await app.request(`/api/session?session=${session.token}`);
    expect(sessRes.status).toBe(200);
    const sessJson = (await sessRes.json()) as { screens: { id: string }[] };
    expect(sessJson.screens.map((s) => s.id).sort()).toEqual(['ppt/bar', 'ppt/foo']);

    const screenRes = await app.request(
      `/api/screens/ppt/foo?session=${session.token}`,
    );
    expect(screenRes.status).toBe(200);

    const reviewRes = await app.request(
      `/api/screens/ppt/foo/review?session=${session.token}`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          decisions: [
            { itemKey: 'view-info', ok: true },
            { itemKey: 'edit-info', ok: false, note: 'missing on mobile' },
          ],
          generalNote: 'header looks good',
        }),
      },
    );
    expect(reviewRes.status).toBe(200);
    const reviewJson = (await reviewRes.json()) as { nextScreenId?: string; done?: boolean };
    expect(reviewJson.nextScreenId).toBe('ppt/bar');

    // The markdown file should now contain an Agent Log entry + a Specific note.
    const updated = await readFile(path.join(tmpRoot, 'docs/screens/ppt/foo.md'), 'utf8');
    expect(updated).toMatch(/review: 1 OK, 1 note/);
    expect(updated).toMatch(/header looks good/);
  });

  it('rejects api requests without the session token', async () => {
    const session = createSession({ screens: [] });
    const app = await buildServer({ session, onFinish: () => {} });
    const res = await app.request('/api/session');
    expect(res.status).toBe(401);
  });
});
```

- [ ] **Step 2: Run test to verify it passes**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test review-server
```

Expected: PASS, 2 tests.

- [ ] **Step 3: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add frontend/packages/screen-map/tests/review-server/
git commit -m "test(screen-map): review-server end-to-end (session, walk, persist)"
```

---

## Task 14: Visual Review server — startup script (port pick + browser open + signal handling)

**Files:**
- Create: `frontend/packages/screen-map/src/review-server/start.ts`

- [ ] **Step 1: Implement `start.ts`**

`frontend/packages/screen-map/src/review-server/start.ts`:

```typescript
import { serve } from '@hono/node-server';
import { exec } from 'node:child_process';
import { discoverScreenMaps } from '../discover.js';
import { parseScreenMap } from '../parse.js';
import { createSession } from './session.js';
import { buildServer } from './server.js';
import type { Product } from '../types.js';

export interface StartOptions {
  repoRoot: string;
  product?: Product;
  filter?: (frontmatter: { id: string; product: string; implementations: Record<string, unknown> }) => boolean;
  preview?: 'local' | 'staging' | 'design';
  /** Starting port to try. Increments until a free port is found. */
  startPort?: number;
}

export interface StartResult {
  port: number;
  url: string;
  shutdown: () => Promise<void>;
}

export async function startReviewServer(opts: StartOptions): Promise<StartResult> {
  const screensDir = `${opts.repoRoot}/docs/screens`;
  const files = await discoverScreenMaps(screensDir);
  let screens = await Promise.all(files.map((f) => parseScreenMap(f)));
  if (opts.product) {
    screens = screens.filter((s) => s.frontmatter.product === opts.product);
  }
  if (opts.filter) {
    screens = screens.filter((s) => opts.filter!(s.frontmatter as never));
  }
  const session = createSession({ screens, defaultPreview: opts.preview });

  let serverHandle: { close: (cb?: () => void) => void } | null = null;
  const onFinish = () => serverHandle?.close();
  const app = await buildServer({ session, onFinish });

  const port = await findFreePort(opts.startPort ?? 5179);
  serverHandle = serve({ fetch: app.fetch, port });

  const url = `http://127.0.0.1:${port}/?session=${session.token}`;
  openBrowser(url);

  const shutdown = (): Promise<void> =>
    new Promise((resolve) => {
      if (!serverHandle) return resolve();
      serverHandle.close(() => resolve());
    });

  process.once('SIGINT', () => {
    shutdown().then(() => process.exit(0));
  });

  return { port, url, shutdown };
}

async function findFreePort(start: number): Promise<number> {
  for (let p = start; p < start + 100; p++) {
    const ok = await new Promise<boolean>((resolve) => {
      import('node:net').then(({ createServer }) => {
        const tester = createServer();
        tester.once('error', () => resolve(false));
        tester.once('listening', () => tester.close(() => resolve(true)));
        tester.listen(p, '127.0.0.1');
      });
    });
    if (ok) return p;
  }
  throw new Error(`no free port between ${start} and ${start + 100}`);
}

function openBrowser(url: string): void {
  const platform = process.platform;
  const cmd =
    platform === 'darwin'
      ? `open "${url}"`
      : platform === 'win32'
        ? `start "" "${url}"`
        : `xdg-open "${url}"`;
  exec(cmd, () => {});
}
```

- [ ] **Step 2: Add `@hono/node-server` as a dependency**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend
pnpm --filter @ppt/screen-map add @hono/node-server@^1.13.0
```

- [ ] **Step 3: Verify typecheck**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map typecheck
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add frontend/packages/screen-map/src/review-server/start.ts \
        frontend/packages/screen-map/package.json \
        frontend/pnpm-lock.yaml
git commit -m "feat(screen-map): review-server startup (port pick + browser open + SIGINT)"
```

---

## Task 15: Visual Review SPA — HTML shell + styles + entry

**Files:**
- Create: `frontend/packages/screen-map/src/review-server/client/index.html`
- Create: `frontend/packages/screen-map/src/review-server/client/styles.css`
- Create: `frontend/packages/screen-map/src/review-server/client/app.tsx`

- [ ] **Step 1: Write the HTML shell**

`frontend/packages/screen-map/src/review-server/client/index.html`:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Screen-Map Review</title>
    <link rel="stylesheet" href="/styles.css" />
  </head>
  <body>
    <div id="root"></div>
    <script type="module">
      window.__SESSION_TOKEN__ = '__SESSION_TOKEN__';
    </script>
    <script type="module" src="/app.tsx"></script>
  </body>
</html>
```

- [ ] **Step 2: Write the CSS**

`frontend/packages/screen-map/src/review-server/client/styles.css`:

```css
* { box-sizing: border-box; }
body { margin: 0; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; background: #f6f7f9; }
#root { height: 100vh; display: flex; flex-direction: column; }
.topbar { display: flex; gap: 1rem; padding: 0.75rem 1rem; background: #1e293b; color: #fff; align-items: center; }
.topbar button { background: #334155; color: #fff; border: 0; padding: 0.4rem 0.75rem; border-radius: 4px; cursor: pointer; }
.topbar button:hover { background: #475569; }
.topbar .progress { font-weight: 500; }
.layout { flex: 1; display: grid; grid-template-columns: 1fr 1fr; gap: 0; min-height: 0; }
.left, .right { padding: 1rem; overflow: auto; min-width: 0; }
.left { background: #fff; border-right: 1px solid #e2e8f0; }
.checklist-row { display: flex; gap: 0.5rem; align-items: flex-start; padding: 0.5rem; border-bottom: 1px solid #f1f5f9; }
.checklist-row.ok { background: #ecfdf5; }
.checklist-row.note { background: #fef9c3; }
.checklist-row textarea { flex: 1; padding: 0.4rem; border: 1px solid #e2e8f0; border-radius: 4px; resize: vertical; min-height: 2.5rem; font-family: inherit; }
.preview-toggle { display: flex; gap: 0.5rem; padding: 0.5rem; }
.preview-toggle button { padding: 0.3rem 0.6rem; border: 1px solid #cbd5e1; background: #fff; border-radius: 4px; cursor: pointer; }
.preview-toggle button.active { background: #1e293b; color: #fff; border-color: #1e293b; }
.preview-pane iframe, .preview-pane img { width: 100%; height: 100%; border: 0; min-height: 0; }
.save-btn { width: 100%; padding: 0.75rem; background: #2563eb; color: #fff; border: 0; border-radius: 4px; font-weight: 500; cursor: pointer; margin-top: 1rem; }
.save-btn:hover { background: #1d4ed8; }
.metadata { font-size: 0.85rem; color: #475569; margin-bottom: 1rem; }
.general-note { width: 100%; min-height: 4rem; padding: 0.5rem; border: 1px solid #cbd5e1; border-radius: 4px; font-family: inherit; margin-top: 1rem; }
```

- [ ] **Step 3: Write the SPA entry**

`frontend/packages/screen-map/src/review-server/client/app.tsx`:

```typescript
// @ts-nocheck — this file runs in the browser via esm.sh; not type-checked by tsc.
import { h, render } from 'https://esm.sh/preact@10.24.3';
import { useState, useEffect } from 'https://esm.sh/preact@10.24.3/hooks';
import htm from 'https://esm.sh/htm@3.1.1';

const html = htm.bind(h);
const TOKEN = window.__SESSION_TOKEN__;

async function api(path, init) {
  const sep = path.includes('?') ? '&' : '?';
  const res = await fetch(`${path}${sep}session=${TOKEN}`, init);
  return res.json();
}

function App() {
  const [session, setSession] = useState(null);
  const [currentId, setCurrentId] = useState(null);
  useEffect(() => {
    api('/api/session').then((s) => {
      setSession(s);
      setCurrentId(s.screens[s.currentIdx]?.id ?? null);
    });
  }, []);
  if (!session) return html`<div class="topbar"><span>loading…</span></div>`;
  if (!currentId) return html`<div class="topbar"><span>review complete</span></div>`;
  return html`<${ScreenView}
    sessionToken=${TOKEN}
    screenId=${currentId}
    total=${session.screens.length}
    onNext=${(nextId) => setCurrentId(nextId)}
  />`;
}

function ScreenView({ sessionToken, screenId, total, onNext }) {
  const [screen, setScreen] = useState(null);
  const [decisions, setDecisions] = useState({});
  const [generalNote, setGeneralNote] = useState('');
  const [previewMode, setPreviewMode] = useState('local');
  useEffect(() => {
    setDecisions({});
    setGeneralNote('');
    api(`/api/screens/${encodeURIComponent(screenId)}`).then(setScreen);
  }, [screenId]);
  if (!screen) return html`<div class="topbar"><span>loading screen…</span></div>`;
  const featureItems = parseChecklist(screen.body);
  async function saveAndNext() {
    const decArr = featureItems.map((f) => ({
      itemKey: f.key,
      ok: !!decisions[f.key]?.ok,
      note: decisions[f.key]?.note,
    }));
    const r = await api(`/api/screens/${encodeURIComponent(screenId)}/review`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ decisions: decArr, generalNote }),
    });
    if (r.done) {
      await api('/api/session/finish', { method: 'POST' });
      window.close();
    } else {
      onNext(r.nextScreenId);
    }
  }
  const previewSrc =
    previewMode === 'local' ? screen.previewUrls.local
    : previewMode === 'staging' ? screen.previewUrls.staging
    : null;
  return html`
    <div class="topbar">
      <span class="progress">${screen.frontmatter.id} (${screen.frontmatter.product})</span>
      <span>${total} screens total</span>
    </div>
    <div class="layout">
      <div class="left">
        <div class="metadata">${formatStatus(screen.frontmatter)}</div>
        <h3>Functionality</h3>
        ${featureItems.map((f) => html`<${ChecklistRow} key=${f.key} item=${f} state=${decisions[f.key]} onChange=${(s) => setDecisions((prev) => ({ ...prev, [f.key]: s }))} />`)}
        <h3>General note for this screen</h3>
        <textarea class="general-note" value=${generalNote} onInput=${(e) => setGeneralNote(e.currentTarget.value)} />
        <button class="save-btn" onClick=${saveAndNext}>Save & Next</button>
      </div>
      <div class="right">
        <div class="preview-toggle">
          <button class=${previewMode === 'local' ? 'active' : ''} onClick=${() => setPreviewMode('local')}>Local</button>
          <button class=${previewMode === 'staging' ? 'active' : ''} onClick=${() => setPreviewMode('staging')}>Staging</button>
        </div>
        <div class="preview-pane">
          ${previewSrc ? html`<iframe src=${previewSrc}></iframe>` : html`<p>(no preview URL for this screen)</p>`}
        </div>
      </div>
    </div>
  `;
}

function ChecklistRow({ item, state, onChange }) {
  const ok = state?.ok ?? false;
  const note = state?.note ?? '';
  const cls = ok ? 'checklist-row ok' : note ? 'checklist-row note' : 'checklist-row';
  return html`<div class=${cls}>
    <input type="checkbox" checked=${ok} onChange=${(e) => onChange({ ok: e.currentTarget.checked, note })} />
    <div style=${{ flex: 1 }}>
      <div>${item.label}</div>
      <textarea placeholder="optional note" value=${note} onInput=${(e) => onChange({ ok, note: e.currentTarget.value })} />
    </div>
  </div>`;
}

function parseChecklist(body) {
  const idx = body.indexOf('## Functionality Checklist');
  if (idx < 0) return [];
  const after = body.slice(idx).split(/\r?\n/);
  const items = [];
  for (const line of after) {
    const match = line.match(/^- \[([ x])\] (.+)$/);
    if (match) {
      const label = match[2];
      const key = label.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
      items.push({ key, label });
    }
    if (line.startsWith('## ') && !line.startsWith('## Functionality')) break;
  }
  return items;
}

function formatStatus(fm) {
  const parts = [];
  for (const [p, impl] of Object.entries(fm.implementations)) {
    parts.push(`${p}: ${impl.buildStatus} / ${impl.redesignStatus}`);
  }
  return parts.join(' • ');
}

render(html`<${App} />`, document.getElementById('root'));
```

- [ ] **Step 4: Commit (no test — UI is exercised via the integration test indirectly)**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add frontend/packages/screen-map/src/review-server/client/
git commit -m "feat(screen-map): review-server SPA (Preact via esm.sh, no build step)"
```

---

## Task 16: cli.ts — wire `init`, `edit`, `review` subcommands

**Files:**
- Modify: `frontend/packages/screen-map/src/cli.ts`

- [ ] **Step 1: Add the three subcommands**

In `frontend/packages/screen-map/src/cli.ts`, after the existing `validate` subcommand block, append three new blocks. The full updated file:

```typescript
#!/usr/bin/env -S npx tsx
import path from 'node:path';
import { Command } from 'commander';
import { buildValidationContext } from './context.js';
import { discoverScreenMaps } from './discover.js';
import { parseScreenMap, ScreenMapParseError } from './parse.js';
import { validateScreenMap } from './validate.js';
import { scanCandidates } from './scan.js';
import { mergeCandidates, type GroupingDecision } from './grouping.js';
import { bulkWriteScreenMaps } from './init-write.js';
import { loadScreenContext } from './edit-context.js';
import { startReviewServer } from './review-server/start.js';
import { createDesignSource } from './design-source/index.js';

const program = new Command();
program
  .name('screen-map')
  .description('CLI for the @ppt/screen-map system')
  .version('0.1.0');

program
  .command('validate')
  .description('validate every screen-map under <root>/docs/screens')
  .option('--root <path>', 'repo root', process.cwd())
  .option('--strict', 'exit non-zero on any error', false)
  .action(async (opts: { root: string; strict: boolean }) => {
    const repoRoot = path.resolve(opts.root);
    const screensDir = path.join(repoRoot, 'docs/screens');
    const files = await discoverScreenMaps(screensDir);
    const ctx = await buildValidationContext({ repoRoot });

    let totalErrors = 0;
    let totalWarnings = 0;
    for (const file of files) {
      try {
        const screen = await parseScreenMap(file);
        const issues = validateScreenMap(screen, ctx);
        if (issues.length === 0) {
          process.stdout.write(`  ok  ${path.relative(repoRoot, file)}\n`);
          continue;
        }
        for (const issue of issues) {
          const tag = issue.severity === 'error' ? 'error' : 'warn ';
          process.stdout.write(
            `  ${tag} ${path.relative(repoRoot, file)} :: ${issue.path} :: ${issue.message}\n`,
          );
          if (issue.severity === 'error') totalErrors += 1;
          else totalWarnings += 1;
        }
      } catch (err) {
        if (err instanceof ScreenMapParseError) {
          for (const issue of err.issues) {
            process.stderr.write(
              `  parse ${path.relative(repoRoot, file)} :: ${issue}\n`,
            );
          }
          totalErrors += 1;
        } else {
          throw err;
        }
      }
    }
    process.stdout.write(
      `Validated ${files.length} screen-maps: ${totalErrors} errors, ${totalWarnings} warnings.\n`,
    );
    if (opts.strict && totalErrors > 0) process.exit(1);
  });

program
  .command('init')
  .description('scan + interactive grouping + bulk-write screen-maps for a product')
  .requiredOption('--product <name>', 'ppt | reality')
  .option('--root <path>', 'repo root', process.cwd())
  .option('--designs <zipPath>', 'DesignSource ZIP file')
  .option('--add <names...>', 'user-added candidate names')
  .option('--decisions <jsonPath>', 'JSON file with grouping decisions (skip interactive prompt)')
  .option('--force', 'overwrite existing screen-maps', false)
  .action(async (opts: {
    product: 'ppt' | 'reality';
    root: string;
    designs?: string;
    add?: string[];
    decisions?: string;
    force: boolean;
  }) => {
    const repoRoot = path.resolve(opts.root);
    const designSource = opts.designs
      ? await createDesignSource({ adapter: 'zip', file: opts.designs }, { repoRoot })
      : undefined;
    const candidates = await scanCandidates({
      product: opts.product,
      repoRoot,
      sources: {
        sitemap: true,
        useCases: true,
        epics: true,
        designSource,
        userAdd: opts.add ?? [],
      },
    });
    let decisions: GroupingDecision[] = [];
    if (opts.decisions) {
      const fs = await import('node:fs/promises');
      decisions = JSON.parse(await fs.readFile(opts.decisions, 'utf8'));
    }
    const concepts = mergeCandidates(candidates, decisions);
    const screensDir = path.join(repoRoot, 'docs/screens');
    const written = await bulkWriteScreenMaps(concepts, screensDir, { force: opts.force });
    process.stdout.write(`Wrote ${written.length} screen-maps under ${screensDir}\n`);
    for (const file of written) process.stdout.write(`  + ${path.relative(repoRoot, file)}\n`);
  });

program
  .command('edit <id>')
  .description('print a markdown context summary for one screen')
  .option('--root <path>', 'repo root', process.cwd())
  .option('--playwright', 'capture a screenshot via Playwright', false)
  .action(async (id: string, opts: { root: string; playwright: boolean }) => {
    const repoRoot = path.resolve(opts.root);
    const summary = await loadScreenContext(id, {
      repoRoot,
      includePlaywright: opts.playwright,
    });
    process.stdout.write(summary + '\n');
  });

program
  .command('review')
  .description('spawn the Visual Review server and open the browser')
  .option('--root <path>', 'repo root', process.cwd())
  .option('--product <name>', 'ppt | reality')
  .option('--preview <mode>', 'local | staging | design', 'local')
  .action(async (opts: { root: string; product?: 'ppt' | 'reality'; preview: 'local' | 'staging' | 'design' }) => {
    const repoRoot = path.resolve(opts.root);
    const result = await startReviewServer({
      repoRoot,
      product: opts.product,
      preview: opts.preview,
    });
    process.stdout.write(`Review server running at ${result.url}\n`);
    process.stdout.write('Press Ctrl-C to stop.\n');
    // Keep the process alive — SIGINT handler in start.ts handles shutdown.
    await new Promise(() => {});
  });

program.parseAsync().catch((err) => {
  process.stderr.write(`Unexpected error: ${(err as Error).message}\n`);
  process.exit(2);
});
```

- [ ] **Step 2: Verify typecheck**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map typecheck
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add frontend/packages/screen-map/src/cli.ts
git commit -m "feat(screen-map): cli adds init, edit, review subcommands"
```

---

## Task 17: index.ts — re-export new public API

**Files:**
- Modify: `frontend/packages/screen-map/src/index.ts`

- [ ] **Step 1: Update `index.ts`**

Replace `frontend/packages/screen-map/src/index.ts` with:

```typescript
export * from './types.js';
export * from './schema.js';
export { parseScreenMap, parseScreenMapString, ScreenMapParseError } from './parse.js';
export { writeScreenMap, writeScreenMapString } from './write.js';
export { validateScreenMap, type ValidationIssue, type ValidationContext } from './validate.js';
export { discoverScreenMaps } from './discover.js';
export { buildValidationContext, type BuildContextOptions } from './context.js';
export { scanCandidates, type CandidateScreen, type ScanOptions } from './scan.js';
export { mergeCandidates, type GroupingDecision } from './grouping.js';
export { bulkWriteScreenMaps, type BulkWriteOptions } from './init-write.js';
export { loadScreenContext, type LoadScreenContextOptions } from './edit-context.js';
export {
  type DesignFrame,
  type DesignSource,
  type DesignSourceConfig,
  createDesignSource,
} from './design-source/index.js';
export { ZipAdapter } from './design-source/zip-adapter.js';
export { ClaudeDesignAdapter } from './design-source/claude-design.js';
```

- [ ] **Step 2: Verify typecheck**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map typecheck
```

Expected: PASS.

- [ ] **Step 3: Run full test suite**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test
```

Expected: PASS for all suites — Phase 1 + new tests from this phase.

- [ ] **Step 4: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add frontend/packages/screen-map/src/index.ts
git commit -m "feat(screen-map): re-export Phase 2 public API from index.ts"
```

---

## Task 18: Skill manifest — `screen-map-init`

**Files:**
- Create: `.claude/skills/screen-map-init/SKILL.md`

- [ ] **Step 1: Write SKILL.md**

`.claude/skills/screen-map-init/SKILL.md`:

````markdown
---
name: screen-map-init
description: Bootstrap or extend docs/screens/ for one product by scanning sitemap, use-cases, epics, an optional design ZIP, and any user-provided candidate names; then walks the user through interactive grouping in chat before bulk-writing the markdown files. Use when the user asks to "init screens", "bootstrap screen-map", "create screen maps for ppt|reality", or runs `/screens init`.
---

# Screen-Map Init Skill

Bulk-scans candidate screens for a product, presents a grouping plan in chat, and writes finalized screen-maps once the user approves.

## When to use

- User invokes `/screens init` (with `--product=ppt` or `--product=reality`).
- User asks to "init screens for <product>" or "bootstrap the screen-map".
- User provides a designs ZIP and wants those frames mapped to screens.

## Inputs (forwarded to the CLI)

- `--product=ppt|reality` (required).
- `--root <path>` (defaults to repo root).
- `--designs=<path-to-zip>` (optional).
- `--add="Name 1" --add="Name 2"` (optional user-added candidates).
- `--decisions=<jsonPath>` (optional; supplies grouping decisions non-interactively).
- `--force` (optional; overwrite existing files).

## Workflow

1. Resolve repo root via `git rev-parse --show-toplevel`.
2. Run an initial scan (no `--decisions`) to enumerate candidates from sitemap + use-cases + epics + designs + user-added, then **present** the candidate list to the user in chat as a markdown table:
   ```
   | Source | Id | Name | Sitemap | Frame | UC/Epic |
   |---|---|---|---|---|---|
   | sitemap | ppt/buildings-list | Buildings List | ppt-buildings-list | – | – |
   | design | ppt/redesign-foo | Redesign Foo | – | foo-v3 | – |
   | use-cases | ppt/uc-12 | UC-12 | – | – | UC-12 |
   ```
3. Propose groupings: identify candidates that look like duplicates ("ppt/buildings-list" + "ppt/uc-15" both relate to the same building screen) and suggest merges. Suggest skips for noise (one-off UC ids that don't match a real screen).
4. Wait for the user's reply. Accept free-form chat replies like:
   - "merge ppt/uc-12 and ppt/uc-15 into ppt/building-management"
   - "skip ppt/uc-99"
   - "rename ppt/foo to ppt/foo-detail"
   - "OK, go" (apply the proposed plan as-is)
5. Translate the user's reply into a `GroupingDecision[]` JSON, write to a temp file under `/tmp`.
6. Re-invoke the CLI with `--decisions=<tmpfile>` to apply the decisions and write the markdown files.
7. Run `/screens validate` to confirm the output is clean.

## Implementation

```bash
REPO_ROOT="$(git rev-parse --show-toplevel)"

# Step 1: initial scan (no decisions). Capture candidates as JSON for the chat presentation.
pnpm -C "$REPO_ROOT/frontend" --filter @ppt/screen-map cli init \
  --product "$PRODUCT" \
  --root "$REPO_ROOT" \
  ${DESIGNS:+--designs "$DESIGNS"} \
  ${ADD_FLAGS}
```

(The agent constructs `--add` flags for each user-supplied candidate and the optional `--designs` flag from the user's input.)

## Output handling

- All-clean → reply with the table and proposed grouping; ask user to confirm or correct.
- Validate after writing → if validation fails, surface errors; do NOT auto-revert (user can edit and re-run).
- File conflicts → if `bulkWriteScreenMaps` rejects with "already exists", ask the user whether to use `--force` or skip those screens.
````

- [ ] **Step 2: Verify the skill is discoverable**

```bash
ls /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/.claude/skills/screen-map-init/SKILL.md
```

Expected: file exists.

- [ ] **Step 3: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add .claude/skills/screen-map-init/SKILL.md
git commit -m "feat(skill): screen-map-init manifest (chat-driven grouping)"
```

---

## Task 19: Skill manifest — `screen-edit`

**Files:**
- Create: `.claude/skills/screen-edit/SKILL.md`

- [ ] **Step 1: Write SKILL.md**

`.claude/skills/screen-edit/SKILL.md`:

````markdown
---
name: screen-edit
description: Load focused context for a single screen-map (parent + children + related + sitemap entries + recent agent log + optional Playwright screenshot). Use when the user runs `/screens edit <id>` or asks to "load context for screen X", "show me ppt/foo", "what's the current state of <screen>".
---

# Screen-Edit Skill

Pulls everything an agent needs to work on a single screen-map into a markdown summary printed in chat. Avoids hunting for context across files.

## When to use

- User invokes `/screens edit <id>` (e.g. `/screens edit ppt/building-detail`).
- User asks "load <screen>", "show me ppt/foo", "what's the state of reality/property-detail".
- Before making any change to a screen-map, to load context.

## Inputs

- `<id>` (required) — the screen-map id (`<product>/<slug>`).
- `--root <path>` (optional; defaults to repo root).
- `--playwright` (optional; capture a screenshot if the local app is running).

## Implementation

```bash
REPO_ROOT="$(git rev-parse --show-toplevel)"
pnpm -C "$REPO_ROOT/frontend" --filter @ppt/screen-map cli edit "$ID" \
  --root "$REPO_ROOT" \
  ${PLAYWRIGHT:+--playwright}
```

## Output handling

- Print the CLI's markdown summary verbatim into chat.
- Then offer next-step actions:
  - "Edit this file?" → opens the screen-map markdown for editing.
  - "Run `/screens validate`?" → verifies cross-references.
  - "Run Playwright?" → if `--playwright` was not supplied initially.
````

- [ ] **Step 2: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add .claude/skills/screen-edit/SKILL.md
git commit -m "feat(skill): screen-edit manifest (single-screen context loader)"
```

---

## Task 20: Skill manifest — `screen-map-review`

**Files:**
- Create: `.claude/skills/screen-map-review/SKILL.md`

- [ ] **Step 1: Write SKILL.md**

`.claude/skills/screen-map-review/SKILL.md`:

````markdown
---
name: screen-map-review
description: Spawn the local Visual Review server (Hono + Preact SPA) on 127.0.0.1, open the browser, and walk every screen-map for a product with OK / Note checkboxes and a per-screen Save & Next button. Notes persist directly into Agent Log + Notes > Specific. Use when the user runs `/screens review` or asks to "review screens", "walk the screen-map", "do a redesign review".
---

# Screen-Map Review Skill

Drives the per-screen Visual Review UI. Saves user feedback directly into the markdown files (Agent Log + Notes), never mutates statuses.

## When to use

- User invokes `/screens review` (with optional `--product` / `--filter` / `--preview`).
- User asks to "walk through all screens", "review the new redesign", "go through screens one by one".

## Inputs

- `--product=ppt|reality` (optional).
- `--filter=<frontmatter-query>` (optional, e.g. `redesignStatus:in-progress`). Applied client-side after server fetches all matching screens.
- `--preview=local|staging|design` (optional, defaults to `local` if `pnpm dev` is running, else `staging`).
- `--root <path>` (optional).

## Implementation

```bash
REPO_ROOT="$(git rev-parse --show-toplevel)"
pnpm -C "$REPO_ROOT/frontend" --filter @ppt/screen-map cli review \
  --root "$REPO_ROOT" \
  ${PRODUCT:+--product "$PRODUCT"} \
  ${PREVIEW:+--preview "$PREVIEW"}
```

The CLI runs in the foreground, opens the browser at `http://127.0.0.1:5179/?session=<token>`, and waits. The user closes the tab or the SPA hits "Save & Next" past the last screen → server `POST /api/session/finish` → process exits.

## Output handling

- On startup: print "Review server at http://127.0.0.1:<port>?session=<short-token-prefix>". Don't echo the full token (it's already in the browser URL).
- On graceful exit: print a tally — how many screens reviewed, how many had notes.
- On Ctrl-C: server shuts down; print "Review session interrupted".
````

- [ ] **Step 2: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add .claude/skills/screen-map-review/SKILL.md
git commit -m "feat(skill): screen-map-review manifest (visual review server driver)"
```

---

## Task 21: `/screens` slash command — extend dispatcher

**Files:**
- Modify: `.claude/commands/screens.md`

- [ ] **Step 1: Replace `screens.md` content**

`.claude/commands/screens.md`:

````markdown
# /screens — Screen-Map dispatcher

Dispatch into a screen-map subcommand. Phase 2 supports `validate`, `init`, `edit`, `review`. Phase 3 will add `update`, `render`, `query`.

## Usage

```bash
/screens validate                            # Phase 1
/screens validate --strict

/screens init --product=ppt                  # Phase 2 NEW
/screens init --product=reality --designs=designs/2026-q2.zip
/screens init --product=ppt --add="Custom screen 1" --add="Custom screen 2"

/screens edit ppt/building-detail            # Phase 2 NEW
/screens edit reality/property-detail --playwright

/screens review                              # Phase 2 NEW
/screens review --product=ppt --preview=staging
```

## Implementation

Parse `$ARGUMENTS` for the first token (subcommand) and the rest (forwarded flags).

- `validate` → invoke the `screen-map-validate` skill.
- `init` → invoke the `screen-map-init` skill (chat-driven grouping).
- `edit <id>` → invoke the `screen-edit` skill.
- `review` → invoke the `screen-map-review` skill.
- `update | render | query` → respond:
  "This subcommand is part of Phase 3 of the screen-map plan and is not yet wired up. See `docs/superpowers/specs/2026-05-07-screen-map-system-design.md` Section 5."
- Missing/unknown subcommand → print this usage block.
````

- [ ] **Step 2: Commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git add .claude/commands/screens.md
git commit -m "feat(slash): /screens dispatches init, edit, review (phase-2)"
```

---

## Task 22: Phase 2 ship checkpoint

**Files:** none modified — verification only.

- [ ] **Step 1: Run the full screen-map test suite**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend && pnpm --filter @ppt/screen-map test
```

Expected: ALL pass — Phase 1 21 tests + Phase 2 additions ≈ 35-40 total across 11 test files.

- [ ] **Step 2: Run typecheck and biome**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e/frontend
pnpm --filter @ppt/screen-map typecheck
pnpm biome check packages/screen-map
```

Both must pass cleanly.

- [ ] **Step 3: End-to-end smoke test (init → review → validate)**

Run init against an empty tree with the test fixture:

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
pnpm -C frontend --filter @ppt/screen-map cli init --product ppt --add "Phase 2 smoke test"
ls docs/screens/ppt/
```

Expected: a `phase-2-smoke-test.md` file appears under `docs/screens/ppt/`.

Validate it:

```bash
pnpm -C frontend --filter @ppt/screen-map cli validate --root . --strict
```

Expected: clean exit.

Run edit:

```bash
pnpm -C frontend --filter @ppt/screen-map cli edit ppt/phase-2-smoke-test --root .
```

Expected: markdown summary printed.

Cleanup:

```bash
rm docs/screens/ppt/phase-2-smoke-test.md
```

(Skip the review smoke — opens a browser and is interactive; covered by the integration test in Task 13.)

- [ ] **Step 4: Confirm slash command + skills are discoverable**

```bash
ls .claude/commands/screens.md \
   .claude/skills/screen-map-init/SKILL.md \
   .claude/skills/screen-edit/SKILL.md \
   .claude/skills/screen-map-review/SKILL.md
```

Expected: all four files listed.

- [ ] **Step 5: Tag a Phase-2-complete checkpoint commit**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/vigilant-mirzakhani-4baf0e
git commit --allow-empty -m "feat(screen-map): Phase 2 (init, edit, review) complete

Phase 2 in place:
- Polish: schema date coercion, parse CRLF, discover ENOENT narrow,
  context unicode slugify + tests, install-hooks summary refresh.
- DesignSource: interface + ZipAdapter + ClaudeDesignAdapter stub.
- scanCandidates (sitemap + use-cases + epics + design + user-add).
- Visual Review server: hono backend + Preact ESM SPA, integration
  test covers session token gate, screen walk, markdown persistence.
- Skills: screen-map-init (chat-driven grouping), screen-edit
  (focused context loader), screen-map-review (review-server driver).
- /screens dispatcher extended to all 4 subcommands.

Phase 3 (update, render, query, agent self-management protocol,
first bootstrap runs) deferred to a separate plan."
```

- [ ] **Step 6: Push and open PR**

```bash
git push -u origin feature/screen-map-phase-2
gh pr create --title "Screen-Map Phase 2: init, edit, review" --body "$(cat <<'EOF'
## Summary

Phase 2 of the [screen-map system](docs/superpowers/specs/2026-05-07-screen-map-system-design.md). Builds on Phase 1 (PR #220).

- DesignSource adapter layer (interface + ZipAdapter + Claude Design stub).
- Visual Review local server (Hono backend + Preact-via-esm.sh SPA).
- `/screens init`, `/screens edit <id>`, `/screens review` skills.
- Polish: schema date coercion, CRLF body, ENOENT narrowing, Unicode slugify, install-hooks summary refresh.

## Test plan

- [x] `pnpm --filter @ppt/screen-map test` — all suites pass.
- [x] `pnpm biome check packages/screen-map` — clean.
- [x] `pnpm --filter @ppt/screen-map typecheck` — clean.
- [x] End-to-end smoke: init creates files, validate is clean, edit prints summary.
- [x] Review-server integration test covers session walk + markdown persistence.
- [ ] Manual review-server smoke (open the browser and walk one screen) — verify after merge.

Phase 3 (update / render / query / CLAUDE.md addenda / first bootstrap runs) follows in a separate PR.
EOF
)"
```

Expected: PR opened against main (or against `feature/vigilant-mirzakhani-4baf0e` if Phase 1 hasn't merged yet).

---

## Self-Review

1. **Spec coverage:**
   - Spec Section 5.1 (`screen-map-init`) → Tasks 7, 8, 9, 16, 18.
   - Section 5.3 (`screen-map-review`) → Tasks 11–15, 16, 20.
   - Section 5.5 (`screen-edit`) → Tasks 10, 16, 19.
   - Section 6 (Visual Review server) → Tasks 11, 12, 13, 14, 15.
   - Section 7 (DesignSource) → Tasks 4, 5, 6.
   - Phase 1 review-flagged polish items → Tasks 1, 2, 3.
   - `/screens` dispatcher extension → Task 21.
   - Ship checkpoint → Task 22.
   - Spec Sections 5.2 (update), 5.6 (render), 5.7 (query), 9 (agent self-management protocol), bootstrap runs — explicitly deferred to Phase 3 in the plan header.

2. **Placeholder scan:** None of "TBD", "TODO", "implement later", "appropriate error handling", or "similar to task N" used.

3. **Type consistency:**
   - `CandidateScreen` shape consistent across `scan.ts` (defines), `grouping.ts` (consumes), `init-write.ts` (consumes).
   - `DesignFrame` / `DesignSource` consistent across `index.ts`, `zip-adapter.ts`, `claude-design.ts`, `api.ts`, `scan.ts`.
   - `ReviewSession` shape consistent across `session.ts`, `server.ts`, `api.ts`, `start.ts`.
   - CLI flag names: `--root`, `--product`, `--strict`, `--designs`, `--add`, `--decisions`, `--force`, `--preview`, `--playwright`, `--filter`. Each appears identically in cli.ts and in the corresponding skill manifest.

If during execution any of the dependency versions (`hono ^4.6.0`, `@hono/node-server ^1.13.0`, `yauzl-promise ^4.0.0`) have moved, update package.json to match the current latest stable; the API surfaces this plan uses are stable across recent versions.
