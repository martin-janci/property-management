/// <reference types="vitest/globals" />
/**
 * Regression test for #3425 (follow-up to PR #3413).
 *
 * When the primary dashboard-summary query fails, the page's error state used
 * to render the raw `error.message` verbatim — an untranslated, often
 * technical server string ("permission denied: RLS policy rejected",
 * "HTTP 500") leaked straight into every locale. This test drives the page
 * with a failing `useDashboardSummary` and asserts the error body is routed
 * through the i18n-backed `getErrorMessage` helper (the same convention the
 * page's mutation `onError` handlers use) rather than echoing `error.message`.
 */

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import type React from 'react';
import { describe, expect, it, vi } from 'vitest';
import { getErrorMessage } from '../../../lib/api';
import { PortfolioDashboardPage } from './PortfolioDashboardPage';

// A raw server/client error whose `.message` must NOT reach the UI. The 403
// status routes `getErrorMessage` to the translated `errors.catalogue`
// "access denied" copy, which is what the user should actually see.
const RAW_MESSAGE = 'permission denied: RLS policy rejected relation portfolio_summary';
const loadError = Object.assign(new Error(RAW_MESSAGE), { status: 403 });

const idleMutation = { mutate: vi.fn(), isPending: false } as const;
const idleQuery = { data: undefined, isLoading: false, isError: false } as const;

vi.mock('@ppt/api-client', () => ({
  useDashboardSummary: () => ({
    data: undefined,
    isLoading: false,
    isError: true,
    error: loadError,
    refetch: vi.fn(),
  }),
  usePropertyPerformanceCards: () => idleQuery,
  useCashFlowTrend: () => idleQuery,
  usePortfolioAlerts: () => ({ ...idleQuery, data: [] }),
  usePortfolioMetricsSummary: () => idleQuery,
  useMarkAlertRead: () => idleMutation,
  useResolveAlert: () => idleMutation,
  useCalculateMetrics: () => idleMutation,
}));

vi.mock('../../../components', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../../components')>()),
  useToast: () => ({ showToast: vi.fn() }),
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

describe('PortfolioDashboardPage load-error state', () => {
  it('renders the translated error copy, never the raw error.message', () => {
    renderPage();

    // The i18n-backed helper produces a translated, status-derived message.
    const expected = getErrorMessage(loadError);
    expect(expected).not.toBe(RAW_MESSAGE); // sanity: helper actually translates
    expect(screen.getByText(expected)).toBeInTheDocument();

    // The raw server string must not leak into any locale.
    expect(screen.queryByText(RAW_MESSAGE)).toBeNull();
    expect(screen.queryByText(/RLS policy/)).toBeNull();
  });
});
