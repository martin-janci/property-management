/**
 * ppt-web navigation & shell specs.
 *
 * Migrated from the legacy `navigation.spec.ts` + `home.spec.ts`. The generic
 * sitemap factory (`registerNavigationSpecs`) already proves every public route
 * is reachable; this file keeps the ppt-web-specific shell behaviour — named
 * nav-link clicks, the skip link, connection status, the language switcher, the
 * offline indicator, the document title, and responsive rendering — all driven
 * through the {@link HomePage} page object (no raw nav selectors in the body).
 */

import { backendEnabled, expect, test } from '@ppt/e2e';
import { HomePage } from '../pages';

test.describe('ppt-web · primary navigation', () => {
  // Anonymous visitors get the slim public header (brand, language, sign in,
  // create account). The module navigation lives in the authenticated sidebar,
  // so operator/resident links are no longer advertised to logged-out users.
  test('exposes the public nav with sign-in and create-account links', async ({ page }) => {
    const home = new HomePage(page);
    await home.goto();

    await expect(home.nav().root).toBeVisible();
    await expect(home.signInLink()).toBeVisible();
    await expect(home.createAccountLink()).toBeVisible();
  });

  test('navigates to the login page via the nav link', async ({ page }) => {
    const home = new HomePage(page);
    await home.goto();

    await home.signInLink().click();
    await expect(page).toHaveURL(/\/login/);
  });

  test('navigates to registration via the nav link', async ({ page }) => {
    const home = new HomePage(page);
    await home.goto();

    await home.createAccountLink().click();
    await expect(page).toHaveURL(/\/register/);
  });
});

/**
 * Protected-route navigation. documents/news/emergency are auth-gated routes:
 * without a backend the app redirects clicks to `/login`, so clicking the nav
 * link never settles on the target URL. The "back home" case depends on first
 * reaching a protected route, so it lives here too. These skip cleanly unless
 * a backend is wired (E2E_WITH_BACKEND=1).
 */
test.describe('ppt-web · protected-route navigation @requires-backend', () => {
  test.skip(!backendEnabled(), 'protected routes redirect to /login without a backend');

  test('navigates to the documents page via the nav link', async ({ page }) => {
    const home = new HomePage(page);
    await home.goto();

    await home.documentsLink().click();
    await expect(page).toHaveURL(/\/documents/);
  });

  test('navigates to the news page via the nav link', async ({ page }) => {
    const home = new HomePage(page);
    await home.goto();

    await home.newsLink().click();
    await expect(page).toHaveURL(/\/news/);
  });

  test('navigates to the emergency contacts page via the nav link', async ({ page }) => {
    const home = new HomePage(page);
    await home.goto();

    await home.emergencyLink().click();
    await expect(page).toHaveURL(/\/emergency/);
  });

  test('navigates back home from another page', async ({ page }) => {
    const home = new HomePage(page);
    await home.goto();

    await home.documentsLink().click();
    await expect(page).toHaveURL(/\/documents/);

    await home.homeLink().click();
    await expect(page).toHaveURL('/');
  });
});

test.describe('ppt-web · app shell', () => {
  test('has a document title for the home page', async ({ page }) => {
    const home = new HomePage(page);
    await home.goto();

    await expect(page).toHaveTitle(/Property Management|PPT/i);
  });

  test('provides a skip-to-main-content link', async ({ page }) => {
    const home = new HomePage(page);
    await home.goto();

    await expect(home.skipToContentLink()).toBeAttached();
  });

  test('shows the connection status indicator', async ({ page }) => {
    const home = new HomePage(page);
    await home.goto();

    await expect(home.connectionIndicator()).toBeVisible();
  });

  test('does not show the offline indicator while online', async ({ page }) => {
    const home = new HomePage(page);
    await home.goto();

    await expect(home.offlineIndicator()).not.toBeVisible();
  });
});

test.describe('ppt-web · language switcher', () => {
  test('renders the language switcher control', async ({ page }) => {
    const home = new HomePage(page);
    await home.goto();

    await expect(home.languageControl()).toBeVisible();
  });
});

test.describe('ppt-web · responsive shell', () => {
  for (const viewport of [
    { name: 'mobile', width: 375, height: 667 },
    { name: 'tablet', width: 768, height: 1024 },
    { name: 'desktop', width: 1920, height: 1080 },
  ] as const) {
    test(`renders main content at ${viewport.name} viewport`, async ({ page }) => {
      await page.setViewportSize({ width: viewport.width, height: viewport.height });
      const home = new HomePage(page);
      await home.goto();

      await expect(home.main()).toBeVisible();
    });
  }
});
