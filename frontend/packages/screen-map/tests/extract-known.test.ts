import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
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
      '## UC-12 Foo\n- UC-12.1 detail\n## UC-13 Bar\n'
    );
    const ctx = await extractKnownContexts(tmpRoot);
    expect(ctx.knownUseCases.has('UC-12')).toBe(true);
    expect(ctx.knownUseCases.has('UC-12.1')).toBe(true);
    expect(ctx.knownUseCases.has('UC-13')).toBe(true);
  });

  it('extracts Epic IDs from docs/epics/*.md filenames (leading zeros stripped)', async () => {
    await mkdir(path.join(tmpRoot, 'docs/epics'), { recursive: true });
    await writeFile(path.join(tmpRoot, 'docs/epics/EPIC-001-foo.md'), '');
    await writeFile(path.join(tmpRoot, 'docs/epics/EPIC-002-bar.md'), '');
    const ctx = await extractKnownContexts(tmpRoot);
    // Numeric prefixes normalize to the unpadded frontmatter form (`Epic-1`),
    // matching how `epics:` refs are written in screen-map frontmatter.
    expect(ctx.knownEpics.has('Epic-1')).toBe(true);
    expect(ctx.knownEpics.has('Epic-2')).toBe(true);
  });

  it('preserves and upper-cases letter-suffixed epics (10A/10B/7B convention)', async () => {
    await mkdir(path.join(tmpRoot, 'docs/epics'), { recursive: true });
    // Letter-suffixed epic — must round-trip to the canonical `Epic-10A`.
    await writeFile(path.join(tmpRoot, 'docs/epics/EPIC-010A-oauth.md'), '');
    // Plain numeric epic — must still collapse to `Epic-10`, not `Epic-010`.
    await writeFile(path.join(tmpRoot, 'docs/epics/EPIC-010-platform.md'), '');
    // Lowercase suffix normalizes so `Epic-7b` and `Epic-7B` don't diverge.
    await writeFile(path.join(tmpRoot, 'docs/epics/EPIC-007b-docs.md'), '');
    const ctx = await extractKnownContexts(tmpRoot);
    expect(ctx.knownEpics.has('Epic-10A')).toBe(true);
    expect(ctx.knownEpics.has('Epic-10')).toBe(true);
    expect(ctx.knownEpics.has('Epic-7B')).toBe(true);
    // The letter-suffixed epic must NOT collapse to the bare numeric prefix.
    expect(ctx.knownEpics.has('Epic-010A')).toBe(false);
  });

  it('extracts component names from frontend/packages/ui-kit exports', async () => {
    await mkdir(path.join(tmpRoot, 'frontend/packages/ui-kit/src'), { recursive: true });
    await writeFile(
      path.join(tmpRoot, 'frontend/packages/ui-kit/src/index.ts'),
      `export { BuildingHeader } from './BuildingHeader.js';
export { UnitsTable } from './UnitsTable.js';
export type { UnitsTableProps } from './UnitsTable.js';
export const StatusBadge = (props: any) => null;
`
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
