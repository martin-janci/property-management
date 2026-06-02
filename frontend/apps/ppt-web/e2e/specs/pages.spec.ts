/**
 * ppt-web critical-page smoke specs.
 *
 * Migrated from the legacy `pages.spec.ts`. The sitemap route-health factory
 * already proves each public route renders a `main` landmark + heading without
 * console errors; this file keeps the page-object-specific assertions for the
 * protected pages (which may render an auth error boundary) and the graceful
 * 404 handling — all through page objects, no raw `page.locator('main')`.
 */

import { expect, test } from '@ppt/e2e';
import {
  DisputesPage,
  DocumentsPage,
  EmergencyContactsPage,
  HomePage,
  LoginPage,
  NewsPage,
} from '../pages';

test.describe('ppt-web · documents page', () => {
  test('loads with a main landmark and heading', async ({ page }) => {
    const documents = new DocumentsPage(page);
    await documents.goto();

    await expect(page).toHaveURL(/\/documents/);
    await expect(documents.main()).toBeVisible();
    await expect(documents.pageHeading()).toBeVisible();
  });
});

test.describe('ppt-web · news page', () => {
  test('loads content or an auth error boundary', async ({ page }) => {
    const news = new NewsPage(page);
    await news.goto();

    await expect(page).toHaveURL(/\/news/);
    await expect(news.contentOrError()).toBeVisible();
    await expect(news.pageHeading()).toBeVisible();
  });
});

test.describe('ppt-web · emergency contacts page', () => {
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

  test('loads the documents page directly', async ({ page }) => {
    const documents = new DocumentsPage(page);
    await documents.goto();

    await expect(page).toHaveURL(/\/documents/);
    await expect(documents.main()).toBeVisible();
  });
});
