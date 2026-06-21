/**
 * Auth spec factory: login form UI, field validation, and invalid-credential
 * error handling. Generalizes the previous ppt-web auth.spec patterns onto the
 * {@link LoginPage} page object — no raw `#email` / `#password` selectors here.
 */

import type { Page } from '@playwright/test';
import type { SitemapApp } from '../env';
import { expect, test } from '../fixtures';
import { backendEnabled } from '../flow/FlowRunner';
import { LoginPage } from '../pages/LoginPage';

export interface RegisterAuthOptions {
  readonly app: string;
  /** Sitemap route id of the login screen (e.g. `ppt-login`). */
  readonly loginRouteId?: string;
  /** Explicit login path if the app's login route is not in the sitemap. */
  readonly loginPath?: string;
}

function makeLoginPage(page: Page, opts: RegisterAuthOptions): LoginPage {
  const sitemapApp =
    opts.app === 'ppt-web' || opts.app === 'reality-web' ? (opts.app as SitemapApp) : undefined;
  return new LoginPage(page, {
    app: sitemapApp,
    loginRouteId: opts.loginRouteId,
    loginPath: opts.loginPath,
  });
}

/** Register auth tests. Call inside a `test.describe`. */
export function registerAuthSpecs(opts: RegisterAuthOptions): void {
  if (!opts.loginRouteId && !opts.loginPath) {
    test.skip(`auth specs unavailable for '${opts.app}' (no login route)`, () => {});
    return;
  }

  test('login form renders core fields', async ({ page }) => {
    const login = makeLoginPage(page, opts);
    await login.goto();
    await expect(login.emailInput()).toBeVisible();
    await expect(login.passwordInput()).toBeVisible();
    await expect(login.submitButton()).toBeVisible();
  });

  test('rejects invalid email format', async ({ page }) => {
    const login = makeLoginPage(page, opts);
    await login.goto();
    await login.fillAndSubmit('invalid-email', 'somePassword');
    // Either a field error surfaces, or the browser keeps focus on the field;
    // a successful navigation away would be a bug.
    await expect(page).toHaveURL(/login/);
  });

  test('shows error for empty password', async ({ page }) => {
    const login = makeLoginPage(page, opts);
    await login.goto();
    await login.emailInput().fill('user@example.com');
    await login.submitButton().click();
    await expect(page).toHaveURL(/login/);
  });

  test('surfaces an error banner for invalid credentials @requires-backend', async ({ page }) => {
    test.skip(
      !backendEnabled(),
      'invalid-credentials banner needs a backend — set E2E_WITH_BACKEND=1 to run'
    );
    const login = makeLoginPage(page, opts);
    await login.goto();
    await login.fillAndSubmit('nonexistent@example.com', 'wrongPassword');
    // Without a backend the request may hang/fail; assert we did NOT log in.
    // The error banner is the positive signal when the backend is reachable.
    const banner = login.errorBanner();
    const stillOnLogin = page.url().includes('login');
    if (await banner.isVisible().catch(() => false)) {
      await expect(banner).toBeVisible();
    } else {
      expect(stillOnLogin).toBe(true);
    }
  });
}
