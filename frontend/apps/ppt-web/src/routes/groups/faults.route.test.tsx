/**
 * Faults route group — FaultsPageRoute API wiring tests.
 *
 * Verifies that the route wrapper correctly threads the live @ppt/api-client
 * hooks into the FaultsPage component — specifically:
 *  (a) loading state propagates from useFaults → "no faults found" absent
 *  (b) error state propagates → isError alert rendered + retry button calls refetch
 *  (c) successful data (snake_case API shapes) is mapped to camelCase UI props
 *      (fault title reaches FaultCard)
 *  (d) empty list → empty state rendered
 *
 * Uses MemoryRouter + Routes + TanStack QueryClientProvider so the
 * route component renders exactly as in production.
 */

/// <reference types="vitest/globals" />
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Suspense } from 'react';
import { MemoryRouter, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// ── i18n ──
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: { defaultValue?: string }) => opts?.defaultValue ?? key,
  }),
}));

// ── toast ──
const mockShowToast = vi.fn();
vi.mock('../../components', () => ({
  useToast: () => ({ showToast: mockShowToast }),
}));

// ── auth ──
vi.mock('../../contexts', () => ({
  useAuth: () => ({ user: { role: 'manager' } }),
}));

// ── api-client stubs ──
const mockUseFaults = vi.fn();
vi.mock('@ppt/api-client', () => ({
  useFaults: (...args: unknown[]) => mockUseFaults(...args),
  useBuildings: () => ({ data: undefined }),
  useCreateFault: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useFault: () => ({ data: undefined, isLoading: true, error: null }),
  useTriageFault: () => ({ mutate: vi.fn() }),
  useResolveFault: () => ({ mutate: vi.fn() }),
  useConfirmFault: () => ({ mutate: vi.fn() }),
  useReopenFault: () => ({ mutate: vi.fn() }),
  useAddComment: () => ({ mutate: vi.fn() }),
  useAddAttachment: () => ({ mutate: vi.fn() }),
  useDeleteAttachment: () => ({ mutate: vi.fn() }),
  useUpdateFault: () => ({ mutateAsync: vi.fn(), isPending: false }),
  faultKeys: {
    all: ['faults'],
    lists: () => ['faults', 'list'],
    list: (q: unknown) => ['faults', 'list', q],
    details: () => ['faults', 'detail'],
    detail: (id: string) => ['faults', 'detail', id],
  },
}));

// Import the route group AFTER mocks are in place.
import { faultRoutes } from './faults';

const noopRefetch = vi.fn().mockResolvedValue(undefined);

function renderFaultsRoute() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter initialEntries={['/faults']}>
        <Suspense fallback={<div>loading…</div>}>
          <Routes>{faultRoutes()}</Routes>
        </Suspense>
      </MemoryRouter>
    </QueryClientProvider>
  );
}

describe('FaultsPageRoute API wiring', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('(a) does not show empty state while loading', async () => {
    mockUseFaults.mockReturnValue({
      data: undefined,
      isLoading: true,
      error: null,
      refetch: noopRefetch,
    });

    renderFaultsRoute();
    // Wait for Suspense to resolve lazy-loaded component.
    // When isLoading=true the FaultList renders a spinner and neither the empty
    // state nor the error text appears. We wait for the Suspense fallback to
    // disappear (the "loading…" span from our <Suspense fallback> goes away once
    // the lazy chunk resolves), then assert the empty state is absent.
    // Use findByRole with a known stable element: the "Report Fault" button that
    // FaultList always renders regardless of loading/error.
    await screen.findByRole('button', { name: 'Report Fault' }, { timeout: 3000 });

    expect(screen.queryByText('No faults found.')).not.toBeInTheDocument();
  });

  it('(b) shows error alert when useFaults errors, retry button calls refetch', async () => {
    const err = new Error('network error');
    mockUseFaults.mockReturnValue({
      data: undefined,
      isLoading: false,
      error: err,
      refetch: noopRefetch,
    });

    renderFaultsRoute();

    const alert = await screen.findByRole('alert', {}, { timeout: 5000 });
    expect(alert).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Retry' }));
    expect(noopRefetch).toHaveBeenCalledTimes(1);
  });

  it('(c) renders fault title when useFaults returns data', async () => {
    mockUseFaults.mockReturnValue({
      data: {
        faults: [
          {
            id: 'f-1',
            building_id: 'b-1',
            unit_id: null,
            title: 'Dripping tap in unit 3B',
            category: 'plumbing',
            priority: 'medium',
            status: 'new',
            created_at: '2026-06-01T00:00:00Z',
          },
        ],
        count: 1,
      },
      isLoading: false,
      error: null,
      refetch: noopRefetch,
    });

    renderFaultsRoute();

    expect(
      await screen.findByText('Dripping tap in unit 3B', {}, { timeout: 5000 })
    ).toBeInTheDocument();
  });

  it('(d) shows empty state when list is empty and no error', async () => {
    mockUseFaults.mockReturnValue({
      data: { faults: [], count: 0 },
      isLoading: false,
      error: null,
      refetch: noopRefetch,
    });

    renderFaultsRoute();

    expect(await screen.findByText('No faults found.', {}, { timeout: 5000 })).toBeInTheDocument();
  });
});
