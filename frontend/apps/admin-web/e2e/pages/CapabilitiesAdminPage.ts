/**
 * Capabilities admin page object (`/identity/capabilities`).
 *
 * Models `apps/admin-web/src/pages/CapabilitiesAdminPage.tsx`, which has two
 * views driven by the `?view=` query param:
 *   - registry (default): an `<h1>` "Capabilities registry" + a card grid of
 *     every platform capability string.
 *   - per-user (`?view=user&id=`): an `<h1>` "Capabilities · <email>", a grant
 *     table, and a "+ Grant capability" button that opens a `role="dialog"`
 *     grant drawer ("Grant capability").
 *
 * Selectors are testid-first with a role/text fallback via `locate`.
 */

import type { Locator } from '@playwright/test';
import { locate } from '@ppt/e2e';
import { AdminShellPage } from './AdminShellPage';
import { ADMIN_ROUTES } from './routes';

export class CapabilitiesAdminPage extends AdminShellPage {
  get path(): string {
    return ADMIN_ROUTES.capabilities;
  }

  /** Registry-view heading ("Capabilities registry"). */
  registryTitle(): Locator {
    return locate(this.page, {
      testId: 'admin-capabilities-registry-title',
      role: 'heading',
      name: /capabilities registry/i,
    });
  }

  /** The capability card grid (registry view). */
  capabilityGrid(): Locator {
    return locate(this.page, { testId: 'admin-capabilities-grid', css: '.cap-grid' });
  }

  /** A single capability card located by its capability string. */
  capabilityCard(capability: string): Locator {
    return locate(this.main(), {
      testId: `admin-capabilities-card-${capability}`,
      text: capability,
    });
  }

  /** Per-user heading ("Capabilities · <email>"). */
  userTitle(): Locator {
    return locate(this.page, {
      testId: 'admin-capabilities-user-title',
      role: 'heading',
      name: /capabilities ·/i,
    });
  }

  /** "+ Grant capability" trigger (per-user view, gated on memberships_grant). */
  grantButton(): Locator {
    return locate(this.page, {
      testId: 'admin-capabilities-grant-button',
      role: 'button',
      name: /grant capability/i,
    });
  }

  /** The grant drawer dialog opened by {@link grantButton}. */
  grantDrawer(): Locator {
    return locate(this.page, {
      testId: 'admin-capabilities-grant-drawer-overlay',
      role: 'dialog',
      name: /grant capability/i,
    });
  }

  /** Close control inside the grant drawer. */
  grantDrawerClose(): Locator {
    return locate(this.grantDrawer(), {
      testId: 'admin-capabilities-grant-close-button',
      role: 'button',
      name: /close drawer/i,
    });
  }

  /** "Try again" retry button shown when the registry fails to load. */
  retryButton(): Locator {
    return locate(this.page, {
      testId: 'admin-capabilities-retry-button',
      role: 'button',
      name: /try again/i,
    });
  }
}
