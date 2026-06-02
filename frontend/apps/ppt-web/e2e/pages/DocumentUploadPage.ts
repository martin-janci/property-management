/**
 * ppt-web document-upload page object
 * (`ppt-document-upload`, protected `/documents/upload`).
 *
 * Encapsulates the upload form surface backing the `flow-document-upload`
 * user flow. Selectors are testid-first (per the convention's
 * `documents-upload-*` ids) with resilient label/role fallbacks, so they
 * resolve once the component testid sweep merges but never hard-crash here.
 */

import type { Locator, Page } from '@playwright/test';
import { SitemapPage } from '@ppt/e2e';
import { PPT_DOCUMENT_UPLOAD_ROUTE_ID, PPT_WEB_APP } from './constants';

export class DocumentUploadPage extends SitemapPage {
  constructor(page: Page) {
    super(page, PPT_WEB_APP, PPT_DOCUMENT_UPLOAD_ROUTE_ID);
  }

  /** Page heading (first heading landmark). */
  pageHeading(): Locator {
    return this.heading().first();
  }

  /** The upload form (`upload-form` per the flow catalog). */
  form(): Locator {
    return this.locators()
      .testId('upload-form')
      .or(this.main().getByRole('form'))
      .or(this.main().locator('form'))
      .first();
  }

  /** The file-picker input. */
  fileInput(): Locator {
    return this.locators()
      .testId('documents-upload-file-input')
      .or(this.main().locator('input[type="file"]'))
      .first();
  }

  /** The document-title field. */
  titleInput(): Locator {
    return this.locators()
      .testId('documents-upload-title-input')
      .or(this.locators().label(/title/i))
      .first();
  }

  /** The category selector. */
  categorySelect(): Locator {
    return this.locators()
      .testId('documents-upload-category-select')
      .or(this.locators().label(/category/i))
      .first();
  }

  /** The submit affordance (`upload-btn` per the flow catalog). */
  submitButton(): Locator {
    return this.locators()
      .testId('documents-upload-submit-button')
      .or(this.locators().testId('upload-btn'))
      .or(this.locators().button(/upload|submit|save/i))
      .first();
  }
}
