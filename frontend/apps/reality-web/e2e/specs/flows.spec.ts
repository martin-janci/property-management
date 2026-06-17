/**
 * Flow-driven specs for reality-web — the core reusable suite.
 *
 * `registerFlowSpecs` walks `@ppt/sitemap`'s `allFlows`, selects those whose
 * `apps` include `reality-web` (flow-search-listings, flow-add-favorite), and
 * drives each through the shared {@link FlowRunner} (specs-as-data: the catalog
 * is the spec, the runner is the only interpreter).
 *
 * Backend gating is automatic: flows needing a live reality-server (api calls /
 * non-anonymous role — flow-add-favorite is a `portal_user` flow with
 * `favorites_add`) are tagged `@requires-backend` and `test.skip()` cleanly
 * unless `E2E_WITH_BACKEND=1`. flow-add-favorite's prerequisite
 * (flow-search-listings) runs first through the same runner.
 */

import { registerFlowSpecs, test } from '@ppt/e2e';
import { APP } from '../pages';

test.describe('reality-web · flows', () => {
  registerFlowSpecs({ app: APP });
});
