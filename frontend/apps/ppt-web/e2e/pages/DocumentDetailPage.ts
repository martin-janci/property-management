/**
 * ppt-web document-detail page object (`ppt-document-detail`,
 * protected `/documents/:documentId`).
 *
 * The `:documentId` path param is resolved through the sitemap via the
 * framework's {@link SitemapPage} param substitution — no manual string
 * interpolation of the URL.
 */

import type { Locator, Page } from '@playwright/test';
import { SitemapPage } from '@ppt/e2e';
import { PPT_DOCUMENT_DETAIL_ROUTE_ID, PPT_WEB_APP } from './constants';

export class DocumentDetailPage extends SitemapPage {
  constructor(page: Page, documentId: string) {
    super(page, PPT_WEB_APP, PPT_DOCUMENT_DETAIL_ROUTE_ID, { params: { documentId } });
  }

  /** Document title heading. */
  pageHeading(): Locator {
    return this.heading().first();
  }

  /** The document-detail panel (testid `document-detail`). */
  detailPanel(): Locator {
    return this.locators().testId('document-detail').or(this.main()).first();
  }

  /** The download affordance on the detail surface. */
  downloadLink(): Locator {
    return this.locators()
      .testId('documents-detail-download-link')
      .or(this.locators().link(/download/i))
      .first();
  }
}
