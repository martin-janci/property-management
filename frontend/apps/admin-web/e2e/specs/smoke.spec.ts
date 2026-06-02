/**
 * admin-web framework-factory smoke specs.
 *
 * Wires the four reusable `@ppt/e2e` spec factories:
 *
 *   - navigation / route-health / a11y — sitemap-driven. admin-web routes are
 *     NOT in `@ppt/sitemap` (it only ships ppt-web / reality-web route data),
 *     so each of these self-skips with a clear message. They are invoked here
 *     so the suite lights up automatically the moment admin-web routes are
 *     added to the sitemap — zero spec changes required.
 *   - auth — fully active. admin-web has no sitemap login route, so we pass the
 *     explicit `loginPath` ('/login', from ADMIN_ROUTES). These run without a
 *     backend: the login form is a static SPA route.
 */

import {
  registerA11ySpecs,
  registerAuthSpecs,
  registerNavigationSpecs,
  registerRouteHealthSpecs,
  test,
} from '@ppt/e2e';
import { ADMIN_ROUTES } from '../pages';

const app = 'admin-web';

test.describe('admin-web navigation', () => {
  registerNavigationSpecs({ app });
});

test.describe('admin-web route health', () => {
  registerRouteHealthSpecs({ app });
});

test.describe('admin-web accessibility', () => {
  registerA11ySpecs({ app });
});

test.describe('admin-web auth', () => {
  // loginRouteId is 'admin-login' in the app design, but that id is not in the
  // sitemap; the explicit loginPath drives the LoginPage instead.
  registerAuthSpecs({ app, loginRouteId: 'admin-login', loginPath: ADMIN_ROUTES.login });
});
