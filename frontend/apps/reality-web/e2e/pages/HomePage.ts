/**
 * HomePage — Reality Portal landing page (`reality-home`).
 *
 * Owns every selector for the hero search section so specs never touch a raw
 * locator. Selectors are testid-first (`search-form` is the verified reality
 * id) with a resilient role/text fallback via {@link locate}: they bind to the
 * real testids once the component sweep merges, yet still resolve today against
 * roles/text. The path is resolved from the sitemap (`/`) via {@link sitemapPage}
 * rather than hardcoded.
 */

import type { Locator, Page } from '@playwright/test';
import { BasePage, type GotoOptions, locate, sitemapPage } from '@ppt/e2e';
import { APP, type RealityLocale, withLocale } from './support';

export class HomePage extends BasePage {
  private readonly resolvedPath: string;

  constructor(page: Page, locale?: RealityLocale) {
    super(page);
    this.resolvedPath = withLocale(sitemapPage(page, APP, 'reality-home').path, locale);
  }

  get path(): string {
    return this.resolvedPath;
  }

  override async goto(options?: GotoOptions): Promise<void> {
    await super.goto(options);
    await this.heroTitle().waitFor({ state: 'visible' });
  }

  /** The hero `<h1>` headline. */
  heroTitle(): Locator {
    return this.heading().first();
  }

  /** The hero search form — verified `search-form` testid, role/css fallback. */
  searchForm(): Locator {
    return locate(this.page, { testId: 'search-form', role: 'search', css: 'form' });
  }

  /** The hero free-text search input, scoped to the search form. */
  searchInput(): Locator {
    return locate(this.searchForm(), {
      testId: 'search-query-input',
      role: 'searchbox',
      css: 'input[type="search"], input[type="text"]',
    });
  }

  /** Submit button for the hero search form. */
  searchButton(): Locator {
    return locate(this.searchForm(), {
      testId: 'search-submit-button',
      role: 'button',
      name: /^search$|hľad/i,
    });
  }

  /** Buy/Sale transaction toggle. */
  buyToggle(): Locator {
    return locate(this.page, {
      testId: 'search-type-sale-button',
      role: 'button',
      name: /^buy$|predaj/i,
    });
  }

  /** Rent transaction toggle. */
  rentToggle(): Locator {
    return locate(this.page, {
      testId: 'search-type-rent-button',
      role: 'button',
      name: /^rent$|prenáj/i,
    });
  }

  /** A "popular city" quick-link by visible name (e.g. `Bratislava`). */
  quickCityLink(city: string): Locator {
    return locate(this.page, { role: 'button', name: city, text: city });
  }

  /**
   * Drive the hero search: optionally type a query, then submit. Waits for the
   * resulting navigation to the listings route — explicit URL wait, no timeout.
   */
  async search(query?: string): Promise<void> {
    if (query) {
      await this.searchInput().fill(query);
    }
    await this.searchButton().click();
    await this.page.waitForURL(/\/listings(\?|$|\/)/);
  }
}
