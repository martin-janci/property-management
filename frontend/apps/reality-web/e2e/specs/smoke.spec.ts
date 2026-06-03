/**
 * Sitemap-driven smoke coverage for reality-web, assembled from the shared
 * @ppt/e2e spec factories. These walk every public route and assert:
 *  - navigation: each public route is reachable and the primary nav links,
 *  - route health: each content route shows a `main` landmark + heading and
 *    logs no console errors (SSO callback routes are excluded by the factory),
 *  - accessibility: an axe-core scan reports serious/critical violations
 *    (report-only unless E2E_A11Y_STRICT is set).
 *
 * The auth spec factory is intentionally NOT wired here: reality-web is a
 * public Next.js portal whose primary sign-in is the SSO callback, and its
 * email/password `/auth/login` form is not modelled in @ppt/sitemap — so the
 * generic, sitemap-resolved auth factory has no login surface to drive
 * (pointing it at `reality-auth-callback` asserted form fields against a page
 * that renders only a callback spinner). The callback's own behaviour is
 * covered by the flow specs, not by the email/password auth factory.
 *
 * Everything is sourced from @ppt/sitemap, so the suite tracks the route
 * catalog automatically as routes are added or removed.
 */

import {
  registerA11ySpecs,
  registerNavigationSpecs,
  registerRouteHealthSpecs,
  test,
} from '@ppt/e2e';
import { APP } from '../pages';

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
