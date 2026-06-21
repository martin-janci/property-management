/**
 * ppt-web login page object.
 *
 * Extends the framework {@link FrameworkLoginPage} (which already owns the
 * resilient email / password / submit / error selectors) and layers on the
 * ppt-web-specific surface the legacy `auth.spec.ts` exercised: the "Sign in"
 * heading and the password-visibility toggle.
 *
 * The login path is sitemap-resolved from `ppt-login`; no hardcoded `/login`.
 */

import type { Locator, Page } from '@playwright/test';
import { LoginPage as FrameworkLoginPage } from '@ppt/e2e';
import { PPT_LOGIN_ROUTE_ID, PPT_WEB_APP } from './constants';

export class LoginPage extends FrameworkLoginPage {
  constructor(page: Page) {
    super(page, { app: PPT_WEB_APP, loginRouteId: PPT_LOGIN_ROUTE_ID });
  }

  /** The "Sign in" page heading. */
  signInHeading(): Locator {
    return this.heading(/sign in/i);
  }

  /** Password-visibility toggle button (ppt-web specific control). */
  passwordToggle(): Locator {
    return this.locators()
      .testId('login-password-toggle')
      .or(this.page.locator('.login-password-toggle'))
      .first();
  }

  /** Toggle password visibility and return the resulting input `type`. */
  async togglePasswordVisibility(): Promise<string | null> {
    await this.passwordToggle().click();
    return this.passwordInput().getAttribute('type');
  }
}
