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

  it('extracts Epic IDs from _bmad-output/epics.md headings', async () => {
    await mkdir(path.join(tmpRoot, '_bmad-output'), { recursive: true });
    await writeFile(
      path.join(tmpRoot, '_bmad-output/epics.md'),
      [
        '## Epic List',
        '#### Epic 1: User Authentication & Sessions',
        '#### Epic 12: Meter Readings & Utilities',
        '## Epic 1: User Authentication & Sessions',
        '',
      ].join('\n')
    );
    const ctx = await extractKnownContexts(tmpRoot);
    // Heading epic ids map to the unpadded frontmatter form (`Epic-1`),
    // matching how `epics:` refs are written in screen-map frontmatter.
    expect(ctx.knownEpics.has('Epic-1')).toBe(true);
    expect(ctx.knownEpics.has('Epic-12')).toBe(true);
    // The `## Epic List` section header must NOT be read as an epic.
    expect(ctx.knownEpics.has('Epic-List')).toBe(false);
  });

  it('preserves and upper-cases letter-suffixed epics (10A/10B/7B convention)', async () => {
    await mkdir(path.join(tmpRoot, '_bmad-output'), { recursive: true });
    await writeFile(
      path.join(tmpRoot, '_bmad-output/epics.md'),
      [
        // Letter-suffixed epic — must round-trip to the canonical `Epic-10A`.
        '#### Epic 10A: OAuth Provider Foundation',
        // Plain numeric epic — must still collapse to `Epic-10`, not `Epic-010`.
        '## Epic 10B: Platform Administration',
        // Lowercase suffix normalizes so `Epic-7b` and `Epic-7B` don't diverge.
        '### Epic 7b: Advanced Document Features',
        // A trailing `-SSO`/`-Complete` heading qualifier is dropped.
        '#### Epic 2B-Complete: WebSocket & Mobile Notification Infrastructure',
        '',
      ].join('\n')
    );
    const ctx = await extractKnownContexts(tmpRoot);
    expect(ctx.knownEpics.has('Epic-10A')).toBe(true);
    expect(ctx.knownEpics.has('Epic-10B')).toBe(true);
    expect(ctx.knownEpics.has('Epic-7B')).toBe(true);
    // The `-Complete` qualifier collapses onto the canonical `Epic-2B`.
    expect(ctx.knownEpics.has('Epic-2B')).toBe(true);
    expect(ctx.knownEpics.has('Epic-2B-COMPLETE')).toBe(false);
  });

  it('only reads heading lines, not prose mentions or malformed ids', async () => {
    await mkdir(path.join(tmpRoot, '_bmad-output'), { recursive: true });
    await writeFile(
      path.join(tmpRoot, '_bmad-output/epics.md'),
      [
        '#### Epic 11: Financial Management & Payments',
        // Prose cross-reference — must NOT be mined as a catalog entry.
        'This story depends on Epic 99 which lives elsewhere.',
        // Missing-hyphen typo in a heading — must NOT become `Epic-10BETA`.
        '#### Epic 10beta: typo',
        '',
      ].join('\n')
    );
    const ctx = await extractKnownContexts(tmpRoot);
    expect(ctx.knownEpics.has('Epic-11')).toBe(true);
    expect(ctx.knownEpics.has('Epic-99')).toBe(false);
    expect(ctx.knownEpics.has('Epic-10BETA')).toBe(false);
    // Only the single well-formed heading contributes.
    expect(ctx.knownEpics.size).toBe(1);
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
