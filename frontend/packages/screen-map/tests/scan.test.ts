import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { scanCandidates } from '../src/scan.js';

const here = path.dirname(fileURLToPath(import.meta.url));
const fixturesDir = path.join(here, 'fixtures');

describe('scanCandidates', () => {
  it('returns sitemap routes/screens for the requested product', async () => {
    const candidates = await scanCandidates({
      product: 'ppt',
      repoRoot: '/', // not used for sitemap source
      sources: {
        sitemap: true,
        useCases: false,
        epics: false,
        designSource: undefined,
        userAdd: [],
      },
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
      sources: {
        sitemap: false,
        useCases: true,
        epics: false,
        designSource: undefined,
        userAdd: [],
      },
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
      sources: {
        sitemap: false,
        useCases: false,
        epics: true,
        designSource: undefined,
        userAdd: [],
      },
    });
    const epics = candidates.flatMap((c) => c.epics ?? []);
    // Leading zeros are stripped to match the unpadded frontmatter refs.
    expect(epics).toContain('Epic-1');
    // Letter-suffixed epics (10A/10B/7B convention) round-trip, upper-cased.
    expect(epics).toContain('Epic-10B');
  });

  it('includes user-add entries with source: "user"', async () => {
    const candidates = await scanCandidates({
      product: 'ppt',
      repoRoot: '/',
      sources: {
        sitemap: false,
        useCases: false,
        epics: false,
        designSource: undefined,
        userAdd: ['Faults assignment modal', 'Inventory dashboard'],
      },
    });
    expect(candidates).toHaveLength(2);
    expect(candidates.every((c) => c.source === 'user')).toBe(true);
    expect(candidates[0].name).toBe('Faults assignment modal');
  });

  it('extracts design frames from a DesignSource adapter', async () => {
    const { ZipAdapter } = await import('../src/design-source/zip-adapter.js');
    const fixturePath = path.join(fixturesDir, 'designs-2026-q2.zip');
    const adapter = await ZipAdapter.fromFile(fixturePath, '/');
    const candidates = await scanCandidates({
      product: 'ppt',
      repoRoot: '/',
      sources: {
        sitemap: false,
        useCases: false,
        epics: false,
        designSource: adapter,
        userAdd: [],
      },
    });
    const designCandidates = candidates.filter((c) => c.source === 'design');
    expect(designCandidates).toHaveLength(2);
    expect(designCandidates.map((c) => c.frameId).sort()).toEqual([
      'building-detail-v3-mobile',
      'building-detail-v3-web',
    ]);
  });
});
