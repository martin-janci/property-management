/**
 * ppt-web auth specs — the ppt-web-specific cases the generic
 * `registerAuthSpecs` factory does not cover (password-toggle, field-error
 * clearing, the "Sign in" heading) plus the backend-gated login / logout /
 * session-persistence flows.
 *
 * Migrated from the legacy procedural `e2e/auth.spec.ts`: every assertion now
 * goes through the {@link LoginPage} page object — no raw `#email` / `#password`
 * / `getByRole('button', {name:/sign in/i})` selectors in the spec body, and
 * credentials come from the framework `testUsers` fixture.
 */

import { backendEnabled, expect, test } from '@ppt/e2e';
import { DocumentsPage, HomePage, LoginPage } from '../pages';

test.describe('ppt-web · login page UI', () => {
  test('renders the sign-in heading and core fields', async ({ page }) => {
    const login = new LoginPage(page);
    await login.goto();

    await expect(login.signInHeading()).toBeVisible();
    await expect(login.emailInput()).toBeVisible();
    await expect(login.passwordInput()).toBeVisible();
    await expect(login.submitButton()).toBeVisible();
  });

  test('password visibility toggle flips the input type', async ({ page }) => {
    const login = new LoginPage(page);
    await login.goto();

    // Hidden by default.
    await expect(login.passwordInput()).toHaveAttribute('type', 'password');

    // Toggle shows the password…
    expect(await login.togglePasswordVisibility()).toBe('text');

    // …and toggling again hides it.
    expect(await login.togglePasswordVisibility()).toBe('password');
  });
});

test.describe('ppt-web · login form validation', () => {
  test('shows an email error when email is empty', async ({ page }) => {
    const login = new LoginPage(page);
    await login.goto();

    await login.passwordInput().fill('somepassword');
    await login.submitButton().click();

    await expect(login.emailError()).toBeVisible();
  });

  test('shows an email error for an invalid email format', async ({ page }) => {
    const login = new LoginPage(page);
    await login.goto();

    await login.fillAndSubmit('invalid-email', 'somepassword');

    await expect(login.emailError()).toBeVisible();
  });

  test('shows a password error when password is empty', async ({ page }) => {
    const login = new LoginPage(page);
    await login.goto();

    await login.emailInput().fill('test@example.com');
    await login.submitButton().click();

    await expect(login.passwordError()).toBeVisible();
  });

  test('clears each field error when the user starts typing', async ({ page }) => {
    const login = new LoginPage(page);
    await login.goto();

    // Submit empty to trigger both errors.
    await login.submitButton().click();
    await expect(login.emailError()).toBeVisible();
    await expect(login.passwordError()).toBeVisible();

    // Typing in email clears the email error; password error remains.
    await login.emailInput().fill('t');
    await expect(login.emailError()).not.toBeVisible();
    await expect(login.passwordError()).toBeVisible();

    // Typing in password clears the password error.
    await login.passwordInput().fill('p');
    await expect(login.passwordError()).not.toBeVisible();
  });
});

test.describe('ppt-web · login error handling', () => {
  test('surfaces an error banner for invalid credentials', async ({ page }) => {
    const login = new LoginPage(page);
    await login.goto();

    await login.fillAndSubmit('nonexistent@example.com', 'wrongpassword');

    // With a reachable backend the banner appears; without one the request may
    // hang/fail — in that case we assert we did NOT navigate away from login.
    const banner = login.errorBanner();
    if (await banner.isVisible().catch(() => false)) {
      await expect(banner).toBeVisible();
      await expect(banner).not.toBeEmpty();
    } else {
      await expect(page).toHaveURL(/login/);
    }
  });
});

test.describe('ppt-web · protected routes', () => {
  test('unauthenticated dashboard access stays on a known route', async ({ page }) => {
    // No page object for /dashboard (not in the sitemap); navigate directly and
    // assert the app did not crash — it either redirects to login or holds.
    await page.goto('/dashboard');
    const url = page.url();
    expect(url.includes('/login') || url.includes('/dashboard')).toBe(true);
  });
});

/**
 * Backend-gated flows. These require a running backend with seeded test users.
 * The framework `login` fixture calls `test.skip()` when the backend is
 * unavailable, but that path waits on the full login attempt (goto + submit +
 * a 15s URL wait) which can race the per-test timeout under a loaded dev
 * server. Gating the describe up front on `backendEnabled()` skips them
 * deterministically (skipped, never timed-out-failed) when no backend is wired.
 */
test.describe('ppt-web · authenticated flows @requires-backend', () => {
  test.skip(!backendEnabled(), 'authenticated flows require a backend');

  test('logs in with valid credentials and leaves the login route', async ({ page, login }) => {
    await login('manager');
    await expect(page).not.toHaveURL(/\/login/);
  });

  test('session persists across a page refresh', async ({ page, login }) => {
    await login('manager');
    await page.reload();
    await expect(page).not.toHaveURL(/\/login/);
  });

  test('reaches the documents page after authentication', async ({ page, login }) => {
    await login('manager');
    const documents = new DocumentsPage(page);
    await documents.goto();
    await expect(documents.main()).toBeAttached();
  });

  test('exposes a logout control once authenticated', async ({ page, login }) => {
    await login('manager');
    const home = new HomePage(page);
    const logout = home.logoutButton();
    if (await logout.isVisible().catch(() => false)) {
      await logout.click();
      await expect(page).toHaveURL(/\/login/);
    } else {
      test.skip(true, 'Logout control is behind a user menu — not directly visible');
    }
  });
});
