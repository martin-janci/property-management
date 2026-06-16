/**
 * InquiriesPage — viewing & contact inquiries (`reality-inquiries`, auth-gated).
 *
 * Lists the user's submitted inquiries (each card titled by the listing it
 * targets) or an empty state. Selectors are testid-first with role/text
 * fallback via {@link locate}. Exposes the in-page "Sign in required" guard so
 * unauthenticated specs can assert it without a session or backend.
 */

import type { Locator, Page } from '@playwright/test';
import { BasePage, type GotoOptions, locate, sitemapPage } from '@ppt/e2e';
import { APP, type RealityLocale, withLocale } from './support';

export class InquiriesPage extends BasePage {
  private readonly resolvedPath: string;

  constructor(page: Page, locale?: RealityLocale) {
    super(page);
    this.resolvedPath = withLocale(sitemapPage(page, APP, 'reality-inquiries').path, locale);
  }

  get path(): string {
    return this.resolvedPath;
  }

  override async goto(options?: GotoOptions): Promise<void> {
    await super.goto(options);
    await this.ready().waitFor({ state: 'visible' });
  }

  /** Settled state: the page heading (authed) or the sign-in guard (anon). */
  ready(): Locator {
    return this.pageTitle().or(this.signInRequired());
  }

  /** Page heading ("Inquiries") — the level-1 title, always rendered. */
  pageTitle(): Locator {
    return this.main().getByRole('heading', { level: 1, name: /inquir|dopyt|žiadost/i });
  }

  /** All inquiry cards. */
  cards(): Locator {
    return locate(this.main(), {
      testId: 'inquiry-card',
      css: '[data-testid="inquiry-card"]',
    });
  }

  /** Either an inquiry card or the empty-state heading. */
  content(): Locator {
    return this.cards().first().or(this.emptyState());
  }

  /** Empty-state heading shown when there are no inquiries. */
  emptyState(): Locator {
    return locate(this.main(), {
      testId: 'inquiries-empty-state',
      role: 'heading',
      text: /no inquiries|žiadne/i,
    });
  }

  /** The "Sign in required" guard shown to unauthenticated visitors. */
  signInRequired(): Locator {
    return locate(this.main(), {
      testId: 'sign-in-required',
      role: 'heading',
      text: /sign in required|prihlás/i,
    });
  }

  /** How many inquiries are currently rendered. */
  async count(): Promise<number> {
    return this.cards().count();
  }
}
