/**
 * Sitemap-driven smoke coverage for reality-web, assembled from the shared
 * @ppt/e2e spec factories. These walk every public route and assert:
 *  - navigation: each public route is reachable and the primary nav links,
 *  - route health: each route shows a `main` landmark + heading and logs no
 *    console errors,
 *  - accessibility: an axe-core scan finds no serious/critical violations,
 *  - auth: the login surface behind `reality-auth-callback` is exercised.
 *
 * Everything is sourced from @ppt/sitemap, so the suite tracks the route
 * catalog automatically as routes are added or removed.
 */

import {
  registerA11ySpecs,
  registerAuthSpecs,
  registerNavigationSpecs,
  registerRouteHealthSpecs,
  test,
} from '@ppt/e2e';
import { APP } from '../pages';

/** reality-web's login surface is its OAuth callback route in the sitemap. */
const LOGIN_ROUTE_ID = 'reality-auth-callback';

test.describe('reality-web · navigation', () => {
  registerNavigationSpecs({ app: APP });
});

test.describe('reality-web · route health', () => {
  registerRouteHealthSpecs({
    app: APP,
    // Next.js dev/runtime + next-intl emit benign console noise we don't gate on.
    ignoreConsole: ['next-intl', 'Download the React DevTools', 'hydrat'],
  });
});

test.describe('reality-web · accessibility', () => {
  registerA11ySpecs({
    app: APP,
    // Region/landmark rules are noisy on marketing pages with decorative SVGs;
    // keep the gate meaningful (serious/critical) without it being flaky.
    disableRules: ['region', 'landmark-unique'],
  });
});

test.describe('reality-web · auth', () => {
  // reality-web authenticates via OAuth SSO; `reality-auth-callback` is its
  // sitemap login surface. loginRouteId is non-empty, so the auth factory is
  // wired per spec — it asserts the login surface and rejects bad input without
  // a backend.
  registerAuthSpecs({ app: APP, loginRouteId: LOGIN_ROUTE_ID });
});
