/// <reference types="vitest/globals" />
/**
 * Regression test for the Sentiment dashboard query-error state (Epic 13 — 13.2).
 *
 * Before the fix the route destructured only `data`/`isLoading` from
 * `useSentimentDashboard()` and swallowed `isError`, so a failed dashboard
 * query left the KPI cards stuck on their placeholder `—`/`N/A` forever with
 * no user-visible error and no retry affordance (silent failure). This mirrors
 * the sibling IoT dashboard convention (`IotDashboardPage` query-error state):
 * `isError` renders a page-level error panel (role="alert") with a Retry
 * button that invokes `onRetry`, replacing the KPI rollup + panels.
 *
 * Presentational component — no network/context mocks needed; i18n is the real
 * English bundle from the shared test setup.
 */

import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { SentimentDashboardPage, type SentimentDashboardPageProps } from './SentimentDashboardPage';

function renderPage(overrides: Partial<SentimentDashboardPageProps> = {}) {
  const props: SentimentDashboardPageProps = {
    dashboard: undefined,
    dashboardLoading: false,
    trends: [],
    trendsLoading: false,
    alerts: [],
    alertsLoading: false,
    pendingAcknowledgeId: null,
    onAcknowledge: vi.fn(),
    ...overrides,
  };
  return { props, ...render(<SentimentDashboardPage {...props} />) };
}

describe('SentimentDashboardPage query-error state', () => {
  it('surfaces an error panel with a retry affordance when the dashboard query failed (isError)', () => {
    const onRetry = vi.fn();
    renderPage({ isError: true, onRetry });

    // The swallowed query error must now be user-visible, not an endless
    // placeholder-"—"/"N/A" dashboard.
    const panel = screen.getByRole('alert');
    expect(panel).toHaveTextContent(/failed to load/i);

    const retry = screen.getByRole('button', { name: /retry/i });
    retry.click();
    expect(onRetry).toHaveBeenCalledTimes(1);

    // The KPI rollup is replaced by the error state, so the stat cards are not
    // rendered underneath it.
    expect(screen.queryByText(/org avg/i)).toBeNull();
  });

  it('renders the dashboard normally when there is no query error', () => {
    renderPage({ isError: false });

    expect(screen.queryByRole('alert')).toBeNull();
    expect(screen.getByText(/org avg/i)).toBeInTheDocument();
  });
});
