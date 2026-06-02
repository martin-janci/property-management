/**
 * ppt-web documents page object (`ppt-documents`, protected `/documents`).
 *
 * Encapsulates the documents-list surface and the navigation affordance to the
 * upload flow. Selectors are testid-first with resilient ARIA/text fallbacks.
 */

import type { Locator, Page } from '@playwright/test';
import { SitemapPage } from '@ppt/e2e';
import { PPT_DOCUMENTS_ROUTE_ID, PPT_WEB_APP } from './constants';

export class DocumentsPage extends SitemapPage {
  constructor(page: Page) {
    super(page, PPT_WEB_APP, PPT_DOCUMENTS_ROUTE_ID);
  }

  /** Page heading (first heading landmark). */
  pageHeading(): Locator {
    return this.heading().first();
  }

  /** The documents browse search field — anchors the list surface. */
  searchInput(): Locator {
    return this.locators()
      .testId('documents-browse-search-input')
      .or(this.main().getByRole('searchbox'))
      .first();
  }

  /** The documents list / table region (individual rows are `document-item`). */
  list(): Locator {
    return this.locators()
      .testId('document-item')
      .or(this.main().getByRole('list'))
      .or(this.main().getByRole('table'))
      .first();
  }

  /** Affordance that opens the document-upload flow. */
  uploadButton(): Locator {
    return this.locators()
      .testId('documents-upload-link')
      .or(this.locators().link(/upload/i))
      .or(this.locators().button(/upload/i))
      .first();
  }
}
