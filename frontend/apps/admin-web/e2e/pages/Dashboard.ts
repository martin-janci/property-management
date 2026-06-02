/**
 * admin-web Dashboard (`/`) page object.
 *
 * Models the overview screen from `apps/admin-web/src/pages/Dashboard.tsx`:
 * a time-of-day greeting `<h1>`, a 4-stat tile row, and capability-gated cards
 * (recent high-risk events, active impersonation, pending merge collisions,
 * quick actions). Stat-tile labels and card headers are literal English strings
 * in the component, so they are locale-stable selectors.
 *
 * Selectors are testid-first with a role/text fallback via `locate`.
 */

import type { Locator } from '@playwright/test';
import { locate } from '@ppt/e2e';
import { AdminShellPage } from './AdminShellPage';
import { ADMIN_ROUTES } from './routes';

/** The four overview stat tiles, keyed by their literal label. */
export type DashboardStat =
  | 'Tenants active'
  | 'Operators online'
  | 'Pending merges'
  | 'High-risk · 24h';

export class Dashboard extends AdminShellPage {
  get path(): string {
    return ADMIN_ROUTES.dashboard;
  }

  /** Greeting heading, e.g. "Good morning, Operator." (the page `<h1>`). */
  greeting(): Locator {
    return locate(this.page, {
      testId: 'admin-dashboard-greeting',
      role: 'heading',
      name: /good (morning|afternoon|evening)/i,
    });
  }

  /** Attention / MFA-window subline under the greeting. */
  subline(): Locator {
    return locate(this.page, {
      testId: 'admin-dashboard-subline',
      text: /(needs? attention|everything looks clear)/i,
    });
  }

  /** A stat tile located by its (literal) label text, scoped to `<main>`. */
  statTile(label: DashboardStat): Locator {
    return locate(this.main(), {
      testId: `admin-dashboard-stat-${label.replace(/\s+/g, '-').toLowerCase()}`,
      text: label,
    });
  }

  /** "Recent high-risk events · 24h" card header (gated on audit_read). */
  highRiskEventsCard(): Locator {
    return locate(this.page, {
      testId: 'admin-dashboard-high-risk-card',
      role: 'heading',
      name: /recent high-risk events/i,
    });
  }

  /** "Quick actions" card header. */
  quickActionsCard(): Locator {
    return locate(this.page, {
      testId: 'admin-dashboard-quick-actions-card',
      role: 'heading',
      name: /^quick actions$/i,
    });
  }

  /** Quick-action button to open the audit log. */
  openAuditLogAction(): Locator {
    return locate(this.page, {
      testId: 'admin-dashboard-open-audit-button',
      role: 'button',
      name: /open audit log/i,
    });
  }

  /** Quick-action button to jump to user search. */
  findUserAction(): Locator {
    return locate(this.page, {
      testId: 'admin-dashboard-find-user-button',
      role: 'button',
      name: /find user/i,
    });
  }
}
