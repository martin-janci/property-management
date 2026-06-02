/**
 * Playwright config for admin-web (Super-admin Control Plane).
 *
 * Delegates everything (base URL, reporters, retries, webServer, browser
 * projects) to the shared `@ppt/e2e` factory. The only app-specific bits are:
 *   - `app: 'admin-web'`            — drives env/baseURL resolution.
 *   - `port: 3100`                  — admin-web's real Vite dev port (see
 *                                     apps/admin-web/vite.config.ts).
 *   - `use.appName: 'admin-web'`    — the worker option that feeds the `app`
 *                                     fixture so page objects resolve the right
 *                                     env. Default would otherwise be 'ppt-web'.
 *
 * Specs live under ./e2e/specs (the `testDir` below).
 */

import { defineE2EConfig } from '@ppt/e2e';

export default defineE2EConfig({
  app: 'admin-web',
  port: 3100,
  testDir: './e2e/specs',
  overrides: {
    use: { appName: 'admin-web' },
  },
});
