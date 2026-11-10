/// <reference types="vitest/globals" />
/**
 * Regression test for the portfolio alert mutations silently swallowing
 * failures.
 *
 * Before the fix, `markRead.mutate()` / `resolve.mutate()` were fired with no
 * `onError`, so a failed POST produced no user-visible feedback. These tests
 * drive the real page (with real `useMutation`-backed alert mutations whose
 * network call rejects) and assert that an error toast is surfaced — matching
 * the community/faults `onError → showToast({ type: 'error' })` convention.
 */

import { QueryClient, QueryClientProvider, useMutation } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type React from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { PortfolioDashboardPage } from './PortfolioDashboardPage';

const h = vi.hoisted(() => ({ showToast: vi.fn() }));

const unreadAlert = {
  id: 'alert-1',
  portfolio_id: 'pf-1',
  property_id: null,
  alert_type: 'occupancy_drop',
  severity: 'high',
  title: 'Occupancy dropped',
  message: 'Occupancy fell below threshold',
  metric_name: 'occupancy_rate',
  current_value: '0.80',
  threshold_value: '0.90',
  is_read: false,
  is_resolved: false,
  created_at: '2026-01-01T00:00:00Z',
};

const summary = {
  total_portfolio_value: '1000000',
  total_equity: '400000',
  total_debt: '600000',
  debt_to_equity_ratio: '1.5',
  ltv_ratio: '0.6',
  ytd_noi: '50000',
  ytd_cash_flow: '20000',
  ytd_return_pct: '0.05',
  property_count: 3,
  total_units: 30,
  occupied_units: 24,
  occupancy_rate: '0.8',
  currency: 'EUR',
  as_of_date: '2026-01-01',
};

// Query hooks return static data so the dashboard renders one unread alert;
// the two alert mutation hooks use a REAL `useMutation` whose network call
// rejects, so the page's own `onError` wiring (the thing under test) is what
// decides whether the failure reaches the user.
vi.mock('@ppt/api-client', () => ({
  useDashboardSummary: () => ({ data: summary, isLoading: false, isError: false }),
  usePropertyPerformanceCards: () => ({ data: [], isLoading: false, isError: false }),
  useCashFlowTrend: () => ({ data: [], isLoading: false, isError: false }),
  usePortfolioAlerts: () => ({ data: [unreadAlert], isLoading: false, isError: false }),
  usePortfolioMetricsSummary: () => ({
    data: { property_metrics: [] },
    isLoading: false,
    isError: false,
  }),
  useMarkAlertRead: () =>
    useMutation({ mutationFn: () => Promise.reject(new Error('mark read boom')) }),
  useResolveAlert: () =>
    useMutation({ mutationFn: () => Promise.reject(new Error('resolve boom')) }),
}));

vi.mock('../../../components', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../../components')>()),
  useToast: () => ({ showToast: h.showToast }),
}));

function renderPage() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  const Wrapper: React.FC = () => (
    <QueryClientProvider client={queryClient}>
      <PortfolioDashboardPage portfolioId="pf-1" />
    </QueryClientProvider>
  );
  return render(<Wrapper />);
}

beforeEach(() => {
  h.showToast.mockReset();
});

describe('PortfolioDashboardPage alert mutations', () => {
  it('surfaces an error toast when marking an alert read fails', async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole('button', { name: 'Mark Read' }));

    await waitFor(() =>
      expect(h.showToast).toHaveBeenCalledWith(expect.objectContaining({ type: 'error' }))
    );
  });

  it('surfaces an error toast when resolving an alert fails', async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole('button', { name: 'Resolve' }));

    await waitFor(() =>
      expect(h.showToast).toHaveBeenCalledWith(expect.objectContaining({ type: 'error' }))
    );
  });
});
