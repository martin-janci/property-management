/**
 * FavoritesPage — the user's saved listings (`reality-favorites`, auth-gated).
 *
 * Shows either a grid of favorited listing cards or a "no favorites yet" empty
 * state. Both surfaces are exposed so specs can assert either branch without a
 * fixed wait. Selectors are testid-first (`listing-card`, `favorite-btn` are
 * verified) with role/text fallback via {@link locate}.
 *
 * reality-web's protected pages render an in-page "Sign in required" guard for
 * unauthenticated visitors (rather than redirecting), so the guard is exposed
 * too — letting unauthenticated specs assert it without a session or backend.
 */

import type { Locator, Page } from '@playwright/test';
import { BasePage, type GotoOptions, locate, sitemapPage } from '@ppt/e2e';
import { APP, type RealityLocale, withLocale } from './support';

export class FavoritesPage extends BasePage {
  private readonly resolvedPath: string;

  constructor(page: Page, locale?: RealityLocale) {
    super(page);
    this.resolvedPath = withLocale(sitemapPage(page, APP, 'reality-favorites').path, locale);
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

  /** Page heading ("My Favorites") — the level-1 title, not the empty-state h2. */
  pageTitle(): Locator {
    return this.main().getByRole('heading', { level: 1, name: /favorit|obľúb/i });
  }

  /** Either a favorited listing card or the empty-state heading. */
  content(): Locator {
    return this.favoriteCards().first().or(this.emptyState());
  }

  /** All favorited listing cards — verified `listing-card` testid, link fallback. */
  favoriteCards(): Locator {
    return locate(this.main(), {
      testId: 'listing-card',
      css: '[data-testid="listing-card"]',
    }).or(
      this.main()
        .getByRole('link', { name: /.+/ })
        .filter({ has: this.page.getByRole('button', { name: /favorit|obľúb/i }) })
    );
  }

  /** Favorite toggle inside the nth card — verified `favorite-btn` testid. */
  favoriteButton(index = 0): Locator {
    return locate(this.favoriteCards().nth(index), {
      testId: 'favorite-btn',
      role: 'button',
      name: /favorit|obľúb/i,
    });
  }

  /** Empty-state heading shown when there are no favorites. */
  emptyState(): Locator {
    return locate(this.main(), {
      testId: 'favorites-empty-state',
      role: 'heading',
      text: /no favorites yet|žiadne/i,
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

  /** How many favorites are currently rendered. */
  async count(): Promise<number> {
    return this.favoriteCards().count();
  }
}
