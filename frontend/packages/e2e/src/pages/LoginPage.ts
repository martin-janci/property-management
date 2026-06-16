/**
 * Reusable login page object.
 *
 * Generalizes the (previously duplicated) ppt-web auth selectors into one
 * resilient surface. The login path is resolved from the sitemap when a
 * `loginRouteId` is supplied; otherwise it falls back to a sensible default.
 *
 * Selector strategy (resilient → fragile fallback):
 *  - email:    `getByLabel(/email/i)` → `#email`
 *  - password: `getByLabel(/password/i)` → `#password`
 *  - submit:   `role=button name=/sign in|log in|prihlasit/i`
 */

import type { Locator, Page } from '@playwright/test';
import { getRoute } from '@ppt/sitemap';
import type { SitemapApp } from '../env';
import { BasePage } from './BasePage';

export interface LoginPageOptions {
  /** App whose login route lives in the sitemap (ppt-web / reality-web). */
  readonly app?: SitemapApp;
  /** Sitemap route id of the login screen (e.g. `ppt-login`). */
  readonly loginRouteId?: string;
  /** Explicit login path; wins over sitemap resolution. */
  readonly loginPath?: string;
}

export class LoginPage extends BasePage {
  private readonly loginPath: string;

  constructor(page: Page, options: LoginPageOptions = {}) {
    super(page);
    this.loginPath = LoginPage.resolveLoginPath(options);
  }

  get path(): string {
    return this.loginPath;
  }

  /**
   * Wait for the login form rather than a `<main>` landmark. Some login screens
   * (e.g. admin-web's bare `<form aria-label="admin login">`) render no `<main>`
   * at all, so the BasePage default would time out. Waiting on the email field
   * is the resilient, app-agnostic readiness signal for a login surface.
   */
  override async waitReady(): Promise<void> {
    await this.page.waitForLoadState('domcontentloaded');
    await this.emailInput().waitFor({ state: 'visible' });
  }

  // ---- form fields (single source of truth for login selectors) ----

  emailInput(): Locator {
    return this.page.getByLabel(/email/i).or(this.page.locator('#email')).first();
  }

  passwordInput(): Locator {
    return this.page
      .getByLabel(/password|heslo/i)
      .or(this.page.locator('#password'))
      .first();
  }

  submitButton(): Locator {
    return this.page.getByRole('button', { name: /sign in|log ?in|prihlás/i });
  }

  /** Field-level email validation error. */
  emailError(): Locator {
    return this.page.locator('#email-error');
  }

  /** Field-level password validation error. */
  passwordError(): Locator {
    return this.page.locator('#password-error');
  }

  /** General invalid-credentials / failure banner. */
  errorBanner(): Locator {
    return this.page.locator('.login-error-banner').or(this.page.getByRole('alert')).first();
  }

  // ---- actions ----

  /** Type credentials and submit. Does not assert outcome. */
  async fillAndSubmit(email: string, password: string): Promise<void> {
    await this.emailInput().fill(email);
    await this.passwordInput().fill(password);
    await this.submitButton().click();
  }

  private static resolveLoginPath(options: LoginPageOptions): string {
    if (options.loginPath) {
      return options.loginPath;
    }
    if (options.app && options.loginRouteId) {
      const route = getRoute(options.app, options.loginRouteId);
      if (route) {
        return route.path;
      }
    }
    return '/login';
  }
}
