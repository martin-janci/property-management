import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { describe, expect, it } from 'vitest';
import { buildValidationContext } from '../src/context.js';

async function withTmpRepo(fn: (root: string) => Promise<void>): Promise<void> {
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
      await writeFile(path.join(root, 'docs/seq.md'), '## Časový plán\n\nbody.\n');
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

  it('exposes knownUseCases, knownEpics, knownComponents from extract-known', async () => {
    await withTmpRepo(async (root) => {
      await mkdir(path.join(root, 'docs'), { recursive: true });
      await writeFile(path.join(root, 'docs/use-cases.md'), '## UC-12 Foo\n');
      await mkdir(path.join(root, '_bmad-output'), { recursive: true });
      await writeFile(path.join(root, '_bmad-output/epics.md'), '#### Epic 1: Foo\n');
      await mkdir(path.join(root, 'frontend/packages/ui-kit/src'), { recursive: true });
      await writeFile(
        path.join(root, 'frontend/packages/ui-kit/src/index.ts'),
        'export { BuildingHeader } from "./x.js";\n'
      );
      const ctx = await buildValidationContext({ repoRoot: root });
      expect(ctx.knownUseCases?.has('UC-12')).toBe(true);
      expect(ctx.knownEpics?.has('Epic-1')).toBe(true);
      expect(ctx.knownComponents?.has('BuildingHeader')).toBe(true);
    });
  });
});
