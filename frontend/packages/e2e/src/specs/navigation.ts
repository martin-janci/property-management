/**
 * Navigation spec factory: every public route is reachable, and the primary
 * nav links to the major ones. Sourced entirely from the sitemap so it works
 * for any sitemap-backed app.
 */

import { expect, test } from '../fixtures';
import { SitemapPage } from '../pages/SitemapPage';
import { isSitemapApp, navigablePublicRoutes } from './shared';

export interface RegisterNavigationOptions {
  readonly app: string;
}

/** Register navigation tests. Call inside a `test.describe`. */
export function registerNavigationSpecs({ app }: RegisterNavigationOptions): void {
  if (!isSitemapApp(app)) {
    test.skip(`navigation specs unavailable for '${app}' (no sitemap routes)`, () => {});
    return;
  }
  const routes = navigablePublicRoutes(app);

  for (const route of routes) {
    test(`reaches public route: ${route.id} (${route.path})`, async ({ page }) => {
      const po = new SitemapPage(page, app, route.id);
      await po.goto();
      await expect(po.main()).toBeAttached();
      // URL settled on the expected path (allowing locale prefix / trailing).
      expect(page.url()).toContain(po.path === '/' ? '' : (po.path.split('?')[0] ?? ''));
    });
  }

  test('primary navigation exposes links', async ({ page }) => {
    const first = routes[0];
    if (!first) {
      test.skip(true, 'no public routes to anchor navigation');
      return;
    }
    const po = new SitemapPage(page, app, first.id);
    await po.goto();
    // At least one navigable link should be present in the nav landmark.
    await expect(po.nav().links().first()).toBeVisible();
  });
}
