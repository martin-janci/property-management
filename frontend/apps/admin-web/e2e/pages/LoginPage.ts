/**
 * admin-web Login page object.
 *
 * The admin login screen (`apps/admin-web/src/pages/LoginPage.tsx`) is a bare
 * `<form aria-label="admin login">` with wrapped-label Email / Password inputs,
 * an optional TOTP step, and a "Sign in" / "Verify code" submit button. It has
 * NO `<main>` landmark, so `waitReady()` is overridden to wait on the form.
 *
 * All admin-login selectors live here — specs never reach for raw `#email` etc.
 * Selectors are testid-first (resolve once the component sweep merges) with a
 * role/label fallback so they stay green in the interim. The framework ships a
 * generic `LoginPage`, but admin-web's MFA step is app-specific, so the whole
 * form is modelled locally as the single source of truth.
 */

import type { Locator } from '@playwright/test';
import { BasePage, locate } from '@ppt/e2e';
import { ADMIN_ROUTES } from './routes';

export class LoginPage extends BasePage {
  get path(): string {
    return ADMIN_ROUTES.login;
  }

  /** Wait for the login form rather than the (absent) `<main>` landmark. */
  override async waitReady(): Promise<void> {
    await this.page.waitForLoadState('domcontentloaded');
    await this.form().waitFor({ state: 'visible' });
  }

  /** The login `<form>` (labelled region). */
  form(): Locator {
    return locate(this.page, { testId: 'admin-login-form', role: 'form', name: /admin login/i });
  }

  emailInput(): Locator {
    return locate(this.page, {
      testId: 'admin-login-email-input',
      role: 'textbox',
      name: /email/i,
    });
  }

  passwordInput(): Locator {
    // Password fields are not exposed as the `textbox` role, so fall back to the
    // wrapping label rather than a role lookup.
    return locate(this.page, {
      testId: 'admin-login-password-input',
      css: 'input[type="password"]',
    }).or(this.page.getByLabel(/password/i));
  }

  /** TOTP code field — only present after the first submit returns mfaRequired. */
  mfaCodeInput(): Locator {
    return locate(this.page, {
      testId: 'admin-login-mfa-code-input',
      role: 'textbox',
      name: /authenticator code/i,
    });
  }

  submitButton(): Locator {
    return locate(this.page, {
      testId: 'admin-login-submit-button',
      role: 'button',
      name: /sign in|verify code/i,
    });
  }

  /** Invalid-credentials / failure banner (`<p role="alert">`). */
  errorBanner(): Locator {
    return locate(this.page, { testId: 'admin-login-error-alert', role: 'alert' });
  }

  /** Type credentials and submit. Does not assert outcome. */
  async fillAndSubmit(email: string, password: string): Promise<void> {
    await this.emailInput().fill(email);
    await this.passwordInput().fill(password);
    await this.submitButton().click();
  }
}
