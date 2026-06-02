/**
 * Platform Health page object (`/platform/health`).
 *
 * Models `apps/admin-web/src/pages/PlatformHealthPage.tsx`: an `<h1>` title +
 * Refresh button, current-metric cards (each with a History drill-down dialog),
 * an Alerts table with an active-only / all toggle and Acknowledge action, and
 * a Metric Thresholds table. Section headings use hard-coded English defaults,
 * so they are locale-stable.
 *
 * Selectors are testid-first with a role/text fallback via `locate`.
 */

import type { Locator } from '@playwright/test';
import { locate } from '@ppt/e2e';
import { AdminShellPage } from './AdminShellPage';
import { ADMIN_ROUTES } from './routes';

export class PlatformHealthPage extends AdminShellPage {
  get path(): string {
    return ADMIN_ROUTES.platformHealth;
  }

  /** Page heading ("Platform Health"). */
  title(): Locator {
    return locate(this.page, {
      testId: 'admin-health-title',
      role: 'heading',
      name: /platform health/i,
    });
  }

  /** Refresh button (re-fetches the dashboard). */
  refreshButton(): Locator {
    return locate(this.page, {
      testId: 'admin-health-refresh-button',
      role: 'button',
      name: /refresh/i,
    });
  }

  /** "Current Metrics" section heading (`<h2>`). */
  currentMetricsSection(): Locator {
    return locate(this.page, {
      testId: 'admin-health-metrics-heading',
      role: 'heading',
      name: /current metrics/i,
    });
  }

  /** "Alerts" section heading (`<h2>`). */
  alertsSection(): Locator {
    return locate(this.page, {
      testId: 'admin-health-alerts-heading',
      role: 'heading',
      name: /^alerts$/i,
    });
  }

  /** "Metric Thresholds" section heading (`<h2>`). */
  thresholdsSection(): Locator {
    return locate(this.page, {
      testId: 'admin-health-thresholds-heading',
      role: 'heading',
      name: /metric thresholds/i,
    });
  }

  /** Alerts filter toggle ("Active only" | "All alerts"). */
  alertsToggle(mode: 'active' | 'all'): Locator {
    return locate(this.page, {
      testId: `admin-health-alerts-${mode}-button`,
      role: 'button',
      name: mode === 'active' ? /active only/i : /all alerts/i,
    });
  }

  /** "History" drill-down button on the first metric card. */
  metricHistoryButton(): Locator {
    return locate(this.page, {
      testId: 'admin-health-metric-history-button',
      role: 'button',
      name: /^history$/i,
    });
  }

  /** Metric-history drill-down dialog. */
  metricHistoryDialog(): Locator {
    return locate(this.page, {
      testId: 'admin-health-metric-history-dialog',
      role: 'dialog',
      name: /metric history/i,
    });
  }

  /** Load-error alert (shown when the dashboard fetch fails). */
  errorAlert(): Locator {
    return locate(this.page, { testId: 'admin-health-error-alert', role: 'alert' });
  }
}
