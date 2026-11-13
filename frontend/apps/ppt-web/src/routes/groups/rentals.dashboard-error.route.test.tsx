/// <reference types="vitest/globals" />
/**
 * Rentals dashboard fetch-error regression (code-review finding
 * code-review-ppt-web-core-route-query-silent-error).
 *
 * `RentalsDashboardPageRoute` destructured only `{ data, isLoading }` from its
 * listRentalBookings / getRentalStatistics / listRentalConnections queries and
 * dropped `isError`/`error`. On a failed query `data` is undefined, so the stats
 * mapped to all-zero (totalBookings 0, occupancy 0, …) with `isLoading=false` —
 * a genuine fetch failure was indistinguishable from an organization that
 * legitimately has no rentals, i.e. the error was swallowed.
 *
 * The fix threads the query error into `RentalsDashboardPage` so it renders a
 * dedicated error state (message + retry) instead of all-zero stats. These tests
 * pin that contract:
 *
 *  - on a query failure the error state is shown (fails on `main`, where the
 *    all-zero dashboard leaks through with no error affordance);
 *  - the retry button re-issues the failed queries;
 *  - a successful (empty) result does NOT show the error state.
 */
import { getRentalStatistics, listRentalBookings, listRentalConnections } from '@ppt/api-client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ToastProvider } from '../../components';
import '../../i18n';
import en from '../../../messages/en.json';
import { rentalRoutes } from './rentals';

vi.mock('@ppt/api-client', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@ppt/api-client')>()),
  getToken: vi.fn(() => 'tok'),
  listRentalBookings: vi.fn(),
  getRentalStatistics: vi.fn(),
  listRentalConnections: vi.fn(),
}));

vi.mock('../../contexts/AuthContext', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../contexts/AuthContext')>()),
  useAuth: () => ({
    isAuthenticated: true,
    isLoading: false,
    user: { id: 'u1', organizationId: 'org-1', role: 'manager' },
    logout: vi.fn(),
  }),
}));

const LOAD_FAILED = en.rentals.dashboard.loadFailed;
const RETRY = en.rentals.dashboard.retry;

function renderPage() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <ToastProvider>
        <MemoryRouter initialEntries={['/rentals']}>
          <Routes>{rentalRoutes()}</Routes>
        </MemoryRouter>
      </ToastProvider>
    </QueryClientProvider>
  );
}

beforeEach(() => {
  vi.clearAllMocks();
});

function mockAllReject() {
  vi.mocked(listRentalBookings).mockRejectedValue(new Error('boom'));
  vi.mocked(getRentalStatistics).mockRejectedValue(new Error('boom'));
  vi.mocked(listRentalConnections).mockRejectedValue(new Error('boom'));
}

describe('RentalsDashboardPageRoute fetch error state', () => {
  it('shows the error state when a dashboard query fails', async () => {
    mockAllReject();
    renderPage();
    expect(await screen.findByText(LOAD_FAILED)).toBeInTheDocument();
    expect(screen.getByRole('alert')).toBeInTheDocument();
  });

  it('retries the failed queries when retry is clicked', async () => {
    mockAllReject();
    renderPage();
    await screen.findByText(LOAD_FAILED);
    const before = vi.mocked(listRentalBookings).mock.calls.length;
    fireEvent.click(screen.getByRole('button', { name: RETRY }));
    await waitFor(() =>
      expect(vi.mocked(listRentalBookings).mock.calls.length).toBeGreaterThan(before)
    );
  });

  it('does not show the error state on a successful empty result', async () => {
    vi.mocked(listRentalBookings).mockResolvedValue({ bookings: [], total: 0 } as never);
    vi.mocked(getRentalStatistics).mockResolvedValue({} as never);
    vi.mocked(listRentalConnections).mockResolvedValue([] as never);
    renderPage();
    expect(await screen.findByText(en.rentals.dashboard.title)).toBeInTheDocument();
    expect(screen.queryByText(LOAD_FAILED)).not.toBeInTheDocument();
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });
});
