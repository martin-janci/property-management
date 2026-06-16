/**
 * ComparePage — property comparison tool (`reality-compare`, public).
 *
 * Shows a side-by-side comparison table of the listings the user has queued, or
 * an empty state inviting them to add listings. Both surfaces are exposed so
 * specs can assert either branch deterministically. Selectors are testid-first
 * with role/text fallback via {@link locate}.
 */

import type { Locator, Page } from '@playwright/test';
import { BasePage, type GotoOptions, locate, sitemapPage } from '@ppt/e2e';
import { APP, type RealityLocale, withLocale } from './support';

export class ComparePage extends BasePage {
  private readonly resolvedPath: string;

  constructor(page: Page, locale?: RealityLocale) {
    super(page);
    this.resolvedPath = withLocale(sitemapPage(page, APP, 'reality-compare').path, locale);
  }

  get path(): string {
    return this.resolvedPath;
  }

  override async goto(options?: GotoOptions): Promise<void> {
    await super.goto(options);
    await this.ready().waitFor({ state: 'visible' });
  }

  /** Settled state: the comparison table or the empty state. */
  ready(): Locator {
    return this.table().or(this.emptyState());
  }

  /** Page heading ("Compare Properties"). */
  pageTitle(): Locator {
    return this.main().getByRole('heading', { level: 1, name: /compar|porovn/i });
  }

  /** The comparison table/grid container. */
  table(): Locator {
    return locate(this.main(), {
      testId: 'compare-table',
      role: 'table',
      css: '[data-testid="compare-table"]',
    });
  }

  /** Columns in the comparison table (one per queued listing). */
  columns(): Locator {
    return locate(this.main(), {
      testId: 'compare-column',
      css: '[data-testid="compare-column"]',
    });
  }

  /** Empty-state heading shown when nothing has been queued for comparison. */
  emptyState(): Locator {
    return locate(this.main(), {
      testId: 'compare-empty-state',
      role: 'heading',
      text: /no.+to compare|nothing to compare|žiadne/i,
    });
  }

  /** How many listings are currently queued for comparison. */
  async count(): Promise<number> {
    return this.columns().count();
  }
}
