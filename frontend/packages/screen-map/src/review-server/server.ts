import { timingSafeEqual } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { Hono } from 'hono';
import type { ReviewSession } from './session.js';

export interface ServerOptions {
  session: ReviewSession;
  /** Hook for graceful shutdown — typically calls server.close(). */
  onFinish: () => void;
}

export async function buildServer(opts: ServerOptions): Promise<Hono> {
  const app = new Hono();
  const here = path.dirname(fileURLToPath(import.meta.url));
  const clientDir = path.join(here, 'client');

  // Host header validation — DNS rebinding mitigation. We only reject when a
  // Host header is present AND points anywhere other than localhost. Browsers
  // always include a Host header on real network requests, so a real DNS-
  // rebinding attempt is caught here. An ABSENT Host is treated as in-process
  // (e.g. unit tests via app.request()) and allowed through.
  app.use('*', async (c, next) => {
    const host = c.req.header('Host');
    if (host && !host.startsWith('127.0.0.1:') && !host.startsWith('localhost:')) {
      return c.text('forbidden host', 403);
    }
    await next();
  });

  // Token gate — applies to every /api/* route. Timing-safe comparison.
  app.use('/api/*', async (c, next) => {
    const provided = c.req.query('session') ?? '';
    const expected = opts.session.token;
    const a = Buffer.from(provided, 'hex');
    const b = Buffer.from(expected, 'hex');
    if (a.length !== b.length || !timingSafeEqual(a, b)) {
      return c.json({ error: 'invalid session token' }, 401);
    }
    await next();
  });

  // Static client (HTML, JS, CSS).
  app.get('/', async (c) => {
    const html = await readFile(path.join(clientDir, 'index.html'), 'utf8');
    // JSON.stringify safely escapes the token for JS string interpolation,
    // even if the token's charset changes in the future.
    return c.html(html.replace('"__SESSION_TOKEN__"', JSON.stringify(opts.session.token)));
  });
  app.get('/styles.css', async (c) => {
    const css = await readFile(path.join(clientDir, 'styles.css'), 'utf8');
    return c.body(css, 200, { 'Content-Type': 'text/css' });
  });
  app.get('/app.tsx', async (c) => {
    const js = await readFile(path.join(clientDir, 'app.tsx'), 'utf8');
    return c.body(js, 200, { 'Content-Type': 'application/javascript' });
  });

  // API routes.
  const { attachApi } = await import('./api.js');
  attachApi(app, opts);

  return app;
}
