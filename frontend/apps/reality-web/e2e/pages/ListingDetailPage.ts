/**
 * ListingDetailPage — a single property listing (`reality-listing-detail`).
 *
 * Path needs a `slug`; it is resolved through the sitemap so the `[slug]`
 * segment substitution lives in one place. Selectors are testid-first
 * (`favorite-btn` is verified) with role/text fallback via {@link locate}.
 */

import type { Locator, Page } from '@playwright/test';
import { BasePage, type GotoOptions, locate, sitemapPage } from '@ppt/e2e';
import { APP, type RealityLocale, withLocale } from './support';

export class ListingDetailPage extends BasePage {
  private readonly resolvedPath: string;

  constructor(page: Page, slug: string, locale?: RealityLocale) {
    super(page);
    const base = sitemapPage(page, APP, 'reality-listing-detail', { params: { slug } }).path;
    this.resolvedPath = withLocale(base, locale);
  }

  get path(): string {
    return this.resolvedPath;
  }

  override async goto(options?: GotoOptions): Promise<void> {
    await super.goto(options);
    await this.title().waitFor({ state: 'visible' });
  }

  /** The listing title heading. */
  title(): Locator {
    return this.heading().first();
  }

  /** Favorite toggle for this listing — verified `favorite-btn` testid. */
  favoriteButton(): Locator {
    return locate(this.page, {
      testId: 'favorite-btn',
      role: 'button',
      name: /favorit|obľúb/i,
    });
  }

  /** Whether the listing is currently marked as a favorite. */
  async isFavorited(): Promise<boolean> {
    const button = this.favoriteButton();
    const favorited = await button.getAttribute('data-favorited');
    if (favorited !== null) {
      return favorited === 'true';
    }
    const label = await button.getAttribute('aria-label');
    return /remove from favorites|odstrániť/i.test(label ?? '');
  }

  /** "Contact agent" call-to-action. */
  contactAgentButton(): Locator {
    return locate(this.page, {
      testId: 'listing-contact-agent-button',
      role: 'button',
      name: /contact agent|kontakt/i,
    });
  }

  /** "Schedule viewing" call-to-action. */
  scheduleViewingButton(): Locator {
    return locate(this.page, {
      testId: 'listing-schedule-viewing-button',
      role: 'button',
      name: /schedule viewing|obhliad/i,
    });
  }

  /** The "not found" heading shown for an unknown slug. */
  notFound(): Locator {
    return locate(this.main(), {
      testId: 'listing-not-found',
      role: 'heading',
      text: /listing not found|nenájden/i,
    });
  }
}
