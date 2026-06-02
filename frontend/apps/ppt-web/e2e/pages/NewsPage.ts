/**
 * ppt-web news page object (`ppt-news`, protected `/news`).
 *
 * Unauthenticated, the route may render an error boundary instead of the list;
 * the page object exposes both surfaces so specs can assert "content OR auth
 * error" without raw selectors.
 */

import type { Locator, Page } from '@playwright/test';
import { SitemapPage } from '@ppt/e2e';
import { PPT_NEWS_ROUTE_ID, PPT_WEB_APP } from './constants';

export class NewsPage extends SitemapPage {
  constructor(page: Page) {
    super(page, PPT_WEB_APP, PPT_NEWS_ROUTE_ID);
  }

  /** Page heading (first heading landmark — content or error message). */
  pageHeading(): Locator {
    return this.heading().first();
  }

  /** Error boundary / alert region shown when access is unauthorized. */
  errorBoundary(): Locator {
    return this.locators().role('alert').first();
  }

  /** Either the main content or the auth-error boundary — whichever renders. */
  contentOrError(): Locator {
    return this.main().or(this.errorBoundary()).first();
  }
}
