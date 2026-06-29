/**
 * Playwright config for accounting-web (@ppt/accounting-web).
 *
 * Delegates everything (base URL, reporters, retries, webServer, browser
 * projects) to the shared `@ppt/e2e` factory.
 *
 * Port: 3002 (pinned in `dev` script so all apps can run side-by-side).
 */

import { defineE2EConfig } from '@ppt/e2e';

export default defineE2EConfig({ app: 'accounting-web', port: 3002, testDir: './e2e/specs' });
