import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
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
    const result = await bulkWriteScreenMaps(concepts, tmpRoot);
    expect(result.written).toHaveLength(2);
    expect(result.skipped).toHaveLength(0);

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
    const c: CandidateScreen[] = [{ id: 'ppt/foo', name: 'Foo', product: 'ppt', source: 'user' }];
    await bulkWriteScreenMaps(c, tmpRoot);
    await expect(bulkWriteScreenMaps(c, tmpRoot)).rejects.toThrow(/already exists/);
    // With force on a template-shaped file: overwrite succeeds.
    const result = await bulkWriteScreenMaps(c, tmpRoot, { force: true });
    expect(result.written).toHaveLength(1);
    expect(result.skipped).toHaveLength(0);
  });

  it('rejects a grouping-mutated invalid id at the write boundary (no file written)', async () => {
    // Interactive grouping (grouping.ts rename/merge) can set an arbitrary
    // user-supplied id after `scanCandidates` runs its IdSchema guard, so the
    // write boundary must re-validate. These ids all have a non-empty slug —
    // they pass the pre-existing `no slug` check — but fail IdSchema / the
    // product-prefix superRefine, and must never be written (#2406).
    const cases: CandidateScreen[] = [
      // whitespace + uppercase in slug (rename to "Foo Bar")
      { id: 'ppt/Foo Bar', name: 'Foo Bar', product: 'ppt', source: 'user' },
      // uppercase slug
      { id: 'ppt/UPPER', name: 'Upper', product: 'ppt', source: 'user' },
      // product-prefix mismatch: id says reality, product says ppt
      { id: 'reality/x', name: 'X', product: 'ppt', source: 'user' },
    ];
    for (const c of cases) {
      await expect(bulkWriteScreenMaps([c], tmpRoot)).rejects.toThrow(
        /invalid screen-map frontmatter/
      );
      // The guard must throw before writeFile — nothing lands on disk.
      const slug = c.id.split('/')[1];
      const leaked = path.join(tmpRoot, c.product, `${slug}.md`);
      await expect(readFile(leaked, 'utf8')).rejects.toThrow();
    }
  });

  it('preserves user-edited screen-maps even when force=true', async () => {
    const c: CandidateScreen[] = [{ id: 'ppt/foo', name: 'Foo', product: 'ppt', source: 'user' }];
    await bulkWriteScreenMaps(c, tmpRoot);
    const file = path.join(tmpRoot, 'ppt/foo.md');
    // Simulate a user editing the file: remove the `init: created from scan` marker.
    const original = await readFile(file, 'utf8');
    const edited = original.replace('— init: created from scan', '— manual: human-edited');
    await writeFile(file, edited, 'utf8');
    const result = await bulkWriteScreenMaps(c, tmpRoot, { force: true });
    expect(result.written).toHaveLength(0);
    expect(result.skipped).toEqual([file]);
    // File should remain user-edited.
    const after = await readFile(file, 'utf8');
    expect(after).toContain('manual: human-edited');
  });
});
