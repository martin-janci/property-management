import { exec } from 'node:child_process';
import { serve } from '@hono/node-server';
import { discoverScreenMaps } from '../discover.js';
import { parseScreenMap } from '../parse.js';
import type { Product, ScreenMapFrontmatter } from '../types.js';
import { buildServer } from './server.js';
import { createSession } from './session.js';

export interface StartOptions {
  repoRoot: string;
  product?: Product;
  filter?: (frontmatter: ScreenMapFrontmatter) => boolean;
  preview?: 'local' | 'staging' | 'design';
  /** Starting port to try. Increments until a free port is found. */
  startPort?: number;
}

export interface StartResult {
  port: number;
  url: string;
  /** Resolves when the server has been shut down (via SIGINT or POST /api/session/finish). */
  finished: Promise<void>;
  /** Imperatively trigger shutdown. */
  shutdown: () => Promise<void>;
}

export async function startReviewServer(opts: StartOptions): Promise<StartResult> {
  const screensDir = `${opts.repoRoot}/docs/screens`;
  const files = await discoverScreenMaps(screensDir);
  let screens = await Promise.all(files.map((f) => parseScreenMap(f)));
  if (opts.product) {
    screens = screens.filter((s) => s.frontmatter.product === opts.product);
  }
  if (opts.filter) {
    screens = screens.filter((s) => opts.filter!(s.frontmatter));
  }
  const session = createSession({ screens, defaultPreview: opts.preview });

  let finishResolve!: () => void;
  const finished = new Promise<void>((resolve) => {
    finishResolve = resolve;
  });

  let serverHandle: { close: (cb?: () => void) => void } | null = null;
  const onFinish = () => {
    serverHandle?.close(() => finishResolve());
  };
  const app = await buildServer({ session, onFinish });

  const port = await findFreePort(opts.startPort ?? 5179);
  serverHandle = serve({ fetch: app.fetch, port });

  const url = `http://127.0.0.1:${port}/?session=${session.token}`;
  openBrowser(url);

  const shutdown = (): Promise<void> => {
    onFinish();
    return finished;
  };

  process.once('SIGINT', () => {
    shutdown();
  });

  return { port, url, finished, shutdown };
}

async function findFreePort(start: number): Promise<number> {
  for (let p = start; p < start + 100; p++) {
    const ok = await new Promise<boolean>((resolve) => {
      import('node:net').then(({ createServer }) => {
        const tester = createServer();
        tester.once('error', () => resolve(false));
        tester.once('listening', () => tester.close(() => resolve(true)));
        tester.listen(p, '127.0.0.1');
      });
    });
    if (ok) return p;
  }
  throw new Error(`no free port between ${start} and ${start + 100}`);
}

function openBrowser(url: string): void {
  const platform = process.platform;
  const cmd =
    platform === 'darwin'
      ? `open "${url}"`
      : platform === 'win32'
        ? `start "" "${url}"`
        : `xdg-open "${url}"`;
  exec(cmd, () => {});
}
