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

  // Token gate — applies to every /api/* route.
  app.use('/api/*', async (c, next) => {
    const provided = c.req.query('session');
    if (provided !== opts.session.token) {
      return c.json({ error: 'invalid session token' }, 401);
    }
    await next();
  });

  // Static client (HTML, JS, CSS).
  app.get('/', async (c) => {
    const html = await readFile(path.join(clientDir, 'index.html'), 'utf8');
    return c.html(html.replace('__SESSION_TOKEN__', opts.session.token));
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
