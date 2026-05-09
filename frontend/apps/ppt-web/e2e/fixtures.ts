/**
 * Playwright Test Fixtures
 * Epic 131: E2E Test Suite
 *
 * Custom fixtures for E2E tests including authentication helpers.
 */

import { test as base, type Page } from '@playwright/test';

/** Test user credentials for E2E testing */
export const testUsers = {
  admin: {
    email: 'admin@test.example',
    password: 'TestPassword123!',
  },
  manager: {
    email: 'manager@test.example',
    password: 'TestPassword123!',
  },
  resident: {
    email: 'resident@test.example',
    password: 'TestPassword123!',
  },
};

/** Extended test fixtures with authentication helpers */
export const test = base.extend<{
  /** Authenticated page - logs in before test */
  authenticatedPage: Page;
}>({
  authenticatedPage: async ({ page }, use) => {
    // Navigate to login page
    await page.goto('/login');

    // Fill in credentials using ID selectors (more reliable)
    await page.locator('#email').fill(testUsers.manager.email);
    await page.locator('#password').fill(testUsers.manager.password);

    // Submit login form
    await page.getByRole('button', { name: /sign in/i }).click();

    // Wait for navigation to complete (home page or dashboard)
    // This will throw if backend is unavailable - tests using this fixture
    // should handle the error appropriately
    await page.waitForURL(/^\/$|\/dashboard/, { timeout: 10000 });

    // Use the authenticated page
    await use(page);
  },
});

export { expect } from '@playwright/test';

/**
 * Helper to wait for page to be fully loaded
 */
export async function waitForPageLoad(page: Page) {
  await page.waitForLoadState('networkidle');
}

/**
 * Helper to dismiss any toast notifications
 */
export async function dismissToasts(page: Page) {
  const dismissButtons = page.locator('[aria-label*="dismiss"], [aria-label*="close"]');
  const count = await dismissButtons.count();
  for (let i = 0; i < count; i++) {
    await dismissButtons
      .nth(i)
      .click()
      .catch(() => {
        // Ignore if button is already gone
      });
  }
}
