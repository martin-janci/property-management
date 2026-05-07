import { mkdir, mkdtemp, readFile, rm } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { discoverScreenMaps } from '../../src/discover.js';
import { bulkWriteScreenMaps } from '../../src/init-write.js';
import { parseScreenMap } from '../../src/parse.js';
import { buildServer } from '../../src/review-server/server.js';
import { createSession } from '../../src/review-server/session.js';

let tmpRoot: string;
beforeEach(async () => {
  tmpRoot = await mkdtemp(path.join(os.tmpdir(), 'review-srv-'));
  await mkdir(path.join(tmpRoot, 'docs/screens'), { recursive: true });
  await bulkWriteScreenMaps(
    [
      { id: 'ppt/foo', name: 'Foo', product: 'ppt', source: 'sitemap' },
      { id: 'ppt/bar', name: 'Bar', product: 'ppt', source: 'sitemap' },
    ],
    path.join(tmpRoot, 'docs/screens')
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

    const screenRes = await app.request(`/api/screens/ppt/bar?session=${session.token}`);
    expect(screenRes.status).toBe(200);

    const reviewRes = await app.request(`/api/screens/ppt/bar/review?session=${session.token}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        decisions: [
          { itemKey: 'view-info', ok: true },
          { itemKey: 'edit-info', ok: false, note: 'missing on mobile' },
        ],
        generalNote: 'header looks good',
      }),
    });
    expect(reviewRes.status).toBe(200);
    const reviewJson = (await reviewRes.json()) as { nextScreenId?: string; done?: boolean };
    expect(reviewJson.nextScreenId).toBe('ppt/foo');

    // The markdown file should now contain an Agent Log entry + a Specific note.
    const updated = await readFile(path.join(tmpRoot, 'docs/screens/ppt/bar.md'), 'utf8');
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
