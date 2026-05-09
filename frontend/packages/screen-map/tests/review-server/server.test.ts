import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
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

  it('synthesizes the Specific (recent) heading when missing from the body', async () => {
    // Construct a screen WITHOUT the standard ## Notes / ### Specific (recent) heading.
    const screensDir = path.join(tmpRoot, 'docs/screens/ppt');
    await mkdir(screensDir, { recursive: true });
    const filePath = path.join(screensDir, 'custom.md');
    await writeFile(
      filePath,
      [
        '---',
        'id: ppt/custom',
        'name: Custom',
        'product: ppt',
        'implementations:',
        '  ppt-web:',
        '    buildStatus: shipped',
        '    redesignStatus: not-started',
        '    apiStatus: partial',
        '---',
        '',
        '## Functionality Checklist',
        '',
        '- [ ] foo',
        '',
      ].join('\n'),
      'utf8'
    );
    const customScreen = await parseScreenMap(filePath);
    const session = createSession({ screens: [customScreen] });
    const app = await buildServer({ session, onFinish: () => {} });
    const res = await app.request(`/api/screens/ppt/custom/review?session=${session.token}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ decisions: [], generalNote: 'first ever note' }),
    });
    expect(res.status).toBe(200);
    const updated = await readFile(filePath, 'utf8');
    expect(updated).toMatch(/### Specific \(recent\)/);
    expect(updated).toMatch(/first ever note/);
  });

  it('persists per-checklist-item notes into the Agent Log entry', async () => {
    const files = await discoverScreenMaps(path.join(tmpRoot, 'docs/screens'));
    const screens = await Promise.all(files.map((f) => parseScreenMap(f)));
    const session = createSession({ screens });
    const app = await buildServer({ session, onFinish: () => {} });
    const reviewRes = await app.request(`/api/screens/ppt/foo/review?session=${session.token}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        decisions: [
          { itemKey: 'View info', ok: true },
          { itemKey: 'Edit info', ok: false, note: 'missing on mobile' },
        ],
      }),
    });
    expect(reviewRes.status).toBe(200);
    const updated = await readFile(path.join(tmpRoot, 'docs/screens/ppt/foo.md'), 'utf8');
    expect(updated).toMatch(/"missing on mobile" on "Edit info"/);
  });

  it('rejects requests with a non-localhost Host header (DNS rebinding)', async () => {
    const session = createSession({ screens: [] });
    const app = await buildServer({ session, onFinish: () => {} });
    const res = await app.request('/api/session', {
      headers: { Host: 'evil.com' },
    });
    expect(res.status).toBe(403);
  });

  it('serves design frame bytes via /api/designs/:adapter/:frameId', async () => {
    const { ZipAdapter } = await import('../../src/design-source/zip-adapter.js');
    const { fileURLToPath } = await import('node:url');
    const here = path.dirname(fileURLToPath(import.meta.url));
    const fixturePath = path.join(here, '..', 'fixtures', 'designs-2026-q2.zip');
    const adapter = await ZipAdapter.fromFile(fixturePath, '/');
    const session = createSession({ screens: [], designSources: [adapter] });
    const app = await buildServer({ session, onFinish: () => {} });

    // Happy path
    const res = await app.request(
      `/api/designs/zip/building-detail-v3-web?session=${session.token}`
    );
    expect(res.status).toBe(200);
    expect(res.headers.get('Content-Type')).toBe('image/png');
    const buf = new Uint8Array(await res.arrayBuffer());
    expect(buf[0]).toBe(0x89);
    expect(buf[1]).toBe(0x50);

    // Unknown frame
    const r404 = await app.request(`/api/designs/zip/missing?session=${session.token}`);
    expect(r404.status).toBe(404);

    // Unknown adapter
    const rUnknownAdapter = await app.request(
      `/api/designs/figma/whatever?session=${session.token}`
    );
    expect(rUnknownAdapter.status).toBe(404);
  });
});
