/**
 * Playwright configuration for ppt-web (@ppt/web).
 *
 * Delegates entirely to the shared {@link defineE2EConfig} factory from
 * `@ppt/e2e` — base URL, reporters, retries, webServer, and browser projects
 * are all resolved there from env, so there are no per-app magic constants.
 *
 * `app: 'ppt-web'` also seeds the `appName` worker option that drives the
 * `app` fixture. `testDir` points at this app's spec folder (`./e2e/specs`),
 * resolved relative to this config file.
 */

import { defineE2EConfig } from '@ppt/e2e';

// Dev port is the Vite server port from this app's vite.config.ts (3000).
export default defineE2EConfig({ app: 'ppt-web', port: 3000, testDir: './e2e/specs' });
