import { writeFile } from 'node:fs/promises';
import type { Hono } from 'hono';
import type { ScreenMap } from '../types.js';
import { writeScreenMapString } from '../write.js';
import type { ReviewSession } from './session.js';

interface ReviewBody {
  decisions?: Array<{ itemKey: string; ok: boolean; note?: string }>;
  generalNote?: string;
}

export function attachApi(
  app: Hono,
  opts: {
    session: ReviewSession;
    onFinish: () => void;
  }
): void {
  app.get('/api/session', (c) => {
    const summaries = opts.session.screens.map((s) => ({
      id: s.frontmatter.id,
      name: s.frontmatter.name,
      product: s.frontmatter.product,
    }));
    return c.json({
      product: opts.session.screens[0]?.frontmatter.product ?? null,
      screens: summaries,
      currentIdx: opts.session.currentIdx,
      sessionToken: opts.session.token,
      defaultPreview: opts.session.defaultPreview,
    });
  });

  app.get('/api/screens/:id', (c) => {
    const id = c.req.param('id');
    const screen = opts.session.screens.find((s) => s.frontmatter.id === id);
    if (!screen) return c.json({ error: 'not found' }, 404);
    return c.json({
      frontmatter: screen.frontmatter,
      body: screen.body,
      previewUrls: buildPreviewUrls(screen),
    });
  });

  app.post('/api/screens/:id/review', async (c) => {
    const id = c.req.param('id');
    const idx = opts.session.screens.findIndex((s) => s.frontmatter.id === id);
    if (idx < 0) return c.json({ error: 'not found' }, 404);
    const screen = opts.session.screens[idx];
    const body = (await c.req.json()) as ReviewBody;

    // Mutate body: append Agent Log entry; optionally append general note.
    const today = new Date().toISOString().slice(0, 10);
    const numOk = (body.decisions ?? []).filter((d) => d.ok).length;
    const numNotes = (body.decisions ?? []).filter((d) => d.note).length;
    const summary = `${today} — review: ${numOk} OK, ${numNotes} note${numNotes === 1 ? '' : 's'}`;
    const newBody = appendAgentLog(screen.body, `- ${summary}`);
    const finalBody = body.generalNote
      ? appendSpecificNote(newBody, today, body.generalNote)
      : newBody;
    // Update lastReview only.
    screen.frontmatter.lastReview = today;
    screen.body = finalBody;
    const serialized = writeScreenMapString(screen);
    await writeFile(screen.filePath, serialized, 'utf8');

    const next = opts.session.screens[idx + 1];
    opts.session.currentIdx = idx + 1;
    return c.json(next ? { nextScreenId: next.frontmatter.id } : { done: true });
  });

  app.post('/api/session/finish', (c) => {
    setTimeout(opts.onFinish, 100); // let response flush
    return c.json({ ok: true });
  });

  app.get('/api/designs/:adapter/:frameId', async (c) => {
    const adapterName = c.req.param('adapter');
    const frameId = c.req.param('frameId');
    const adapter = opts.session.designSources.get(adapterName);
    if (!adapter || !adapter.readBytes) {
      return c.json({ error: 'unknown adapter or no readBytes support' }, 404);
    }
    const bytes = await adapter.readBytes(frameId);
    if (!bytes) return c.json({ error: 'frame not found' }, 404);
    // Re-wrap into a fresh Uint8Array<ArrayBuffer> to satisfy Hono's strict body type.
    const buf = new Uint8Array(bytes);
    return c.body(buf, 200, { 'Content-Type': 'image/png' });
  });
}

function buildPreviewUrls(screen: ScreenMap): { local: string | null; staging: string | null } {
  const impl = screen.frontmatter.implementations as Record<string, { route?: string }>;
  const route = impl['ppt-web']?.route ?? impl['reality-web']?.route ?? null;
  if (!route) return { local: null, staging: null };
  const local =
    screen.frontmatter.product === 'ppt'
      ? `http://localhost:5173${route}`
      : `http://localhost:3000${route}`;
  const stagingHost = screen.frontmatter.product === 'ppt' ? 'ppt.rlt.sk' : 'www.rlt.sk';
  const staging = `https://${stagingHost}${route}`;
  return { local, staging };
}

function appendAgentLog(body: string, line: string): string {
  const idx = body.indexOf('## Agent Log');
  if (idx < 0) return `${body}\n## Agent Log\n\n${line}\n`;
  // Insert under the heading + comment, before any other entries.
  const before = body.slice(0, idx);
  const after = body.slice(idx);
  const lines = after.split(/\r?\n/);
  const insertIdx = lines.findIndex((l, i) => i > 0 && (l.startsWith('- ') || (l === '' && i > 2)));
  const insertAt = insertIdx > 0 ? insertIdx : lines.length;
  lines.splice(insertAt, 0, line);
  return before + lines.join('\n');
}

function appendSpecificNote(body: string, date: string, note: string): string {
  const heading = '### Specific (recent)';
  const idx = body.indexOf(heading);
  if (idx < 0) return body;
  const before = body.slice(0, idx + heading.length);
  const after = body.slice(idx + heading.length);
  return `${before}\n\n- ${date}: ${note}${after}`;
}
