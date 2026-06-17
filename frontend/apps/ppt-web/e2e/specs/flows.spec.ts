/**
 * ppt-web user-flow specs — the core reusable suite.
 *
 * Delegates entirely to `registerFlowSpecs({ app: 'ppt-web' })`: every
 * {@link import('@ppt/sitemap').UserFlow} in the committed `@ppt/sitemap`
 * `allFlows` catalog whose `apps` includes `ppt-web` is interpreted as an
 * executable E2E by the framework's `FlowRunner` (specs-as-data — the catalog
 * is the spec, the runner is the only interpreter).
 *
 * Backend-gated flows (login-basic, logout, document-upload, document-view,
 * file-dispute) self-annotate `@requires-backend` and `test.skip()` cleanly
 * unless `E2E_WITH_BACKEND=1`, so a code-only run is honest and never
 * green-by-stub. Prerequisite chains (e.g. login before logout) compose
 * through the same runner without duplicating steps.
 */

import { registerFlowSpecs, test } from '@ppt/e2e';
import { PPT_WEB_APP } from '../pages';

test.describe('ppt-web · user flows (sitemap-driven)', () => {
  registerFlowSpecs({ app: PPT_WEB_APP });
});
