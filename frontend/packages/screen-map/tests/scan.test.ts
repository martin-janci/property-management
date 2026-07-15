import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { scanCandidates } from '../src/scan.js';
import { IdSchema } from '../src/schema.js';

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

  it('synthesizes a schema-valid lower-case id for a letter-suffixed epic', async () => {
    const epicsDir = await mkdtemp(path.join(os.tmpdir(), 'scan-epic-id-'));
    try {
      await writeFile(path.join(epicsDir, 'EPIC-010B-platform-admin.md'), '');
      const candidates = await scanCandidates({
        product: 'ppt',
        repoRoot: '/',
        epicsDir,
        sources: {
          sitemap: false,
          useCases: false,
          epics: true,
          designSource: undefined,
          userAdd: [],
        },
      });
      const epic = candidates.find((c) => c.source === 'epics');
      expect(epic).toBeDefined();
      // Human-facing fields keep the upper-cased suffix...
      expect(epic?.name).toBe('Epic-10B');
      expect(epic?.epics).toEqual(['Epic-10B']);
      // ...but the id slug is lower-cased so it satisfies the frontmatter schema.
      expect(epic?.id).toBe('ppt/epic-10b');
      // Guard against drift between the synthesized id and the schema it must
      // pass through in `screens init` (IdSchema is the id regex in schema.ts).
      expect(IdSchema.safeParse(epic?.id).success).toBe(true);
    } finally {
      await rm(epicsDir, { recursive: true, force: true });
    }
  });

  it('rejects malformed epic filenames instead of mis-binning them', async () => {
    const epicsDir = await mkdtemp(path.join(os.tmpdir(), 'scan-epic-neg-'));
    try {
      await mkdir(epicsDir, { recursive: true });
      // Missing hyphen typo — must NOT become a garbage `Epic-10BETA`.
      await writeFile(path.join(epicsDir, 'EPIC-10beta.md'), '');
      // Multi-char malformed suffix — must NOT silently truncate to `Epic-7A`.
      await writeFile(path.join(epicsDir, 'EPIC-7A2-x.md'), '');
      const candidates = await scanCandidates({
        product: 'ppt',
        repoRoot: '/',
        epicsDir,
        sources: {
          sitemap: false,
          useCases: false,
          epics: true,
          designSource: undefined,
          userAdd: [],
        },
      });
      const epics = candidates.flatMap((c) => c.epics ?? []);
      expect(epics).not.toContain('Epic-10BETA');
      expect(epics).not.toContain('Epic-7A');
      // Neither malformed name yields any candidate at all.
      expect(candidates.filter((c) => c.source === 'epics')).toHaveLength(0);
    } finally {
      await rm(epicsDir, { recursive: true, force: true });
    }
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
