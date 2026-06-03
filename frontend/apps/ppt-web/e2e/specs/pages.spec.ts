/**
 * ppt-web critical-page smoke specs.
 *
 * Migrated from the legacy `pages.spec.ts`. The sitemap route-health factory
 * already proves each public route renders a `main` landmark + heading without
 * console errors; this file keeps the page-object-specific assertions for the
 * protected pages (which may render an auth error boundary) and the graceful
 * 404 handling — all through page objects, no raw `page.locator('main')`.
 */

import { backendEnabled, expect, test } from '@ppt/e2e';
import {
  DisputesPage,
  DocumentsPage,
  EmergencyContactsPage,
  HomePage,
  LoginPage,
  NewsPage,
} from '../pages';

/**
 * Protected pages. documents/news/emergency are auth-gated: without a backend
 * the app redirects to `/login` (or renders only an auth error boundary with no
 * stable heading), so these assertions can't hold on a code-only run. They skip
 * cleanly unless a backend is wired (E2E_WITH_BACKEND=1). The public disputes
 * page and the 404 / public direct-access cases below keep running.
 */
test.describe('ppt-web · documents page @requires-backend', () => {
  test.skip(!backendEnabled(), 'protected page redirects to /login without a backend');

  test('loads with a main landmark and heading', async ({ page }) => {
    const documents = new DocumentsPage(page);
    await documents.goto();

    await expect(page).toHaveURL(/\/documents/);
    await expect(documents.main()).toBeVisible();
    await expect(documents.pageHeading()).toBeVisible();
  });
});

test.describe('ppt-web · news page @requires-backend', () => {
  test.skip(!backendEnabled(), 'protected page redirects to /login without a backend');

  test('loads content or an auth error boundary', async ({ page }) => {
    const news = new NewsPage(page);
    await news.goto();

    await expect(page).toHaveURL(/\/news/);
    await expect(news.contentOrError()).toBeVisible();
    await expect(news.pageHeading()).toBeVisible();
  });
});

test.describe('ppt-web · emergency contacts page @requires-backend', () => {
  test.skip(!backendEnabled(), 'protected page redirects to /login without a backend');

  test('loads content or an auth error boundary', async ({ page }) => {
    const emergency = new EmergencyContactsPage(page);
    await emergency.goto();

    await expect(page).toHaveURL(/\/emergency/);
    await expect(emergency.contentOrError()).toBeVisible();
    await expect(emergency.pageHeading()).toBeVisible();
  });
});

test.describe('ppt-web · disputes page', () => {
  test('loads with a main landmark', async ({ page }) => {
    const disputes = new DisputesPage(page);
    await disputes.goto();

    await expect(page).toHaveURL(/\/disputes/);
    await expect(disputes.main()).toBeVisible();
  });
});

test.describe('ppt-web · unknown routes', () => {
  test('handles an unknown route without crashing', async ({ page }) => {
    await page.goto('/nonexistent-page-12345');

    // Either a 404 view, an error boundary, or some body content — never a crash.
    const fallback = page.getByRole('main').or(page.getByRole('alert')).or(page.locator('body'));
    await expect(fallback.first()).toBeVisible();
  });
});

test.describe('ppt-web · direct URL access', () => {
  test('loads the home page directly', async ({ page }) => {
    const home = new HomePage(page);
    await home.goto();

    await expect(page).toHaveURL('/');
    await expect(home.main()).toBeVisible();
  });

  test('loads the login page directly', async ({ page }) => {
    const login = new LoginPage(page);
    await login.goto();

    await expect(page).toHaveURL('/login');
    await expect(login.main()).toBeVisible();
  });

  test('loads the documents page directly @requires-backend', async ({ page }) => {
    // /documents is auth-gated; direct access without a backend redirects to
    // /login, so this only holds against a real backend.
    test.skip(!backendEnabled(), 'protected page redirects to /login without a backend');

    const documents = new DocumentsPage(page);
    await documents.goto();

    await expect(page).toHaveURL(/\/documents/);
    await expect(documents.main()).toBeVisible();
  });
});
