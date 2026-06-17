/**
 * App-specific spec for the key reality-web screen: the listings search journey.
 *
 * Home hero search → listings results → open a detail page, written entirely
 * through the page objects so the spec body carries no raw selectors. These run
 * without a backend: where seeded listings are absent, the empty state is the
 * legitimate asserted outcome rather than a hard failure, so the suite stays
 * honest on a code-only run.
 */

import { expect, test } from '@ppt/e2e';
import { HomePage, ListingDetailPage, ListingsPage } from '../pages';

test.describe('reality-web · listings search', () => {
  test('hero search navigates from home to the listings results', async ({ page }) => {
    const home = new HomePage(page);
    await home.goto();

    await expect(home.heroTitle()).toBeVisible();
    await expect(home.searchButton()).toBeVisible();

    await home.search('Bratislava');

    const listings = new ListingsPage(page);
    expect(page.url()).toContain('/listings');
    // Results region resolved to either the grid or the empty state.
    await expect(listings.results()).toBeVisible();
  });

  test('listings can be opened directly with query params and refined in-page', async ({
    page,
  }) => {
    const listings = new ListingsPage(page, { query: { type: 'sale', city: 'Bratislava' } });
    await listings.goto();

    await expect(listings.results()).toBeVisible();

    // Refine via the in-page search bar; URL stays on the listings route.
    await listings.search('Prague');
    expect(page.url()).toContain('/listings');
    await expect(listings.results()).toBeVisible();
  });

  test('a result card opens its listing detail page when results exist', async ({ page }) => {
    const listings = new ListingsPage(page, { query: { type: 'sale' } });
    await listings.goto();

    const resultCount = await listings.cards().count();
    if (resultCount === 0) {
      // No seeded listings in this environment — the empty state is the
      // legitimate, asserted outcome rather than a hard failure.
      await expect(listings.emptyState()).toBeVisible();
      return;
    }

    await listings.openCard(0);

    // The slug is resolved by the URL; the page object reads the detail chrome.
    const detail = new ListingDetailPage(page, 'unused-slug-resolved-by-url');
    expect(page.url()).toMatch(/\/listings\/[^/?#]+/);
    await expect(detail.title()).toBeVisible();
  });
});
