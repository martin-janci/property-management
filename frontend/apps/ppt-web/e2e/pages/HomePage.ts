/**
 * ppt-web home page object (`ppt-home`, public `/`).
 *
 * Wraps the sitemap-resolved `/` route and exposes the primary-nav links the
 * legacy `home.spec.ts` / `navigation.spec.ts` asserted on — all funnelled
 * through the framework {@link Nav} component, so no raw `getByRole('link')`
 * strings leak into specs.
 */

import type { Locator, Page } from '@playwright/test';
import { SitemapPage, testIds } from '@ppt/e2e';
import { PPT_HOME_ROUTE_ID, PPT_WEB_APP } from './constants';

export class HomePage extends SitemapPage {
  constructor(page: Page) {
    super(page, PPT_WEB_APP, PPT_HOME_ROUTE_ID);
  }

  /** A primary-nav link by its accessible name. */
  navLink(name: RegExp): Locator {
    return this.nav().link(name);
  }

  homeLink(): Locator {
    return this.navLink(/home/i);
  }

  documentsLink(): Locator {
    return this.navLink(/documents/i);
  }

  newsLink(): Locator {
    return this.navLink(/news/i);
  }

  emergencyLink(): Locator {
    return this.navLink(/emergency/i);
  }

  disputesLink(): Locator {
    return this.navLink(/disputes/i);
  }

  accessibilityLink(): Locator {
    return this.navLink(/accessibility/i);
  }

  privacyLink(): Locator {
    return this.navLink(/privacy/i);
  }

  /**
   * Logout control. Resolves the canonical `nav-logout-button` testid first
   * (and the alternate `logout-btn`), falling back to an accessible-name match
   * — so it works whether the control sits in the nav or behind a user menu.
   */
  logoutButton(): Locator {
    return this.locators()
      .testId(testIds.nav.logout)
      .or(this.locators().testId(testIds.auth.logoutButton))
      .or(this.locators().button(/logout|sign out/i))
      .first();
  }

  /** "Skip to main content" landmark link (may be visually hidden until focus). */
  skipToContentLink(): Locator {
    return this.locators().link(/skip to main content/i);
  }

  /** Language switcher combobox/control. */
  languageControl(): Locator {
    return this.language().root;
  }

  /** Connection-status indicator. */
  connectionIndicator(): Locator {
    return this.connection().root;
  }

  /** Offline indicator (should be absent when online). */
  offlineIndicator(): Locator {
    return this.locators()
      .testId('offline-indicator')
      .or(this.page.locator('[class*="offline-indicator"]'))
      .first();
  }
}
