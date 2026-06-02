/**
 * ppt-web disputes page object (`ppt-disputes`, protected `/disputes`).
 *
 * Encapsulates the disputes list and the affordance that opens the
 * "file a dispute" flow (`ppt-dispute-new`).
 */

import type { Locator, Page } from '@playwright/test';
import { SitemapPage } from '@ppt/e2e';
import { PPT_DISPUTES_ROUTE_ID, PPT_WEB_APP } from './constants';

export class DisputesPage extends SitemapPage {
  constructor(page: Page) {
    super(page, PPT_WEB_APP, PPT_DISPUTES_ROUTE_ID);
  }

  /** Page heading (first heading landmark). */
  pageHeading(): Locator {
    return this.heading().first();
  }

  /** The disputes list / table region. */
  list(): Locator {
    return this.locators()
      .testId('disputes-list')
      .or(this.main().getByRole('list'))
      .or(this.main().getByRole('table'))
      .first();
  }

  /** Affordance that opens the file-a-dispute flow (`ppt-dispute-new`). */
  fileDisputeButton(): Locator {
    return this.locators()
      .testId('disputes-create-button')
      .or(this.locators().link(/file|new dispute|new/i))
      .or(this.locators().button(/file|new dispute|new/i))
      .first();
  }
}
