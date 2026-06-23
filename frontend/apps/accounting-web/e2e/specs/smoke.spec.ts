/**
 * accounting-web smoke suite.
 *
 * No backend required: verifies that the app boots and the login gate works.
 * Protected routes redirect to /login; the login form itself is reachable.
 */

import { expect, test } from '@ppt/e2e';

test.describe('accounting-web · auth gate', () => {
  test('root redirects to login', async ({ page }) => {
    await page.goto('/');
    await expect(page).toHaveURL(/\/login/);
  });

  test('login page renders a submit button', async ({ page }) => {
    await page.goto('/sk/login');
    const button = page.getByRole('button', { name: /prihlásiť|login|sign in/i });
    await expect(button).toBeVisible();
  });

  test('protected invoice route redirects to login when unauthenticated', async ({ page }) => {
    await page.goto('/sk/app/invoices');
    await expect(page).toHaveURL(/\/login/);
  });

  test('protected bank-matching route redirects to login when unauthenticated', async ({
    page,
  }) => {
    await page.goto('/sk/app/bank-matching');
    await expect(page).toHaveURL(/\/login/);
  });
});
