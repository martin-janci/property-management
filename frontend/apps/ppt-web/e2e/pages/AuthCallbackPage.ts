/**
 * ppt-web SSO callback page object (`ppt-auth-callback`,
 * public `/auth/callback`).
 *
 * Backs the OAuth-callback integration specs: the `code` / `state` query params
 * are resolved through the sitemap route (no manual URL string-building) and
 * the error banner is exposed as a single resilient selector.
 */

import type { Locator, Page } from '@playwright/test';
import { SitemapPage } from '@ppt/e2e';
import { PPT_WEB_APP } from './constants';

const PPT_AUTH_CALLBACK_ROUTE_ID = 'ppt-auth-callback';

export interface CallbackParams {
  readonly code?: string;
  readonly state?: string;
}

export class AuthCallbackPage extends SitemapPage {
  constructor(page: Page, params: CallbackParams = {}) {
    super(page, PPT_WEB_APP, PPT_AUTH_CALLBACK_ROUTE_ID, {
      query: AuthCallbackPage.toQuery(params),
    });
  }

  /** The error / alert banner shown on a failed or rejected callback. */
  errorBanner(): Locator {
    return this.locators().role('alert').first();
  }

  /**
   * Navigate to the callback without the post-nav `waitReady()` gate: several
   * scenarios deliberately render only an error banner (no `main` landmark) or
   * redirect immediately, so we let the spec drive the wait it needs.
   */
  async open(): Promise<void> {
    await this.goto({ skipReady: true });
  }

  private static toQuery(params: CallbackParams): Record<string, string> {
    const query: Record<string, string> = {};
    if (params.code !== undefined) {
      query.code = params.code;
    }
    if (params.state !== undefined) {
      query.state = params.state;
    }
    return query;
  }
}
