import { mkdtemp, rm } from 'node:fs/promises';
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
    const c: CandidateScreen[] = [{ id: 'ppt/foo', name: 'Foo', product: 'ppt', source: 'user' }];
    await bulkWriteScreenMaps(c, tmpRoot);
    await expect(bulkWriteScreenMaps(c, tmpRoot)).rejects.toThrow(/already exists/);
    // With force: succeed.
    const written = await bulkWriteScreenMaps(c, tmpRoot, { force: true });
    expect(written).toHaveLength(1);
  });
});
