/**
 * SavedSearchesPage — stored search queries (`reality-saved-searches`, auth).
 *
 * Shows a list of saved-search cards (each titled by its name) or an empty
 * state. Selectors are testid-first with role/text fallback via {@link locate}
 * so flow specs read declaratively. Exposes the in-page "Sign in required"
 * guard so unauthenticated specs can assert it without a session.
 */

import type { Locator, Page } from '@playwright/test';
import { BasePage, type GotoOptions, locate, sitemapPage } from '@ppt/e2e';
import { APP, type RealityLocale, withLocale } from './support';

export class SavedSearchesPage extends BasePage {
  private readonly resolvedPath: string;

  constructor(page: Page, locale?: RealityLocale) {
    super(page);
    this.resolvedPath = withLocale(sitemapPage(page, APP, 'reality-saved-searches').path, locale);
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

  /** Page heading ("Saved Searches") — the level-1 title, always rendered. */
  pageTitle(): Locator {
    return this.main().getByRole('heading', { level: 1, name: /saved searches|uložené/i });
  }

  /** All saved-search cards. */
  cards(): Locator {
    return locate(this.main(), {
      testId: 'saved-search-card',
      css: '[data-testid="saved-search-card"]',
    });
  }

  /** A saved search by its display name. */
  searchByName(name: string | RegExp): Locator {
    return this.locators().heading(name);
  }

  /** Empty-state heading shown when no searches are saved. */
  emptyState(): Locator {
    return locate(this.main(), {
      testId: 'saved-searches-empty-state',
      role: 'heading',
      text: /no saved searches|žiadne/i,
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
}
