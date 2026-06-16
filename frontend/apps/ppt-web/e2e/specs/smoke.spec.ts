/**
 * ppt-web smoke suite — sitemap-driven coverage that stays in lockstep with the
 * `@ppt/sitemap` route catalog, so no per-route boilerplate is maintained here.
 *
 *  - `registerNavigationSpecs`  — every navigable public route is reachable.
 *  - `registerRouteHealthSpecs` — each public route renders a `main` landmark +
 *    heading with no unexpected console errors.
 *  - `registerA11ySpecs`        — axe scan per public route.
 *  - `registerAuthSpecs`        — the login form surface (ppt-login).
 *
 * App-specific shell behaviour (named nav clicks, the skip link, language
 * switcher, OAuth callback, page-object page assertions) lives in the sibling
 * page-object specs; this file is the generic, factory-driven floor.
 */

import {
  registerA11ySpecs,
  registerAuthSpecs,
  registerNavigationSpecs,
  registerRouteHealthSpecs,
  test,
} from '@ppt/e2e';
import { PPT_LOGIN_ROUTE_ID, PPT_WEB_APP } from '../pages';

test.describe('ppt-web · navigation (sitemap-driven)', () => {
  registerNavigationSpecs({ app: PPT_WEB_APP });
});

test.describe('ppt-web · route health (sitemap-driven)', () => {
  registerRouteHealthSpecs({ app: PPT_WEB_APP });
});

test.describe('ppt-web · accessibility (sitemap-driven)', () => {
  registerA11ySpecs({ app: PPT_WEB_APP });
});

test.describe('ppt-web · auth form (sitemap-driven)', () => {
  registerAuthSpecs({ app: PPT_WEB_APP, loginRouteId: PPT_LOGIN_ROUTE_ID });
});
