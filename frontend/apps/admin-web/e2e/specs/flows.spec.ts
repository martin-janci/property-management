/**
 * admin-web sitemap-flow specs.
 *
 * `registerFlowSpecs` interprets every `@ppt/sitemap` UserFlow whose `apps`
 * array includes this app and runs it through the framework's `FlowRunner`
 * (specs-as-data — the runner is the only interpreter). No flow currently
 * targets `admin-web`, so the factory emits a single explicit
 * "no flows target admin-web" `test.skip`. The moment a flow lists `admin-web`
 * in its `apps`, it lights up here with zero spec changes.
 */

import { registerFlowSpecs, test } from '@ppt/e2e';

test.describe('admin-web flows', () => {
  registerFlowSpecs({ app: 'admin-web' });
});
