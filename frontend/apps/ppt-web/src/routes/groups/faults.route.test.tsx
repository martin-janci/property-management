/**
 * Faults route group — FaultsPageRoute API wiring tests (gap-79-1).
 *
 * Verifies that the route wrapper correctly threads the live @ppt/api-client
 * hooks into the FaultsPage component — specifically:
 *  (a) loading state propagates from useFaults → "no faults found" absent
 *  (b) error state propagates → isError alert rendered + retry button calls refetch
 *  (c) successful data (snake_case API shapes) is mapped to camelCase UI props
 *      (fault title reaches FaultCard)
 *  (d) empty list → empty state rendered
 *  (e) no Delete action is offered (the API has no DELETE /faults/{id})
 *  (f) useFaultStatistics summary bar rendered when stats are available
 *  (i) create form lists the selected building's units; photo picker hidden
 *
 * Uses MemoryRouter + Routes + TanStack QueryClientProvider so the
 * route component renders exactly as in production.
 */

/// <reference types="vitest/globals" />
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { type ReactNode, Suspense } from 'react';
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
  ProtectedRoute: ({ children }: { children: ReactNode }) => children,
}));

// ── auth ──
vi.mock('../../contexts', () => ({
  useAuth: () => ({ user: { role: 'manager' } }),
}));

// ── api-client stubs ──
const mockUseFaults = vi.fn();
const mockDeleteFaultMutateAsync = vi.fn().mockResolvedValue(undefined);
const mockUseFaultStatistics = vi.fn();
const mockUseBuildings = vi.fn(() => ({ data: undefined }) as unknown);

vi.mock('@ppt/api-client', () => ({
  useFaults: (...args: unknown[]) => mockUseFaults(...args),
  useDeleteFault: () => ({ mutateAsync: mockDeleteFaultMutateAsync }),
  useFaultStatistics: (...args: unknown[]) => mockUseFaultStatistics(...args),
  useBuildings: (...args: unknown[]) => mockUseBuildings(...args),
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

// ── building units (GET /buildings/{id}/units) ──
const mockUseUnits = vi.fn();
vi.mock('../../features/buildings/hooks', () => ({
  useUnits: (...args: unknown[]) => mockUseUnits(...args),
}));

// Import the route group AFTER mocks are in place.
import { faultRoutes } from './faults';

const noopRefetch = vi.fn().mockResolvedValue(undefined);

function renderFaultsRoute(path = '/faults') {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter initialEntries={[path]}>
        <Suspense fallback={<div>loading…</div>}>
          <Routes>{faultRoutes()}</Routes>
        </Suspense>
      </MemoryRouter>
    </QueryClientProvider>
  );
}

/** Minimal API fault summary fixture for reuse. */
function makeApiFault(overrides: Record<string, unknown> = {}) {
  return {
    id: 'f-1',
    building_id: 'b-1',
    unit_id: null,
    title: 'Dripping tap in unit 3B',
    category: 'plumbing',
    priority: 'medium',
    status: 'new',
    created_at: '2026-06-01T00:00:00Z',
    ...overrides,
  };
}

describe('FaultsPageRoute API wiring (gap-79-1)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Default stats: no data (stats bar hidden).
    mockUseFaultStatistics.mockReturnValue({ data: undefined });
    mockUseUnits.mockReturnValue({ data: undefined });
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
      data: { faults: [makeApiFault()], count: 1 },
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

  it('(e) does not offer Delete (no DELETE /faults/{id} on the API)', async () => {
    mockUseFaults.mockReturnValue({
      data: {
        faults: [makeApiFault({ id: 'f-del', title: 'Broken pipe', status: 'new' })],
        count: 1,
      },
      isLoading: false,
      error: null,
      refetch: noopRefetch,
    });

    renderFaultsRoute();
    await screen.findByText('Broken pipe', {}, { timeout: 5000 });

    expect(screen.queryByRole('button', { name: 'Delete' })).not.toBeInTheDocument();
    expect(mockDeleteFaultMutateAsync).not.toHaveBeenCalled();
  });

  it('(f) statistics summary bar renders when useFaultStatistics returns data', async () => {
    mockUseFaults.mockReturnValue({
      data: { faults: [], count: 0 },
      isLoading: false,
      error: null,
      refetch: noopRefetch,
    });
    mockUseFaultStatistics.mockReturnValue({
      data: {
        total_count: 42,
        open_count: 30,
        closed_count: 12,
        by_status: [],
        by_category: [],
        by_priority: [],
        average_resolution_time_hours: null,
        average_rating: null,
      },
    });

    renderFaultsRoute();

    // The stats bar is rendered as a div whose aria-label reads from i18n
    // (t('aria.faultStatistics')). This suite mocks react-i18next so t()
    // echoes the key, so locate the container by that resolved key.
    const statsBar = await screen.findByLabelText('aria.faultStatistics', {}, { timeout: 5000 });
    expect(statsBar).toBeInTheDocument();
    expect(statsBar).toHaveTextContent('42');
    expect(statsBar).toHaveTextContent('30');
    // Closed renders the API's first-class closed_count (not total - open). #1588 F2
    expect(statsBar).toHaveTextContent('12');
  });
});

describe('CreateFaultPageRoute (nav-fixes)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockUseUnits.mockImplementation((buildingId: string) => ({
      data:
        buildingId === 'b-1'
          ? { units: [{ id: 'u-1', designation: '3B' }], total: 1, offset: 0, limit: 100 }
          : undefined,
    }));
  });

  it("(i) lists the selected building's units and hides the photo picker", async () => {
    mockUseBuildings.mockReturnValue({ data: { items: [{ id: 'b-1', name: 'Tower A' }] } });
    renderFaultsRoute('/faults/new');

    const building = await screen.findByLabelText(/faults\.form\.building/, {}, { timeout: 5000 });
    // Before a building is chosen the unit query is disabled.
    expect(mockUseUnits).toHaveBeenLastCalledWith('', { limit: 100 }, false);

    await userEvent.selectOptions(building, 'b-1');

    expect(mockUseUnits).toHaveBeenLastCalledWith('b-1', { limit: 100 }, true);
    expect(await screen.findByRole('option', { name: '3B' })).toBeInTheDocument();
    // Photos could never be persisted (no storage upload flow) → no picker.
    expect(screen.queryByText('faults.photo.sectionTitle')).not.toBeInTheDocument();
  });
});
