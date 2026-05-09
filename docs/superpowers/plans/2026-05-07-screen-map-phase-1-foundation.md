# Screen-Map Phase 1 — Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up the `@ppt/screen-map` package (types, parse, write, validate), the `screen-map-validate` skill, the `/screens` slash command (validate-only stub), the `docs/screens/` skeleton with a template, and pre-commit + CI gates. After Phase 1, anyone can author a screen-map markdown file and have it validated locally and in CI.

**Architecture:** New TS package at `frontend/packages/screen-map`. Frontmatter is a Zod schema; markdown body is preserved verbatim. Validate cross-references the existing `@ppt/sitemap` package (no duplication). Skill + slash command sit in `.claude/`. Pre-commit hook adds a screen-map-validate check; CI runs the same in `--strict` mode on PRs.

**Tech Stack:** TypeScript 5.3+ (strict, ES2022 modules, bundler resolution — match `@ppt/sitemap`), Zod 3.23+, `gray-matter` 4.x for frontmatter parsing, `tsx` for executable TS, `vitest` 2.x for tests, `commander` 12.x for CLI args. pnpm 8.x workspace. Biome for lint/format (project default).

**Spec:** [docs/superpowers/specs/2026-05-07-screen-map-system-design.md](../specs/2026-05-07-screen-map-system-design.md). Phase 1 covers Sections 3 (layout, partial), 4 (file format), 5.4 (`screen-map-validate`), 5.7 partial (`/screens` dispatcher stub), 10 (testing & CI).

**Out of scope for Phase 1 (deferred to Phase 2/3):**

- DesignSource adapters (Phase 2).
- Visual Review server (Phase 2).
- Skills: init, update, review, edit, render, query (Phase 2 / Phase 3).
- `scan.ts` (Phase 2 — only init/update need it).
- CLAUDE.md addenda for agent self-management protocol (Phase 3).
- First bootstrap runs against ppt / reality (Phase 3).

---

## File Structure

### New files

| Path | Responsibility |
|------|----------------|
| `frontend/packages/screen-map/package.json` | pnpm workspace manifest |
| `frontend/packages/screen-map/tsconfig.json` | TypeScript config (mirrors `@ppt/sitemap`) |
| `frontend/packages/screen-map/README.md` | Package overview + dev quickstart |
| `frontend/packages/screen-map/src/index.ts` | Public exports |
| `frontend/packages/screen-map/src/types.ts` | TS types: `ScreenMap`, `Implementation`, `RelatedScreen`, `DiagramRef`, etc. |
| `frontend/packages/screen-map/src/schema.ts` | Zod schemas for frontmatter validation |
| `frontend/packages/screen-map/src/parse.ts` | `parseScreenMap(filePath)` → markdown → `ScreenMap` |
| `frontend/packages/screen-map/src/write.ts` | `writeScreenMap(screen)` → `ScreenMap` → markdown (preserves body) |
| `frontend/packages/screen-map/src/validate.ts` | `validateScreenMap(screen, ctx)` → `ValidationIssue[]` |
| `frontend/packages/screen-map/src/discover.ts` | `discoverScreenMaps(rootDir)` → list `.md` files in `docs/screens/<product>/` |
| `frontend/packages/screen-map/src/cli.ts` | CLI entry (`tsx src/cli.ts validate ...`) |
| `frontend/packages/screen-map/tests/parse.test.ts` | parse round-trip + frontmatter coverage |
| `frontend/packages/screen-map/tests/write.test.ts` | write preserves body, normalises frontmatter |
| `frontend/packages/screen-map/tests/validate.test.ts` | validation rules per Section 5.4 of spec |
| `frontend/packages/screen-map/tests/discover.test.ts` | filesystem walking |
| `frontend/packages/screen-map/tests/fixtures/building-detail.md` | reference fixture for parse/write |
| `frontend/packages/screen-map/tests/fixtures/invalid-frontmatter.md` | fixture covering schema failure |
| `docs/screens/README.md` | format reference + how to use |
| `docs/screens/_template.md` | copy-paste template |
| `docs/screens/.gitkeep` | ensures empty product subdirs are commit-able |
| `docs/screens/ppt/.gitkeep` | placeholder for ppt screens (Phase 2 fills them) |
| `docs/screens/reality/.gitkeep` | placeholder for reality screens |
| `.claude/skills/screen-map-validate/SKILL.md` | skill manifest + instructions |
| `.claude/commands/screens.md` | `/screens` slash command dispatcher (Phase 1: validate only) |
| `.github/workflows/screen-map.yml` | CI: run `validate --strict` on PRs touching `docs/screens/**` or routes |

### Modified files

| Path | Change |
|------|--------|
| `frontend/package.json` | nothing (workspace already discovers `packages/*`) — verify only |
| `scripts/pre-commit` | append a "Screen-map validate" check that runs when `docs/screens/**` is staged |

---

## Task 1: Bootstrap `@ppt/screen-map` package skeleton

**Files:**
- Create: `frontend/packages/screen-map/package.json`
- Create: `frontend/packages/screen-map/tsconfig.json`
- Create: `frontend/packages/screen-map/README.md`
- Create: `frontend/packages/screen-map/src/index.ts`

- [ ] **Step 1: Create package.json**

`frontend/packages/screen-map/package.json`:

```json
{
  "name": "@ppt/screen-map",
  "private": true,
  "description": "Project-management screen-map: types, parse/write, validate, CLI",
  "type": "module",
  "main": "./src/index.ts",
  "types": "./src/index.ts",
  "exports": {
    ".": {
      "types": "./src/index.ts",
      "import": "./src/index.ts"
    }
  },
  "bin": {
    "screen-map": "./src/cli.ts"
  },
  "scripts": {
    "build": "tsc",
    "typecheck": "tsc --noEmit",
    "test": "vitest run",
    "test:watch": "vitest",
    "cli": "tsx src/cli.ts"
  },
  "dependencies": {
    "@ppt/sitemap": "workspace:*",
    "commander": "^12.1.0",
    "gray-matter": "^4.0.3",
    "zod": "^3.23.8"
  },
  "devDependencies": {
    "@types/node": "^20.10.0",
    "tsx": "^4.7.0",
    "typescript": "^5.3.0",
    "vitest": "^2.1.8"
  }
}
```

- [ ] **Step 2: Create tsconfig.json**

`frontend/packages/screen-map/tsconfig.json`:

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "lib": ["ES2022"],
    "strict": true,
    "skipLibCheck": true,
    "esModuleInterop": true,
    "resolveJsonModule": true,
    "declaration": true,
    "declarationMap": true,
    "outDir": "./dist",
    "rootDir": "./src",
    "types": ["node"]
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist"]
}
```

- [ ] **Step 3: Create README.md**

`frontend/packages/screen-map/README.md`:

```markdown
# @ppt/screen-map

TS API + CLI for the screen-map system (see `docs/superpowers/specs/2026-05-07-screen-map-system-design.md`).

## What it does

- Parses `docs/screens/<product>/<id>.md` into typed objects.
- Validates frontmatter (Zod) and cross-references `@ppt/sitemap`.
- Writes back changes preserving body markdown verbatim.
- Exposes a CLI used by the `/screens validate` slash command and the pre-commit hook.

## Quickstart

```bash
pnpm --filter @ppt/screen-map test
pnpm --filter @ppt/screen-map cli validate --strict
```

## Phase 1 scope

Foundation: types, parse, write, validate, discover, CLI (`validate` subcommand).

Phase 2 adds: scan (route detection), DesignSource, Visual Review server.
Phase 3 adds: render, query, agent self-management glue.
```

- [ ] **Step 4: Create src/index.ts placeholder**

`frontend/packages/screen-map/src/index.ts`:

```typescript
export * from './types.js';
export * from './schema.js';
export { parseScreenMap, parseScreenMapString } from './parse.js';
export { writeScreenMap, writeScreenMapString } from './write.js';
export { validateScreenMap, type ValidationIssue, type ValidationContext } from './validate.js';
export { discoverScreenMaps } from './discover.js';
```

- [ ] **Step 5: Install dependencies**

Run from repo root:

```bash
cd frontend && pnpm install
```

Expected: `@ppt/screen-map` is registered in the workspace; new dependencies are added; no errors.

- [ ] **Step 6: Verify build**

```bash
cd frontend && pnpm --filter @ppt/screen-map build
```

Expected: `tsc` will FAIL because the imports in `index.ts` reference files that do not exist yet. That is correct for now — the next tasks add them. Confirm the failures are *only* "Cannot find module './types.js'" and similar, not toolchain errors.

- [ ] **Step 7: Commit**

```bash
git add frontend/packages/screen-map/package.json \
        frontend/packages/screen-map/tsconfig.json \
        frontend/packages/screen-map/README.md \
        frontend/packages/screen-map/src/index.ts \
        frontend/pnpm-lock.yaml
git commit -m "chore(screen-map): bootstrap @ppt/screen-map package skeleton"
```

---

## Task 2: Define core types

**Files:**
- Create: `frontend/packages/screen-map/src/types.ts`

- [ ] **Step 1: Write types.ts**

`frontend/packages/screen-map/src/types.ts`:

```typescript
/**
 * Two products covered by the screen-map system.
 * - `ppt`: Property Management (ppt-web + mobile)
 * - `reality`: Reality Portal (reality-web + mobile-native)
 */
export type Product = 'ppt' | 'reality';

/**
 * All platforms across both products. A given screen will only use the
 * platforms relevant to its product (ppt → ppt-web/mobile;
 * reality → reality-web/mobile-native).
 */
export type Platform = 'ppt-web' | 'reality-web' | 'mobile' | 'mobile-native';

export type BuildStatus = 'planned' | 'in-progress' | 'shipped' | 'n/a';
export type RedesignStatus = 'not-started' | 'in-progress' | 'applied' | 'n/a';
export type ApiStatus = 'stub' | 'partial' | 'complete' | 'n/a';

export type RelatedRel = 'parent' | 'child' | 'action' | 'sibling';
export type DiagramKind = 'sequence' | 'flow' | 'state' | 'class';

export interface Implementation {
  /** URL pattern for ppt-web/reality-web; absent on mobile platforms. */
  route?: string;
  /** Native screen name for mobile/mobile-native; absent on web. */
  screen?: string;
  /** React component or KMP screen class. */
  component?: string;
  buildStatus: BuildStatus;
  redesignStatus: RedesignStatus;
  apiStatus: ApiStatus;
}

export interface RelatedScreen {
  id: string;
  rel: RelatedRel;
}

export interface DiagramRef {
  /** Path or anchor; e.g. `docs/sequence-diagrams.md#building-detail-load`. */
  ref: string;
  kind: DiagramKind;
}

export interface DesignSourceRef {
  adapter: string;
  /** Adapter-specific. ZipAdapter uses `file` + `frame`. */
  file?: string;
  frame: string;
  [key: string]: unknown;
}

export interface ScreenMapFrontmatter {
  id: string;
  name: string;
  product: Product;
  sitemapRefs?: Partial<Record<Platform, string>>;
  implementations: Partial<Record<Platform, Implementation>>;
  endpoints?: string[];
  relatedScreens?: RelatedScreen[];
  sharedComponents?: string[];
  diagrams?: DiagramRef[];
  useCases?: string[];
  epics?: string[];
  designSources?: DesignSourceRef[];
  owner?: string;
  /** ISO date YYYY-MM-DD. */
  lastReview?: string;
}

export interface ScreenMap {
  /** Absolute or repo-relative path of the source markdown file. */
  filePath: string;
  frontmatter: ScreenMapFrontmatter;
  /** Markdown body (everything after the closing frontmatter delimiter). */
  body: string;
}
```

- [ ] **Step 2: Verify build**

```bash
cd frontend && pnpm --filter @ppt/screen-map typecheck
```

Expected: still fails on missing `./schema.js`, `./parse.js`, etc. — but `types.ts` should compile clean. Look for the specific error list and confirm `types.ts` is not among them.

- [ ] **Step 3: Commit**

```bash
git add frontend/packages/screen-map/src/types.ts
git commit -m "feat(screen-map): define core types (ScreenMap, Implementation, ...)"
```

---

## Task 3: Define Zod schema for frontmatter

**Files:**
- Create: `frontend/packages/screen-map/src/schema.ts`
- Create: `frontend/packages/screen-map/tests/schema.test.ts`

- [ ] **Step 1: Write a failing test**

`frontend/packages/screen-map/tests/schema.test.ts`:

```typescript
import { describe, expect, it } from 'vitest';
import { ScreenMapFrontmatterSchema } from '../src/schema.js';

describe('ScreenMapFrontmatterSchema', () => {
  it('accepts a minimal valid frontmatter', () => {
    const valid = {
      id: 'ppt/building-detail',
      name: 'Building Detail',
      product: 'ppt',
      implementations: {
        'ppt-web': {
          route: '/buildings/:id',
          buildStatus: 'shipped',
          redesignStatus: 'applied',
          apiStatus: 'complete',
        },
      },
    };
    const result = ScreenMapFrontmatterSchema.safeParse(valid);
    expect(result.success).toBe(true);
  });

  it('rejects an unknown product', () => {
    const result = ScreenMapFrontmatterSchema.safeParse({
      id: 'foo/bar',
      name: 'Foo',
      product: 'foo',
      implementations: {},
    });
    expect(result.success).toBe(false);
  });

  it('rejects an unknown buildStatus', () => {
    const result = ScreenMapFrontmatterSchema.safeParse({
      id: 'ppt/x',
      name: 'X',
      product: 'ppt',
      implementations: {
        'ppt-web': {
          buildStatus: 'launched',
          redesignStatus: 'applied',
          apiStatus: 'complete',
        },
      },
    });
    expect(result.success).toBe(false);
  });

  it('requires id to match <product>/<slug> pattern', () => {
    const result = ScreenMapFrontmatterSchema.safeParse({
      id: 'building-detail',
      name: 'Building Detail',
      product: 'ppt',
      implementations: {},
    });
    expect(result.success).toBe(false);
  });

  it('requires lastReview to be ISO date if present', () => {
    const result = ScreenMapFrontmatterSchema.safeParse({
      id: 'ppt/x',
      name: 'X',
      product: 'ppt',
      implementations: {},
      lastReview: '01/05/2026',
    });
    expect(result.success).toBe(false);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd frontend && pnpm --filter @ppt/screen-map test schema
```

Expected: FAIL with "Cannot find module '../src/schema.js'".

- [ ] **Step 3: Implement schema.ts**

`frontend/packages/screen-map/src/schema.ts`:

```typescript
import { z } from 'zod';

export const ProductSchema = z.enum(['ppt', 'reality']);

export const PlatformSchema = z.enum([
  'ppt-web',
  'reality-web',
  'mobile',
  'mobile-native',
]);

export const BuildStatusSchema = z.enum([
  'planned',
  'in-progress',
  'shipped',
  'n/a',
]);
export const RedesignStatusSchema = z.enum([
  'not-started',
  'in-progress',
  'applied',
  'n/a',
]);
export const ApiStatusSchema = z.enum([
  'stub',
  'partial',
  'complete',
  'n/a',
]);

export const RelatedRelSchema = z.enum(['parent', 'child', 'action', 'sibling']);
export const DiagramKindSchema = z.enum(['sequence', 'flow', 'state', 'class']);

export const ImplementationSchema = z.object({
  route: z.string().optional(),
  screen: z.string().optional(),
  component: z.string().optional(),
  buildStatus: BuildStatusSchema,
  redesignStatus: RedesignStatusSchema,
  apiStatus: ApiStatusSchema,
});

export const RelatedScreenSchema = z.object({
  id: z.string().regex(/^[a-z]+\/[a-z0-9-]+$/, {
    message: 'related screen id must match <product>/<slug>',
  }),
  rel: RelatedRelSchema,
});

export const DiagramRefSchema = z.object({
  ref: z.string().min(1),
  kind: DiagramKindSchema,
});

export const DesignSourceRefSchema = z
  .object({
    adapter: z.string().min(1),
    file: z.string().optional(),
    frame: z.string().min(1),
  })
  .passthrough();

const IdSchema = z.string().regex(/^(ppt|reality)\/[a-z0-9-]+$/, {
  message: 'id must match <product>/<slug> using kebab-case',
});

const IsoDateSchema = z
  .string()
  .regex(/^\d{4}-\d{2}-\d{2}$/, { message: 'must be YYYY-MM-DD' });

export const ScreenMapFrontmatterSchema = z
  .object({
    id: IdSchema,
    name: z.string().min(1),
    product: ProductSchema,
    sitemapRefs: z.record(PlatformSchema, z.string()).optional(),
    implementations: z.record(PlatformSchema, ImplementationSchema),
    endpoints: z.array(z.string()).optional(),
    relatedScreens: z.array(RelatedScreenSchema).optional(),
    sharedComponents: z.array(z.string()).optional(),
    diagrams: z.array(DiagramRefSchema).optional(),
    useCases: z.array(z.string()).optional(),
    epics: z.array(z.string()).optional(),
    designSources: z.array(DesignSourceRefSchema).optional(),
    owner: z.string().optional(),
    lastReview: IsoDateSchema.optional(),
  })
  .superRefine((value, ctx) => {
    const [productPrefix] = value.id.split('/');
    if (productPrefix !== value.product) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ['id'],
        message: `id prefix "${productPrefix}" does not match product "${value.product}"`,
      });
    }
  });

export type ScreenMapFrontmatterInput = z.input<
  typeof ScreenMapFrontmatterSchema
>;
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd frontend && pnpm --filter @ppt/screen-map test schema
```

Expected: PASS, 5 tests.

- [ ] **Step 5: Commit**

```bash
git add frontend/packages/screen-map/src/schema.ts \
        frontend/packages/screen-map/tests/schema.test.ts
git commit -m "feat(screen-map): Zod schema for frontmatter"
```

---

## Task 4: Implement parse (markdown → ScreenMap)

**Files:**
- Create: `frontend/packages/screen-map/src/parse.ts`
- Create: `frontend/packages/screen-map/tests/parse.test.ts`
- Create: `frontend/packages/screen-map/tests/fixtures/building-detail.md`
- Create: `frontend/packages/screen-map/tests/fixtures/invalid-frontmatter.md`

- [ ] **Step 1: Create the valid fixture**

`frontend/packages/screen-map/tests/fixtures/building-detail.md`:

```markdown
---
id: ppt/building-detail
name: Building Detail
product: ppt
sitemapRefs:
  ppt-web: ppt-building-detail
  mobile: mobile-building-detail-screen
implementations:
  ppt-web:
    route: /buildings/:id
    component: BuildingDetailPage
    buildStatus: shipped
    redesignStatus: applied
    apiStatus: complete
  mobile:
    screen: BuildingDetailScreen
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: complete
endpoints:
  - building_get
  - building_update
  - units_list
relatedScreens:
  - { id: ppt/buildings-list, rel: parent }
  - { id: ppt/building-edit, rel: action }
sharedComponents:
  - BuildingHeader
  - UnitsTable
diagrams:
  - { ref: docs/sequence-diagrams.md#building-detail-load, kind: sequence }
useCases: [UC-12, UC-13]
epics: [Epic-15]
owner: pm-frontend
lastReview: 2026-05-04
---

## Functionality Checklist

- [x] [w,m] View building info
- [ ] [m] Edit building info (planned)

## Notes

### Broader context
Header card pattern is shared with `reality/property-detail`.

## Agent Log

- 2026-05-07 — agent: initial seed.
```

- [ ] **Step 2: Create the invalid fixture**

`frontend/packages/screen-map/tests/fixtures/invalid-frontmatter.md`:

```markdown
---
id: building-detail
name: Building Detail
product: ppt
implementations: {}
---

Body content.
```

- [ ] **Step 3: Write a failing test**

`frontend/packages/screen-map/tests/parse.test.ts`:

```typescript
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { describe, expect, it } from 'vitest';
import { parseScreenMap, parseScreenMapString } from '../src/parse.js';

const fixturesDir = path.dirname(fileURLToPath(import.meta.url));
const validFixture = path.join(fixturesDir, 'fixtures/building-detail.md');
const invalidFixture = path.join(fixturesDir, 'fixtures/invalid-frontmatter.md');

describe('parseScreenMap', () => {
  it('reads frontmatter and body from a file', async () => {
    const screen = await parseScreenMap(validFixture);
    expect(screen.frontmatter.id).toBe('ppt/building-detail');
    expect(screen.frontmatter.product).toBe('ppt');
    expect(screen.frontmatter.implementations['ppt-web']?.buildStatus).toBe(
      'shipped',
    );
    expect(screen.body).toContain('## Functionality Checklist');
    expect(screen.body).toContain('## Agent Log');
    expect(screen.filePath).toBe(validFixture);
  });

  it('throws a descriptive error on invalid frontmatter', async () => {
    await expect(parseScreenMap(invalidFixture)).rejects.toThrow(
      /id must match/,
    );
  });
});

describe('parseScreenMapString', () => {
  it('parses an in-memory markdown string', async () => {
    const raw = await readFile(validFixture, 'utf8');
    const screen = parseScreenMapString(raw, '<inline>');
    expect(screen.frontmatter.id).toBe('ppt/building-detail');
    expect(screen.filePath).toBe('<inline>');
  });
});
```

- [ ] **Step 4: Run test to verify it fails**

```bash
cd frontend && pnpm --filter @ppt/screen-map test parse
```

Expected: FAIL with "Cannot find module '../src/parse.js'".

- [ ] **Step 5: Implement parse.ts**

`frontend/packages/screen-map/src/parse.ts`:

```typescript
import { readFile } from 'node:fs/promises';
import matter from 'gray-matter';
import { ScreenMapFrontmatterSchema } from './schema.js';
import type { ScreenMap } from './types.js';

export class ScreenMapParseError extends Error {
  constructor(
    public readonly filePath: string,
    public readonly issues: string[],
  ) {
    super(`Invalid screen-map at ${filePath}:\n  - ${issues.join('\n  - ')}`);
    this.name = 'ScreenMapParseError';
  }
}

export function parseScreenMapString(
  source: string,
  filePath: string,
): ScreenMap {
  const parsed = matter(source);
  const result = ScreenMapFrontmatterSchema.safeParse(parsed.data);
  if (!result.success) {
    const issues = result.error.issues.map((i) => {
      const path = i.path.join('.') || '<root>';
      return `${path}: ${i.message}`;
    });
    throw new ScreenMapParseError(filePath, issues);
  }
  return {
    filePath,
    frontmatter: result.data,
    body: parsed.content.replace(/^\n/, ''),
  };
}

export async function parseScreenMap(filePath: string): Promise<ScreenMap> {
  const source = await readFile(filePath, 'utf8');
  return parseScreenMapString(source, filePath);
}
```

- [ ] **Step 6: Run test to verify it passes**

```bash
cd frontend && pnpm --filter @ppt/screen-map test parse
```

Expected: PASS, 3 tests.

- [ ] **Step 7: Commit**

```bash
git add frontend/packages/screen-map/src/parse.ts \
        frontend/packages/screen-map/tests/parse.test.ts \
        frontend/packages/screen-map/tests/fixtures/
git commit -m "feat(screen-map): parse markdown into ScreenMap (gray-matter + Zod)"
```

---

## Task 5: Implement write (ScreenMap → markdown)

**Files:**
- Create: `frontend/packages/screen-map/src/write.ts`
- Create: `frontend/packages/screen-map/tests/write.test.ts`

- [ ] **Step 1: Write a failing test**

`frontend/packages/screen-map/tests/write.test.ts`:

```typescript
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { describe, expect, it } from 'vitest';
import { parseScreenMap, parseScreenMapString } from '../src/parse.js';
import { writeScreenMapString } from '../src/write.js';

const fixturesDir = path.dirname(fileURLToPath(import.meta.url));
const validFixture = path.join(fixturesDir, 'fixtures/building-detail.md');

describe('writeScreenMapString', () => {
  it('preserves the markdown body verbatim across a parse/write round-trip', async () => {
    const original = await readFile(validFixture, 'utf8');
    const parsed = await parseScreenMap(validFixture);
    const written = writeScreenMapString(parsed);
    const reparsed = parseScreenMapString(written, '<inline>');
    expect(reparsed.body).toBe(parsed.body);
  });

  it('reflects mutated frontmatter values', async () => {
    const parsed = await parseScreenMap(validFixture);
    parsed.frontmatter.implementations['mobile']!.redesignStatus = 'applied';
    parsed.frontmatter.lastReview = '2026-05-08';
    const written = writeScreenMapString(parsed);
    expect(written).toMatch(/redesignStatus: applied/);
    expect(written).toMatch(/lastReview: 2026-05-08/);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd frontend && pnpm --filter @ppt/screen-map test write
```

Expected: FAIL with "Cannot find module '../src/write.js'".

- [ ] **Step 3: Implement write.ts**

`frontend/packages/screen-map/src/write.ts`:

```typescript
import { writeFile } from 'node:fs/promises';
import matter from 'gray-matter';
import type { ScreenMap } from './types.js';

/**
 * Serialises a ScreenMap back to markdown. The frontmatter is regenerated from
 * the typed object (ordering is whatever gray-matter chooses); the body is
 * preserved exactly as supplied.
 */
export function writeScreenMapString(screen: ScreenMap): string {
  const yaml = matter.stringify('', screen.frontmatter, {
    language: 'yaml',
  });
  // gray-matter's stringify produces "---\n<yaml>\n---\n" with an extra
  // trailing newline; we want the body to start exactly one blank line below
  // the closing fence.
  const trimmed = yaml.replace(/\n+$/, '');
  return `${trimmed}\n\n${screen.body}`;
}

export async function writeScreenMap(screen: ScreenMap): Promise<void> {
  const serialised = writeScreenMapString(screen);
  await writeFile(screen.filePath, serialised, 'utf8');
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd frontend && pnpm --filter @ppt/screen-map test write
```

Expected: PASS, 2 tests.

- [ ] **Step 5: Commit**

```bash
git add frontend/packages/screen-map/src/write.ts \
        frontend/packages/screen-map/tests/write.test.ts
git commit -m "feat(screen-map): write ScreenMap to markdown preserving body"
```

---

## Task 6: Implement discover (filesystem walking)

**Files:**
- Create: `frontend/packages/screen-map/src/discover.ts`
- Create: `frontend/packages/screen-map/tests/discover.test.ts`

- [ ] **Step 1: Write a failing test**

`frontend/packages/screen-map/tests/discover.test.ts`:

```typescript
import { mkdir, writeFile, rm } from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { discoverScreenMaps } from '../src/discover.js';

let tmpRoot: string;

beforeEach(async () => {
  tmpRoot = await mkdtemp(path.join(os.tmpdir(), 'screen-map-discover-'));
});

afterEach(async () => {
  await rm(tmpRoot, { recursive: true, force: true });
});

async function mkdtemp(prefix: string): Promise<string> {
  const { mkdtemp } = await import('node:fs/promises');
  return mkdtemp(prefix);
}

describe('discoverScreenMaps', () => {
  it('finds .md files under <root>/<product>/', async () => {
    await mkdir(path.join(tmpRoot, 'ppt'), { recursive: true });
    await mkdir(path.join(tmpRoot, 'reality'), { recursive: true });
    await writeFile(path.join(tmpRoot, 'ppt', 'a.md'), '---\nid: ppt/a\n---\n');
    await writeFile(path.join(tmpRoot, 'ppt', 'b.md'), '---\nid: ppt/b\n---\n');
    await writeFile(
      path.join(tmpRoot, 'reality', 'c.md'),
      '---\nid: reality/c\n---\n',
    );

    const found = await discoverScreenMaps(tmpRoot);
    const ids = found.map((f) => path.relative(tmpRoot, f)).sort();
    expect(ids).toEqual(['ppt/a.md', 'ppt/b.md', 'reality/c.md']);
  });

  it('ignores README.md, _template.md, and .gitkeep', async () => {
    await mkdir(path.join(tmpRoot, 'ppt'), { recursive: true });
    await writeFile(path.join(tmpRoot, 'README.md'), '');
    await writeFile(path.join(tmpRoot, '_template.md'), '');
    await writeFile(path.join(tmpRoot, 'ppt', '.gitkeep'), '');
    await writeFile(path.join(tmpRoot, 'ppt', 'a.md'), '---\nid: ppt/a\n---\n');
    const found = await discoverScreenMaps(tmpRoot);
    expect(found).toHaveLength(1);
    expect(found[0].endsWith('ppt/a.md')).toBe(true);
  });

  it('returns [] when the root directory does not exist', async () => {
    const found = await discoverScreenMaps(path.join(tmpRoot, 'missing'));
    expect(found).toEqual([]);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd frontend && pnpm --filter @ppt/screen-map test discover
```

Expected: FAIL with "Cannot find module '../src/discover.js'".

- [ ] **Step 3: Implement discover.ts**

`frontend/packages/screen-map/src/discover.ts`:

```typescript
import { readdir, stat } from 'node:fs/promises';
import path from 'node:path';

const PRODUCT_DIRS = ['ppt', 'reality'];
const IGNORED = new Set(['README.md', '_template.md']);

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

- [ ] **Step 4: Run test to verify it passes**

```bash
cd frontend && pnpm --filter @ppt/screen-map test discover
```

Expected: PASS, 3 tests.

- [ ] **Step 5: Commit**

```bash
git add frontend/packages/screen-map/src/discover.ts \
        frontend/packages/screen-map/tests/discover.test.ts
git commit -m "feat(screen-map): discover screen-maps under docs/screens/<product>/"
```

---

## Task 7: Implement validate

**Files:**
- Create: `frontend/packages/screen-map/src/validate.ts`
- Create: `frontend/packages/screen-map/tests/validate.test.ts`

- [ ] **Step 1: Confirm `@ppt/sitemap` export names**

```bash
grep -E "^export " frontend/packages/sitemap/src/data/index.ts
```

Expected names (verified at plan time): `pptWebRoutes`, `realityWebRoutes`, `mobileScreens`, `apiServerEndpoints`, `realityServerEndpoints`, `allFlows`, plus helper functions. The two endpoint arrays are merged into a single `knownEndpointIds` set in `context.ts` (Task 8 Step 1). If the export names have drifted, rename in `context.ts` accordingly — the rest of the chain is unaffected.

- [ ] **Step 2: Write a failing test**

`frontend/packages/screen-map/tests/validate.test.ts`:

```typescript
import { describe, expect, it } from 'vitest';
import { validateScreenMap } from '../src/validate.js';
import type { ScreenMap, ValidationContext } from '../src/index.js';

const ctx: ValidationContext = {
  knownEndpointIds: new Set(['building_get', 'building_update', 'units_list']),
  knownSitemapIds: new Set([
    'ppt-building-detail',
    'mobile-building-detail-screen',
    'ppt-buildings-list',
  ]),
  knownScreenIds: new Set([
    'ppt/building-detail',
    'ppt/buildings-list',
    'ppt/building-edit',
  ]),
  resolveDiagramRef: (ref) => ref === 'docs/sequence-diagrams.md#building-detail-load',
};

const baseScreen = (): ScreenMap => ({
  filePath: 'docs/screens/ppt/building-detail.md',
  body: '## Notes\n',
  frontmatter: {
    id: 'ppt/building-detail',
    name: 'Building Detail',
    product: 'ppt',
    sitemapRefs: {
      'ppt-web': 'ppt-building-detail',
      mobile: 'mobile-building-detail-screen',
    },
    implementations: {
      'ppt-web': {
        route: '/buildings/:id',
        buildStatus: 'shipped',
        redesignStatus: 'applied',
        apiStatus: 'complete',
      },
    },
    endpoints: ['building_get', 'units_list'],
    relatedScreens: [{ id: 'ppt/buildings-list', rel: 'parent' }],
    diagrams: [
      { ref: 'docs/sequence-diagrams.md#building-detail-load', kind: 'sequence' },
    ],
  },
});

describe('validateScreenMap', () => {
  it('returns no issues for a screen wired up correctly', () => {
    const issues = validateScreenMap(baseScreen(), ctx);
    expect(issues).toEqual([]);
  });

  it('flags an unknown endpoint id', () => {
    const screen = baseScreen();
    screen.frontmatter.endpoints!.push('mystery_endpoint');
    const issues = validateScreenMap(screen, ctx);
    expect(issues).toEqual([
      {
        severity: 'error',
        path: 'endpoints[2]',
        message: 'unknown endpoint id "mystery_endpoint" — not present in @ppt/sitemap',
      },
    ]);
  });

  it('flags an unknown sitemap ref', () => {
    const screen = baseScreen();
    screen.frontmatter.sitemapRefs = { 'ppt-web': 'ppt-bogus' };
    const issues = validateScreenMap(screen, ctx);
    expect(issues).toContainEqual({
      severity: 'error',
      path: 'sitemapRefs.ppt-web',
      message: 'unknown sitemap id "ppt-bogus" — not present in @ppt/sitemap',
    });
  });

  it('flags a related screen that does not exist', () => {
    const screen = baseScreen();
    screen.frontmatter.relatedScreens = [{ id: 'ppt/missing', rel: 'child' }];
    const issues = validateScreenMap(screen, ctx);
    expect(issues).toContainEqual({
      severity: 'error',
      path: 'relatedScreens[0].id',
      message: 'related screen "ppt/missing" does not exist',
    });
  });

  it('flags a diagram ref that does not resolve', () => {
    const screen = baseScreen();
    screen.frontmatter.diagrams = [
      { ref: 'docs/no-such.md#anchor', kind: 'sequence' },
    ];
    const issues = validateScreenMap(screen, ctx);
    expect(issues).toContainEqual({
      severity: 'error',
      path: 'diagrams[0].ref',
      message: 'diagram ref "docs/no-such.md#anchor" does not resolve',
    });
  });

  it('flags a product/id mismatch (covered by schema, but enforced again here as a guard)', () => {
    const screen = baseScreen();
    screen.frontmatter.id = 'reality/building-detail';
    screen.frontmatter.product = 'ppt';
    const issues = validateScreenMap(screen, ctx);
    expect(issues.some((i) => i.path === 'id')).toBe(true);
  });
});
```

- [ ] **Step 3: Run test to verify it fails**

```bash
cd frontend && pnpm --filter @ppt/screen-map test validate
```

Expected: FAIL with "Cannot find module '../src/validate.js'".

- [ ] **Step 4: Implement validate.ts**

`frontend/packages/screen-map/src/validate.ts`:

```typescript
import type { ScreenMap } from './types.js';

export interface ValidationIssue {
  severity: 'error' | 'warning';
  path: string;
  message: string;
}

export interface ValidationContext {
  knownEndpointIds: Set<string>;
  knownSitemapIds: Set<string>;
  knownScreenIds: Set<string>;
  /**
   * Resolve a `diagrams[].ref` value. Implementations check filesystem
   * existence and (for `path#anchor`) the presence of the anchor.
   */
  resolveDiagramRef: (ref: string) => boolean;
}

export function validateScreenMap(
  screen: ScreenMap,
  ctx: ValidationContext,
): ValidationIssue[] {
  const issues: ValidationIssue[] = [];
  const { frontmatter } = screen;

  // Guard the product/id alignment again at validate time.
  const [productPrefix] = frontmatter.id.split('/');
  if (productPrefix !== frontmatter.product) {
    issues.push({
      severity: 'error',
      path: 'id',
      message: `id prefix "${productPrefix}" does not match product "${frontmatter.product}"`,
    });
  }

  // Endpoints must exist in @ppt/sitemap.
  if (frontmatter.endpoints) {
    frontmatter.endpoints.forEach((endpointId, idx) => {
      if (!ctx.knownEndpointIds.has(endpointId)) {
        issues.push({
          severity: 'error',
          path: `endpoints[${idx}]`,
          message: `unknown endpoint id "${endpointId}" — not present in @ppt/sitemap`,
        });
      }
    });
  }

  // Sitemap refs must exist.
  if (frontmatter.sitemapRefs) {
    for (const [platform, sitemapId] of Object.entries(frontmatter.sitemapRefs)) {
      if (!sitemapId) continue;
      if (!ctx.knownSitemapIds.has(sitemapId)) {
        issues.push({
          severity: 'error',
          path: `sitemapRefs.${platform}`,
          message: `unknown sitemap id "${sitemapId}" — not present in @ppt/sitemap`,
        });
      }
    }
  }

  // Related screens must exist.
  if (frontmatter.relatedScreens) {
    frontmatter.relatedScreens.forEach((rel, idx) => {
      if (!ctx.knownScreenIds.has(rel.id)) {
        issues.push({
          severity: 'error',
          path: `relatedScreens[${idx}].id`,
          message: `related screen "${rel.id}" does not exist`,
        });
      }
    });
  }

  // Diagrams must resolve.
  if (frontmatter.diagrams) {
    frontmatter.diagrams.forEach((diagram, idx) => {
      if (!ctx.resolveDiagramRef(diagram.ref)) {
        issues.push({
          severity: 'error',
          path: `diagrams[${idx}].ref`,
          message: `diagram ref "${diagram.ref}" does not resolve`,
        });
      }
    });
  }

  return issues;
}
```

- [ ] **Step 5: Run test to verify it passes**

```bash
cd frontend && pnpm --filter @ppt/screen-map test validate
```

Expected: PASS, 6 tests.

- [ ] **Step 6: Commit**

```bash
git add frontend/packages/screen-map/src/validate.ts \
        frontend/packages/screen-map/tests/validate.test.ts
git commit -m "feat(screen-map): validate consistency against sitemap + related screens"
```

---

## Task 8: Build CLI (`tsx src/cli.ts validate`)

**Files:**
- Create: `frontend/packages/screen-map/src/cli.ts`
- Create: `frontend/packages/screen-map/src/context.ts`
- Create: `frontend/packages/screen-map/tests/cli.test.ts`

- [ ] **Step 1: Implement context.ts (sitemap + diagram-ref helpers)**

`frontend/packages/screen-map/src/context.ts`:

```typescript
import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';
import {
  apiServerEndpoints,
  realityServerEndpoints,
  pptWebRoutes,
  realityWebRoutes,
  mobileScreens,
} from '@ppt/sitemap';
import { discoverScreenMaps } from './discover.js';
import { parseScreenMap } from './parse.js';
import type { ValidationContext } from './validate.js';

export interface BuildContextOptions {
  repoRoot: string;
  /** Defaults to `<repoRoot>/docs/screens`. */
  screensDir?: string;
}

/**
 * Build a ValidationContext by collecting sitemap IDs, screen-map IDs, and a
 * filesystem-aware diagram-ref resolver.
 */
export async function buildValidationContext(
  options: BuildContextOptions,
): Promise<ValidationContext> {
  const screensDir =
    options.screensDir ?? path.join(options.repoRoot, 'docs/screens');

  const knownEndpointIds = new Set([
    ...apiServerEndpoints.map((e) => e.id),
    ...realityServerEndpoints.map((e) => e.id),
  ]);
  const knownSitemapIds = new Set([
    ...pptWebRoutes.map((r) => r.id),
    ...realityWebRoutes.map((r) => r.id),
    ...mobileScreens.map((s) => s.id),
  ]);

  const screenFiles = await discoverScreenMaps(screensDir);
  const knownScreenIds = new Set<string>();
  for (const file of screenFiles) {
    try {
      const screen = await parseScreenMap(file);
      knownScreenIds.add(screen.frontmatter.id);
    } catch {
      // ignore here — the CLI itself reports per-file errors below
    }
  }

  return {
    knownEndpointIds,
    knownSitemapIds,
    knownScreenIds,
    resolveDiagramRef: (ref) => resolveDiagramRef(ref, options.repoRoot),
  };
}

function resolveDiagramRef(ref: string, repoRoot: string): boolean {
  const [filePart, anchor] = ref.split('#');
  if (!filePart) return false;
  const abs = path.isAbsolute(filePart)
    ? filePart
    : path.join(repoRoot, filePart);
  if (!existsSync(abs)) return false;
  if (!anchor) return true;
  // Best-effort: check the anchor appears as a `#`/`##`/... heading slug.
  try {
    const content = readFileSync(abs, 'utf8');
    const slugs = extractHeadingSlugs(content);
    return slugs.has(anchor);
  } catch {
    return false;
  }
}

function extractHeadingSlugs(markdown: string): Set<string> {
  const slugs = new Set<string>();
  const headingRe = /^#{1,6}\s+(.+?)\s*$/gm;
  let m: RegExpExecArray | null;
  while ((m = headingRe.exec(markdown))) {
    slugs.add(slugify(m[1]));
  }
  return slugs;
}

function slugify(s: string): string {
  return s
    .toLowerCase()
    .replace(/[^a-z0-9\s-]/g, '')
    .trim()
    .replace(/\s+/g, '-');
}
```

- [ ] **Step 2: Write a failing CLI integration test**

`frontend/packages/screen-map/tests/cli.test.ts`:

```typescript
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { mkdir, writeFile, rm, mkdtemp, cp } from 'node:fs/promises';
import os from 'node:os';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';

const execFileP = promisify(execFile);
const here = path.dirname(fileURLToPath(import.meta.url));
const cliPath = path.join(here, '..', 'src', 'cli.ts');

let tmpRepo: string;

beforeEach(async () => {
  tmpRepo = await mkdtemp(path.join(os.tmpdir(), 'screen-map-cli-'));
  // Minimal fake repo: docs/screens/ppt/<one valid screen using real sitemap ids>.
  await mkdir(path.join(tmpRepo, 'docs/screens/ppt'), { recursive: true });
});

afterEach(async () => {
  await rm(tmpRepo, { recursive: true, force: true });
});

async function run(args: string[]): Promise<{ stdout: string; stderr: string; code: number }> {
  try {
    const { stdout, stderr } = await execFileP('npx', ['tsx', cliPath, ...args], {
      env: { ...process.env, NO_COLOR: '1' },
    });
    return { stdout, stderr, code: 0 };
  } catch (err: unknown) {
    const e = err as { stdout?: string; stderr?: string; code?: number };
    return { stdout: e.stdout ?? '', stderr: e.stderr ?? '', code: e.code ?? 1 };
  }
}

describe('cli', () => {
  it('exits 0 when no screens exist (empty repo)', async () => {
    const result = await run(['validate', '--root', tmpRepo]);
    expect(result.code).toBe(0);
    expect(result.stdout).toMatch(/0 screen-maps/i);
  });

  it('exits 1 in --strict on a parse error', async () => {
    await writeFile(
      path.join(tmpRepo, 'docs/screens/ppt/bad.md'),
      '---\nid: bad\nname: Bad\nproduct: ppt\nimplementations: {}\n---\n',
    );
    const result = await run(['validate', '--root', tmpRepo, '--strict']);
    expect(result.code).toBe(1);
    expect(result.stderr + result.stdout).toMatch(/id must match/);
  });
});
```

- [ ] **Step 3: Run test to verify it fails**

```bash
cd frontend && pnpm --filter @ppt/screen-map test cli
```

Expected: FAIL — `cli.ts` not yet exported as the entry; either "Cannot find module" or "command failed".

- [ ] **Step 4: Implement cli.ts**

`frontend/packages/screen-map/src/cli.ts`:

```typescript
#!/usr/bin/env -S npx tsx
import path from 'node:path';
import { Command } from 'commander';
import { buildValidationContext } from './context.js';
import { discoverScreenMaps } from './discover.js';
import { parseScreenMap, ScreenMapParseError } from './parse.js';
import { validateScreenMap } from './validate.js';

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

program.parseAsync().catch((err) => {
  process.stderr.write(`Unexpected error: ${(err as Error).message}\n`);
  process.exit(2);
});
```

- [ ] **Step 5: Run test to verify it passes**

```bash
cd frontend && pnpm --filter @ppt/screen-map test cli
```

Expected: PASS, 2 tests. (CLI test invokes `npx tsx ...`; if `tsx` is not on PATH, ensure devDependency from Task 1 is installed.)

- [ ] **Step 6: Run full test suite for the package**

```bash
cd frontend && pnpm --filter @ppt/screen-map test
```

Expected: PASS for all suites — schema, parse, write, discover, validate, cli.

- [ ] **Step 7: Commit**

```bash
git add frontend/packages/screen-map/src/cli.ts \
        frontend/packages/screen-map/src/context.ts \
        frontend/packages/screen-map/tests/cli.test.ts
git commit -m "feat(screen-map): CLI \`validate\` subcommand"
```

---

## Task 9: Create `docs/screens/` skeleton (README + template + .gitkeeps)

**Files:**
- Create: `docs/screens/README.md`
- Create: `docs/screens/_template.md`
- Create: `docs/screens/ppt/.gitkeep`
- Create: `docs/screens/reality/.gitkeep`

- [ ] **Step 1: Create README.md**

`docs/screens/README.md`:

```markdown
# Screen-Map

Project-management layer for both products (PPT and Reality). One file per logical screen; each file mixes a typed YAML frontmatter (status, endpoints, relations) with free-form markdown (functionality checklist, states, notes, agent log).

See the design spec: [`docs/superpowers/specs/2026-05-07-screen-map-system-design.md`](../superpowers/specs/2026-05-07-screen-map-system-design.md).

## Layout

```
docs/screens/
├── README.md
├── _template.md           # copy when creating a new screen
├── ppt/                   # PPT product (ppt-web + mobile)
│   └── <kebab-id>.md
└── reality/               # Reality product (reality-web + mobile-native)
    └── <kebab-id>.md
```

## Frontmatter contract

Authoritative status lives in the frontmatter; markdown body holds the things humans read and edit. The `@ppt/screen-map` package validates the shape; Phase 2 skills mutate it.

Every entry must have:

- `id: <product>/<kebab-slug>` matching the file path.
- `product: ppt` or `reality`.
- `implementations.<platform>` for every platform that *exists or will exist*; use `n/a` statuses if the platform is intentionally not in scope.

See `_template.md` for a full skeleton.

## Tooling

- `/screens validate` — run the validator against this whole tree.
- Pre-commit hook auto-validates any `docs/screens/**` you stage.
- CI re-runs `validate --strict` on PRs that touch `docs/screens/**` or route files.

Phase 2/3 add: init, update, review (with a visual UI), edit, render (mermaid), query.
```

- [ ] **Step 2: Create _template.md**

`docs/screens/_template.md`:

```markdown
---
id: <product>/<kebab-slug>
name: <Human Name>
product: <ppt|reality>
sitemapRefs:
  # ppt-web: <route-id-from-@ppt/sitemap>
  # mobile: <screen-id-from-@ppt/sitemap>
implementations:
  ppt-web:
    route: /...
    component: <Component>
    buildStatus: planned          # planned | in-progress | shipped | n/a
    redesignStatus: not-started   # not-started | in-progress | applied | n/a
    apiStatus: stub               # stub | partial | complete | n/a
  mobile:
    screen: <ScreenName>
    buildStatus: planned
    redesignStatus: not-started
    apiStatus: stub
endpoints: []
relatedScreens: []
sharedComponents: []
diagrams: []
useCases: []
epics: []
designSources: []
owner: <team>
# lastReview: 2026-05-07
---

## Functionality Checklist

Tag each item with the platforms it applies to: `[w]`, `[m]`, `[w,m]`, or `[-]`.

- [ ] [w,m] ...

## States

- **Empty**:
- **Loading**:
- **Error**:

## Notes

### Broader context

### Specific (recent)

## Agent Log

<!-- newest entries on top -->
```

- [ ] **Step 3: Create the .gitkeep files**

```bash
mkdir -p docs/screens/ppt docs/screens/reality
touch docs/screens/ppt/.gitkeep docs/screens/reality/.gitkeep
```

- [ ] **Step 4: Run validate against the new empty tree**

```bash
cd frontend && pnpm --filter @ppt/screen-map cli validate --root ../
```

Expected: `Validated 0 screen-maps: 0 errors, 0 warnings.`

- [ ] **Step 5: Commit**

```bash
git add docs/screens/
git commit -m "docs(screens): skeleton (README, template, product subdirs)"
```

---

## Task 10: Create `screen-map-validate` skill

**Files:**
- Create: `.claude/skills/screen-map-validate/SKILL.md`

- [ ] **Step 1: Write SKILL.md**

`.claude/skills/screen-map-validate/SKILL.md`:

```markdown
---
name: screen-map-validate
description: Validate every screen-map under docs/screens/ against frontmatter schema, @ppt/sitemap IDs, related-screen IDs, and diagram refs. Use when the user runs `/screens validate` or asks to check screen-map consistency. Triggers also include "validate screens", "screen-map drift", "is the screen-map clean".
---

# Screen-Map Validate Skill

Runs the `@ppt/screen-map` CLI in validate mode against the repo's `docs/screens/` tree.

## When to use

- User invokes `/screens validate` (with or without `--strict`).
- User asks to "validate the screen-map", "check screen-map consistency", or to confirm screen-maps are not stale.
- After editing any frontmatter in `docs/screens/<product>/*.md`.

## Inputs

- Optional `--strict` flag (forwarded to the CLI; non-zero exit on any error).
- Optional `--root <path>` flag (forwarded; defaults to repo root).

## What it does

1. Resolves the repo root (`git rev-parse --show-toplevel`).
2. Runs `pnpm --filter @ppt/screen-map cli validate --root <repoRoot> [--strict]`.
3. Reports the number of files validated and any issues.
4. If issues are returned, summarises them per file with `path :: message`. Offers to open the offending file.

## Implementation

```bash
REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT/frontend"
pnpm --filter @ppt/screen-map cli validate --root "$REPO_ROOT" "$@"
```

## Output handling

- All-clean → reply: "Screen-map clean: validated N screen-maps."
- Has errors → echo the CLI output verbatim, then ask the user whether to open the first failing file for editing.
- `--strict` exits non-zero — surface that to the caller (the slash command will pass it along).
```

- [ ] **Step 2: Sanity-check the skill loads**

```bash
ls .claude/skills/screen-map-validate/SKILL.md
```

Expected: file exists. (Skill discovery happens automatically; no registration step needed.)

- [ ] **Step 3: Commit**

```bash
git add .claude/skills/screen-map-validate/SKILL.md
git commit -m "feat(skill): screen-map-validate skill manifest"
```

---

## Task 11: Create `/screens` slash command (Phase 1: validate-only stub)

**Files:**
- Create: `.claude/commands/screens.md`

- [ ] **Step 1: Write the command**

`.claude/commands/screens.md`:

```markdown
# /screens — Screen-Map dispatcher

Dispatch into a screen-map subcommand. Phase 1 supports only `validate`; Phase 2/3 will add `init`, `update`, `review`, `edit`, `render`, `query`.

## Usage

```bash
/screens validate                # validate all screen-maps
/screens validate --strict       # exit non-zero on any error (CI mode)
```

## Implementation

Parse `$ARGUMENTS` for the first token (subcommand) and the rest (forwarded flags).

- If subcommand is `validate`: invoke the `screen-map-validate` skill with the remaining args.
- If subcommand is `init`, `update`, `review`, `edit`, `render`, or `query`: respond
  "This subcommand is part of Phase 2/3 of the screen-map plan and is not yet wired up. See `docs/superpowers/specs/2026-05-07-screen-map-system-design.md` Section 5."
- If subcommand is missing or unknown: print this usage block.
```

- [ ] **Step 2: Try the slash command end-to-end (manual)**

In Claude Code, type `/screens validate`. Expected: skill runs, returns "Screen-map clean: validated 0 screen-maps." (the tree is empty).

- [ ] **Step 3: Commit**

```bash
git add .claude/commands/screens.md
git commit -m "feat(slash): /screens validate dispatcher (phase-1 stub)"
```

---

## Task 12: Pre-commit hook integration

**Files:**
- Modify: `scripts/pre-commit`

- [ ] **Step 1: Inspect the existing hook structure**

```bash
sed -n '1,200p' scripts/pre-commit
```

Note the `# ===== Check N: ... =====` block convention and the `STAGED_FILES` variable. The new check follows the same pattern.

- [ ] **Step 2: Add a new check block**

Open `scripts/pre-commit` and append a new check **before** the auto-version-bump section (the version bump should always come last). The block:

```bash
# =============================================================================
# Check N: Screen-Map validation
# =============================================================================
if echo "$STAGED_FILES" | grep -qE '^docs/screens/.*\.md$|^frontend/packages/screen-map/'; then
    echo -e "${CYAN}Validating screen-maps...${NC}"

    if ! (cd "$ROOT_DIR/frontend" && pnpm --filter @ppt/screen-map cli validate --root "$ROOT_DIR" --strict 2>&1); then
        echo ""
        echo -e "${RED}╔══════════════════════════════════════════════════════════════════╗${NC}"
        echo -e "${RED}║  PRE-COMMIT HOOK FAILED: screen-map validation failed            ║${NC}"
        echo -e "${RED}╠══════════════════════════════════════════════════════════════════╣${NC}"
        echo -e "${RED}║  One or more screen-maps under docs/screens/ are inconsistent.   ║${NC}"
        echo -e "${RED}║                                                                  ║${NC}"
        echo -e "${RED}║  TO FIX: see the errors above. Common causes:                    ║${NC}"
        echo -e "${RED}║    - frontmatter schema violation                                ║${NC}"
        echo -e "${RED}║    - endpoint id missing from @ppt/sitemap                       ║${NC}"
        echo -e "${RED}║    - related screen-map id does not exist                        ║${NC}"
        echo -e "${RED}║                                                                  ║${NC}"
        echo -e "${RED}║  Re-run locally:                                                 ║${NC}"
        echo -e "${YELLOW}║    cd frontend && pnpm --filter @ppt/screen-map cli validate --strict${NC}"
        echo -e "${RED}╚══════════════════════════════════════════════════════════════════╝${NC}"
        echo ""
        FAILED=1
    else
        echo -e "${GREEN}✓ Screen-map validation OK${NC}"
    fi
fi
```

Replace `Check N` with the next available number in your file (count the existing `Check 1` through `Check K`).

- [ ] **Step 3: Reinstall the hook**

```bash
./scripts/install-hooks.sh
```

Expected: prints `✓ Pre-commit hook installed`.

- [ ] **Step 4: Manually trigger the hook on a no-op staged change**

```bash
echo "" >> docs/screens/README.md
git add docs/screens/README.md
git commit -m "chore: trigger screen-map validate"
```

Expected: hook runs, prints `✓ Screen-map validation OK`, commit succeeds.

If you do not want to keep this commit, soft-revert with:

```bash
git reset --soft HEAD~1
git restore --staged docs/screens/README.md
git checkout -- docs/screens/README.md
```

- [ ] **Step 5: Manually trigger a failure**

Create a deliberately broken file:

```bash
cat > docs/screens/ppt/__broken.md <<'EOF'
---
id: bad
name: Bad
product: ppt
implementations: {}
---
EOF
git add docs/screens/ppt/__broken.md
git commit -m "test: broken screen-map (should fail hook)" || true
```

Expected: hook FAILS with "id must match". Cleanup:

```bash
git restore --staged docs/screens/ppt/__broken.md
rm docs/screens/ppt/__broken.md
```

- [ ] **Step 6: Commit the hook change**

```bash
git add scripts/pre-commit
git commit -m "chore(pre-commit): add screen-map validate check"
```

---

## Task 13: CI workflow

**Files:**
- Create: `.github/workflows/screen-map.yml`

- [ ] **Step 1: Write the workflow**

`.github/workflows/screen-map.yml`:

```yaml
name: Screen-Map

on:
  push:
    paths:
      - 'docs/screens/**'
      - 'frontend/packages/screen-map/**'
      - 'frontend/packages/sitemap/**'
      - 'frontend/apps/ppt-web/src/App.tsx'
      - 'frontend/apps/ppt-web/src/routes/**'
      - 'frontend/apps/reality-web/src/app/**'
      - 'frontend/apps/mobile/app/**'
  pull_request:
    paths:
      - 'docs/screens/**'
      - 'frontend/packages/screen-map/**'
      - 'frontend/packages/sitemap/**'
      - 'frontend/apps/ppt-web/src/App.tsx'
      - 'frontend/apps/ppt-web/src/routes/**'
      - 'frontend/apps/reality-web/src/app/**'
      - 'frontend/apps/mobile/app/**'

jobs:
  validate:
    name: Validate screen-maps
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'

      - name: Setup pnpm
        uses: pnpm/action-setup@v2
        with:
          version: 8

      - name: Get pnpm store directory
        shell: bash
        run: echo "STORE_PATH=$(pnpm store path --silent)" >> $GITHUB_ENV

      - name: Cache pnpm
        uses: actions/cache@v4
        with:
          path: ${{ env.STORE_PATH }}
          key: ${{ runner.os }}-pnpm-store-${{ hashFiles('**/pnpm-lock.yaml') }}
          restore-keys: |
            ${{ runner.os }}-pnpm-store-

      - name: Install
        run: cd frontend && pnpm install --frozen-lockfile

      - name: Test screen-map package
        run: cd frontend && pnpm --filter @ppt/screen-map test

      - name: Validate (strict)
        run: cd frontend && pnpm --filter @ppt/screen-map cli validate --root "${{ github.workspace }}" --strict
```

- [ ] **Step 2: Validate the YAML locally**

```bash
yq . .github/workflows/screen-map.yml > /dev/null && echo "OK"
```

If `yq` is not installed, alternatively use Python:

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/screen-map.yml')); print('OK')"
```

Expected: `OK`.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/screen-map.yml
git commit -m "ci(screen-map): validate on PRs touching screens or routes"
```

---

## Task 14: Phase 1 ship checkpoint

**Files:** none modified — verification only.

- [ ] **Step 1: Run the full screen-map test suite**

```bash
cd frontend && pnpm --filter @ppt/screen-map test
```

Expected: ALL pass — schema (5), parse (3), write (2), discover (3), validate (6), cli (2).

- [ ] **Step 2: Run the validator against the live tree**

```bash
cd frontend && pnpm --filter @ppt/screen-map cli validate --root .. --strict
```

Expected: `Validated 0 screen-maps: 0 errors, 0 warnings.` exit 0.

- [ ] **Step 3: Build the package**

```bash
cd frontend && pnpm --filter @ppt/screen-map typecheck
```

Expected: PASS, no `tsc` errors.

- [ ] **Step 4: Confirm the slash command is discoverable**

```bash
ls .claude/commands/screens.md .claude/skills/screen-map-validate/SKILL.md
```

Expected: both files listed.

- [ ] **Step 5: Confirm the pre-commit hook is wired up**

```bash
grep -q "Screen-Map validation" .git/hooks/pre-commit 2>/dev/null && echo "hook OK" || echo "hook NOT installed"
```

Expected: `hook OK`. If `hook NOT installed`, re-run `./scripts/install-hooks.sh`.

- [ ] **Step 6: Run a smoke test by adding a real (minimal) screen**

```bash
cat > docs/screens/ppt/__phase1-smoke.md <<'EOF'
---
id: ppt/phase1-smoke
name: Phase 1 Smoke Test
product: ppt
implementations:
  ppt-web:
    buildStatus: planned
    redesignStatus: not-started
    apiStatus: stub
---

## Functionality Checklist

- [ ] [w] Phase 1 smoke test entry.

## Agent Log

<!-- newest entries on top -->

- 2026-05-07 — phase1: smoke test screen.
EOF

cd frontend && pnpm --filter @ppt/screen-map cli validate --root .. --strict
```

Expected: `Validated 1 screen-maps: 0 errors, 0 warnings.` exit 0.

Cleanup:

```bash
rm docs/screens/ppt/__phase1-smoke.md
```

- [ ] **Step 7: Tag a Phase-1-complete checkpoint commit**

```bash
git commit --allow-empty -m "feat(screen-map): Phase 1 (foundation) complete

Foundation in place:
- @ppt/screen-map package: types, schema, parse, write, discover, validate, CLI
- screen-map-validate skill + /screens slash command (validate stub)
- docs/screens/ skeleton (README, template, product subdirs)
- pre-commit hook + CI workflow

Phase 2 (init, review, edit, DesignSource) and Phase 3 (update, render,
query, agent self-management) tracked as separate plans."
```

- [ ] **Step 8: Push and open a PR**

```bash
git push -u origin "$(git branch --show-current)"
gh pr create --title "Screen-Map Phase 1: foundation" --body "$(cat <<'EOF'
## Summary

- Stand up `@ppt/screen-map` package (types, parse, write, validate, discover, CLI).
- Add `screen-map-validate` skill and `/screens validate` slash command stub.
- Seed `docs/screens/` with README, template, and product subdirs.
- Wire up pre-commit and CI gates.

This is **Phase 1** of the [screen-map system](docs/superpowers/specs/2026-05-07-screen-map-system-design.md). Phase 2 (init, review, edit, DesignSource) and Phase 3 (update, render, query, agent self-management) follow as separate PRs.

## Test plan

- [x] `pnpm --filter @ppt/screen-map test` (all suites green)
- [x] `pnpm --filter @ppt/screen-map cli validate --strict` (clean tree)
- [x] Pre-commit hook fires on `docs/screens/**` changes
- [x] CI workflow validates on PRs touching screens or routes
- [x] `/screens validate` slash command runs end-to-end
EOF
)"
```

Expected: PR opened.

---

## Self-Review

1. **Spec coverage check:**
   - Section 4 (file format) → Tasks 2-5, 9.
   - Section 5.4 (`screen-map-validate` skill) → Task 10.
   - Section 5.7 (`/screens` slash command) → Task 11 (validate-only stub; remaining subcommands explicitly deferred to Phase 2/3).
   - Section 10 (testing & CI) → Task 13 + per-package vitest in Tasks 3-8.
   - Section 3 (layout) → Task 1 (package scaffold) + Task 9 (`docs/screens/`).
   - Sections 5.1-5.3, 5.5, 5.6, 5.7 (full skill set), Section 6 (review server), Section 7 (DesignSource), Section 9 (agent self-management) → explicitly out of Phase 1, deferred and listed in the plan header.

2. **Placeholder scan:** None of "TBD", "TODO", "implement later", "appropriate error handling", or "similar to task N" used.

3. **Type consistency:**
   - `ScreenMap`, `Implementation`, `RelatedScreen`, `DiagramRef` — same names across types.ts, schema.ts, validate.ts, write.ts, parse.ts, cli.ts.
   - `ValidationContext` properties — `knownEndpointIds`, `knownSitemapIds`, `knownScreenIds`, `resolveDiagramRef` — used identically in validate.ts and context.ts and tests.
   - CLI flag names — `--root`, `--strict` — used the same in cli.ts, the skill, the `/screens` command, the pre-commit hook, and the CI workflow.

If you discover during execution that `@ppt/sitemap` exports do not match the names confirmed in Task 8 Step 1 (`pptWebRoutes`, `realityWebRoutes`, `mobileScreens`, `apiServerEndpoints`, `realityServerEndpoints`), update `context.ts` to use the actual export names — the rest of the chain is independent of the specific names.
