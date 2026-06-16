/**
 * Protected-route guard checks for reality-web (no backend required).
 *
 * reality-web's protected pages render an in-page "Sign in required" guard for
 * unauthenticated visitors (rather than redirecting). The page title lives
 * outside the guard and is always present, so both the always-rendered shell
 * and the guard can be asserted deterministically — no session, no seed data.
 *
 * Written entirely through the page objects; no raw selectors in the body.
 */

import { expect, test } from '@ppt/e2e';
import { FavoritesPage, InquiriesPage, SavedSearchesPage } from '../pages';

test.describe('reality-web · protected routes (unauthenticated)', () => {
  test('favorites shows the sign-in guard', async ({ page }) => {
    const favorites = new FavoritesPage(page);
    await favorites.goto();

    await expect(favorites.pageTitle().or(favorites.signInRequired())).toBeVisible();
    await expect(favorites.signInRequired()).toBeVisible();
  });

  test('saved searches shows the sign-in guard', async ({ page }) => {
    const saved = new SavedSearchesPage(page);
    await saved.goto();

    await expect(saved.pageTitle().or(saved.signInRequired())).toBeVisible();
    await expect(saved.signInRequired()).toBeVisible();
  });

  test('inquiries shows the sign-in guard', async ({ page }) => {
    const inquiries = new InquiriesPage(page);
    await inquiries.goto();

    await expect(inquiries.pageTitle().or(inquiries.signInRequired())).toBeVisible();
    await expect(inquiries.signInRequired()).toBeVisible();
  });
});
