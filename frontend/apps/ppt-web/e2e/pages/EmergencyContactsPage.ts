/**
 * ppt-web emergency-contacts page object
 * (`ppt-emergency`, protected `/emergency`).
 *
 * Like {@link NewsPage}, this route can render an error boundary when accessed
 * unauthenticated; both surfaces are exposed.
 */

import type { Locator, Page } from '@playwright/test';
import { SitemapPage } from '@ppt/e2e';
import { PPT_EMERGENCY_ROUTE_ID, PPT_WEB_APP } from './constants';

export class EmergencyContactsPage extends SitemapPage {
  constructor(page: Page) {
    super(page, PPT_WEB_APP, PPT_EMERGENCY_ROUTE_ID);
  }

  /** Page heading (first heading landmark — content or error message). */
  pageHeading(): Locator {
    return this.heading().first();
  }

  /** The emergency-contact directory search field — anchors the directory. */
  directorySearch(): Locator {
    return this.locators()
      .testId('emergency-directory-search-input')
      .or(this.main().getByRole('searchbox'))
      .first();
  }

  /** The emergency-contact directory list region. */
  directory(): Locator {
    return this.locators().testId('emergency-directory').or(this.main().getByRole('list')).first();
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
