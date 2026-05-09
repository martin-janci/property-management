import { mkdir, mkdtemp, rm } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { loadScreenContext } from '../src/edit-context.js';
import { bulkWriteScreenMaps } from '../src/init-write.js';

let tmpRoot: string;
beforeEach(async () => {
  tmpRoot = await mkdtemp(path.join(os.tmpdir(), 'edit-ctx-'));
  await mkdir(path.join(tmpRoot, 'docs/screens'), { recursive: true });
  await bulkWriteScreenMaps(
    [
      {
        id: 'ppt/buildings-list',
        name: 'Buildings List',
        product: 'ppt',
        source: 'sitemap',
        sitemapRefs: { 'ppt-web': 'ppt-buildings-list' },
      },
      {
        id: 'ppt/building-detail',
        name: 'Building Detail',
        product: 'ppt',
        source: 'sitemap',
        sitemapRefs: { 'ppt-web': 'ppt-building-detail' },
      },
    ],
    path.join(tmpRoot, 'docs/screens')
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
      loadScreenContext('ppt/nope', { repoRoot: tmpRoot, includePlaywright: false })
    ).rejects.toThrow(/not found/);
  });
});
