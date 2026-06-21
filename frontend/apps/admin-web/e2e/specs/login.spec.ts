/**
 * admin-web login screen — app-specific UI behaviour (no backend required).
 *
 * The login form is a static SPA route, so these run without a backend. They
 * assert the form contract (fields + submit) and the client-side guard that an
 * invalid submit keeps the operator on /login (the native `required`/`type`
 * validation refuses to navigate). All selectors go through the LoginPage page
 * object — no raw locator strings here.
 */

import { expect, test } from '@ppt/e2e';
import { LoginPage } from '../pages';

test.describe('admin-web login', () => {
  test('renders the email, password, and submit controls', async ({ page }) => {
    const login = new LoginPage(page);
    await login.goto();

    await expect(login.form()).toBeVisible();
    await expect(login.emailInput()).toBeVisible();
    await expect(login.passwordInput()).toBeVisible();
    await expect(login.submitButton()).toBeVisible();
  });

  test('does not expose the MFA code field before the first submit', async ({ page }) => {
    const login = new LoginPage(page);
    await login.goto();

    // The TOTP step only renders after the server returns `mfaRequired`.
    await expect(login.mfaCodeInput()).toBeHidden();
  });

  test('keeps an unauthenticated operator on /login after a bad submit', async ({ page }) => {
    const login = new LoginPage(page);
    await login.goto();

    await login.fillAndSubmit('nobody@test.example', 'wrong-password');

    // Without a backend the fetch fails; the page must NOT navigate to the
    // dashboard. Either an error alert surfaces, or we simply stay on /login.
    const banner = login.errorBanner();
    if (await banner.isVisible().catch(() => false)) {
      await expect(banner).toBeVisible();
    }
    await expect(page).toHaveURL(/\/login\/?$/);
  });
});
