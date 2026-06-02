/**
 * ListingsPage — search results with filters (`reality-listings`).
 *
 * Encapsulates the search bar, the filter controls, the results grid, and the
 * listing cards (each card is a link to its detail page with an embedded
 * favorite toggle). Selectors are testid-first against the verified reality ids
 * (`listing-grid`, `listing-card`, `favorite-btn`, `search-form`) with a
 * resilient role/text fallback via {@link locate}.
 */

import type { Locator, Page } from '@playwright/test';
import { BasePage, type GotoOptions, locate, sitemapPage, tidFilter } from '@ppt/e2e';
import { APP, type RealityLocale, withLocale } from './support';

export interface ListingsQuery {
  readonly type?: 'sale' | 'rent';
  readonly city?: string;
  readonly price_max?: number;
  readonly q?: string;
}

/** Drop undefined entries and widen to the query Record the sitemap expects. */
function toQueryRecord(
  query?: ListingsQuery
): Record<string, string | number | boolean> | undefined {
  if (!query) {
    return undefined;
  }
  const record: Record<string, string | number | boolean> = {};
  for (const [key, value] of Object.entries(query)) {
    if (value !== undefined) {
      record[key] = value;
    }
  }
  return record;
}

export class ListingsPage extends BasePage {
  private readonly resolvedPath: string;

  constructor(page: Page, opts: { query?: ListingsQuery; locale?: RealityLocale } = {}) {
    super(page);
    const query = toQueryRecord(opts.query);
    const base = sitemapPage(page, APP, 'reality-listings', { query }).path;
    this.resolvedPath = withLocale(base, opts.locale);
  }

  get path(): string {
    return this.resolvedPath;
  }

  override async goto(options?: GotoOptions): Promise<void> {
    await super.goto(options);
    await this.results().waitFor({ state: 'visible' });
  }

  /** The in-page search form — verified `search-form` testid. */
  searchForm(): Locator {
    return locate(this.page, { testId: 'search-form', role: 'search', css: 'form' });
  }

  /** Free-text search input in the results header. */
  searchInput(): Locator {
    return locate(this.searchForm(), {
      testId: 'search-query-input',
      role: 'searchbox',
      css: 'input[type="search"], input[type="text"]',
    });
  }

  /** Submit button for the in-page search bar. */
  searchButton(): Locator {
    return locate(this.searchForm(), {
      testId: 'search-submit-button',
      role: 'button',
      name: /^search$|hľad/i,
    });
  }

  /** A filter `<select>` by name (e.g. `city`, `type`, `sort`). */
  filterSelect(name: string): Locator {
    return locate(this.page, {
      testId: tidFilter('listings', name, 'select'),
      role: 'combobox',
      css: `select[name="${name}"]`,
    });
  }

  /** The results grid container — verified `listing-grid` testid. */
  grid(): Locator {
    return locate(this.page, {
      testId: 'listing-grid',
      role: 'list',
      css: '[data-testid="listing-grid"]',
    });
  }

  /**
   * The results region — either the populated grid (which contains listing
   * links) or the "no results" empty state heading. Resolves to whichever is
   * present so `goto` can wait deterministically without a fixed timeout.
   */
  results(): Locator {
    return this.cards().first().or(this.emptyState());
  }

  /** Empty-state heading shown when a search returns nothing. */
  emptyState(): Locator {
    return locate(this.main(), {
      testId: 'listings-empty-state',
      role: 'heading',
      text: /no listings found|žiadne/i,
    });
  }

  /** All listing cards — verified `listing-card` testid, link fallback. */
  cards(): Locator {
    return locate(this.main(), {
      testId: 'listing-card',
      css: '[data-testid="listing-card"]',
    }).or(
      this.main()
        .getByRole('link', { name: /.+/ })
        .filter({ has: this.page.getByRole('button', { name: /favorit|obľúb/i }) })
    );
  }

  /** The nth listing card (0-based). */
  card(index = 0): Locator {
    return this.cards().nth(index);
  }

  /** Favorite toggle inside the nth listing card — verified `favorite-btn`. */
  favoriteButton(index = 0): Locator {
    return locate(this.card(index), {
      testId: 'favorite-btn',
      role: 'button',
      name: /favorit|obľúb/i,
    });
  }

  /** Whether the nth card is currently marked as a favorite. */
  async isFavorited(index = 0): Promise<boolean> {
    const button = this.favoriteButton(index);
    const favorited = await button.getAttribute('data-favorited');
    if (favorited !== null) {
      return favorited === 'true';
    }
    const label = await button.getAttribute('aria-label');
    return /remove from favorites|odstrániť/i.test(label ?? '');
  }

  /** Open the nth listing's detail page and wait for the URL to settle. */
  async openCard(index = 0): Promise<void> {
    await this.card(index).click();
    await this.page.waitForURL(/\/listings\/[^/?#]+/);
  }

  /** Run an in-page free-text search and wait for the URL to reflect it. */
  async search(query: string): Promise<void> {
    await this.searchInput().fill(query);
    await this.searchButton().click();
    await this.page.waitForURL(/\/listings/);
  }
}
