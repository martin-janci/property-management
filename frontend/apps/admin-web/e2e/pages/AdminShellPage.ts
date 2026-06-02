/**
 * Base for every authenticated admin-web page.
 *
 * All authed routes render inside `AdminLayout` (sidebar nav + topbar +
 * `<Outlet/>` in a `<main>` landmark — see
 * `apps/admin-web/src/components/AdminLayout.tsx`). This base encapsulates the
 * shared shell chrome — sidebar links, sign-out — so individual page objects
 * only model their own content.
 *
 * Selectors are testid-first (the convention ids resolve once the component
 * sweep merges) with a resilient role/text fallback via the framework `locate`
 * helper, so the suite never hard-crashes on an id that is not in the checkout
 * yet. Sidebar items are capability-gated, so link presence is a function of
 * the signed-in principal's capabilities.
 */

import type { Locator } from '@playwright/test';
import { BasePage, locate } from '@ppt/e2e';

export abstract class AdminShellPage extends BasePage {
  /** Sidebar (the `<nav>` inside the `<aside>` shell). */
  sidebar(): Locator {
    return locate(this.page, { testId: 'admin-sidebar-nav', role: 'navigation' });
  }

  /** A sidebar nav link by its visible label (capability-gated). */
  navLink(name: string | RegExp): Locator {
    return locate(this.sidebar(), { role: 'link', name });
  }

  /**
   * Sign-out button (rendered in both sidebar and topbar). The `.first()` baked
   * into `locate` keeps this a single deterministic control.
   */
  signOutButton(): Locator {
    return locate(this.page, {
      testId: 'admin-signout-button',
      role: 'button',
      name: /sign out/i,
    });
  }

  /** Contextual-help toggle in the topbar. */
  helpToggle(): Locator {
    return locate(this.page, {
      testId: 'admin-help-toggle-button',
      role: 'button',
      name: /open contextual help/i,
    });
  }

  /** Navigate to a sibling admin route via the sidebar (not a raw URL). */
  async navigateVia(name: string | RegExp): Promise<void> {
    await this.navLink(name).click();
  }
}
