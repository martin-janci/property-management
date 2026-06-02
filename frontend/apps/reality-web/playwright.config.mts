/**
 * Playwright config for reality-web, built on the shared @ppt/e2e factory.
 *
 * The factory resolves base URL, reporters, retries, browser projects and the
 * dev `webServer` from env. We only assert the two app-specific facts here:
 *   - `app: 'reality-web'` selects the sitemap + base URL matrix entry, and
 *   - `appName` (worker option) drives the `app` fixture inside specs.
 *
 * Port: reality-web is pinned to 3001 by the framework (getLocalPort), so all
 * three web apps can run side-by-side. The bare `next dev` script defaults to
 * 3000, so the webServer command is overridden to bind 3001 — keeping the
 * launched server and the resolved baseURL (http://localhost:3001) in sync.
 * Override the target with E2E_BASE_URL / E2E_REALITY_WEB_BASE_URL, or skip the
 * server entirely with E2E_NO_SERVER=1.
 *
 * Why `.mts`: reality-web is a Next.js app with no `"type": "module"`, so
 * Playwright would load a `playwright.config.ts` via CommonJS `require()`. The
 * shared @ppt/sitemap (transitively imported through @ppt/e2e) ships an
 * import-only `exports` map, which a CJS `require()` cannot resolve ("No
 * exports main defined"). Authoring the config as `.mts` forces ESM loading —
 * satisfying sitemap's `import` condition — without touching any shared package
 * or the app's own `package.json` type field. Playwright auto-discovers
 * `playwright.config.mts`, so the `e2e` / `e2e:ui` scripts need no change.
 */

import type { PlaywrightTestConfig } from '@playwright/test';
import { defineE2EConfig, type PptWorkerOptions, shouldStartWebServer } from '@ppt/e2e';

const PORT = 3001;

// `appName` is a framework worker option (PptWorkerOptions), not part of the
// base Playwright `use` type that `defineE2EConfig` overrides accept. The
// factory injects it back into the extended `test` fixture at runtime, so cast
// the merged `use` to the base config's shape here.
const use = { appName: 'reality-web' } satisfies Partial<PptWorkerOptions> as NonNullable<
  PlaywrightTestConfig['use']
>;

export default defineE2EConfig({
  app: 'reality-web',
  port: PORT,
  testDir: './e2e/specs',
  overrides: {
    use,
    // Bind `next dev` to the framework's reality-web port so the launched
    // server matches the resolved baseURL. Honour the same skip conditions as
    // the factory (E2E_BASE_URL / E2E_NO_SERVER).
    webServer: shouldStartWebServer()
      ? {
          command: `pnpm dev --port ${PORT}`,
          url: `http://localhost:${PORT}`,
          reuseExistingServer: !process.env.CI,
          timeout: 120_000,
        }
      : undefined,
  },
});
