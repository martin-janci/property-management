/// <reference types="vitest/globals" />
/**
 * useActionQueue tests.
 *
 * The action-queue backend endpoint does not exist yet, so the hook must:
 *  - return an EMPTY queue (never fabricated data) unless the dev-only mock
 *    gate is active (DEV + dev-panel `mock` mode);
 *  - fail loudly on approve/reject/dismiss instead of a silent fake success;
 *  - serve mock data + simulated mutations only under the mock gate.
 */

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderHook, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import {
  type ActionQueueData,
  applyActionQueueFilters,
  buildManagerQueue,
  isActionQueueAvailable,
  isActionQueueMockEnabled,
  useActionQueue,
} from './useActionQueue';

const api = vi.hoisted(() => ({
  listFaults: vi.fn(),
  listBuildings: vi.fn(),
  listPendingBookings: vi.fn(),
  approveBooking: vi.fn(),
}));
vi.mock('@ppt/api-client', () => api);

const MODE_KEY = 'ppt-dev-panel-mode';

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
  return ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
}

function enableMock() {
  localStorage.setItem(MODE_KEY, 'mock');
}

beforeEach(() => {
  localStorage.clear();
  api.listFaults.mockResolvedValue({ faults: [], count: 0 });
  api.listBuildings.mockResolvedValue({ items: [], total: 0 });
  api.listPendingBookings.mockResolvedValue({ items: [], total: 0 });
  api.approveBooking.mockResolvedValue({});
});

describe('applyActionQueueFilters', () => {
  const data: ActionQueueData = {
    items: [
      {
        id: 'a',
        type: 'fault_pending',
        title: 'Water leak',
        description: 'ceiling',
        priority: 'urgent',
        createdAt: '2026-01-01T00:00:00Z',
        entityId: 'f-1',
        entityType: 'fault',
        actions: [],
      },
      {
        id: 'b',
        type: 'vote_active',
        title: 'Garden vote',
        description: 'renovation',
        priority: 'low',
        createdAt: '2026-01-01T00:00:00Z',
        entityId: 'v-1',
        entityType: 'vote',
        actions: [],
      },
    ],
    total: 2,
    counts: { urgent: 1, high: 0, medium: 0, low: 1 },
  };

  it('filters by type', () => {
    const out = applyActionQueueFilters(data, { types: ['vote_active'] });
    expect(out.items.map((i) => i.id)).toEqual(['b']);
    expect(out.total).toBe(1);
  });

  it('filters by priority', () => {
    const out = applyActionQueueFilters(data, { priorities: ['urgent'] });
    expect(out.items.map((i) => i.id)).toEqual(['a']);
  });

  it('filters by case-insensitive search over title and description', () => {
    expect(applyActionQueueFilters(data, { search: 'GARDEN' }).items.map((i) => i.id)).toEqual([
      'b',
    ]);
    expect(applyActionQueueFilters(data, { search: 'ceiling' }).items.map((i) => i.id)).toEqual([
      'a',
    ]);
  });

  it('returns all items when no filters supplied', () => {
    expect(applyActionQueueFilters(data).items).toHaveLength(2);
  });
});

describe('isActionQueueMockEnabled', () => {
  it('is disabled by default (no dev-panel mock mode set)', () => {
    expect(isActionQueueMockEnabled()).toBe(false);
  });

  it('is enabled under DEV when dev-panel mode is mock', () => {
    enableMock();
    // Tests run under Vite DEV (import.meta.env.DEV === true).
    expect(isActionQueueMockEnabled()).toBe(true);
  });
});

describe('isActionQueueAvailable', () => {
  it('is available for managers (real sources) and not for residents', () => {
    expect(isActionQueueAvailable('manager')).toBe(true);
    expect(isActionQueueAvailable('resident')).toBe(false);
  });
});

describe('buildManagerQueue — composed from real endpoints', () => {
  it('lists new faults and pending bookings, urgent first', async () => {
    api.listFaults.mockResolvedValue({
      faults: [
        {
          id: 'f1',
          title: 'Leak',
          priority: 'urgent',
          building_name: 'A',
          created_at: '2026-11-01T00:00:00Z',
        },
      ],
      count: 1,
    });
    api.listBuildings.mockResolvedValue({ items: [{ id: 'b1' }], total: 1 });
    api.listPendingBookings.mockResolvedValue({
      items: [
        {
          id: 'k1',
          facility_name: 'Gym',
          building_id: 'b1',
          user_name: 'Jana',
          start_time: '2026-11-10T10:00:00Z',
          created_at: '2026-11-02T00:00:00Z',
        },
      ],
      total: 1,
    });

    const queue = await buildManagerQueue();

    expect(api.listFaults).toHaveBeenCalledWith(expect.objectContaining({ status: 'new' }));
    expect(queue.items.map((i) => i.id)).toEqual(['fault:f1', 'booking:k1']);
    expect(queue.items[1].metadata).toEqual({ buildingId: 'b1' });
    expect(queue.counts).toEqual({ urgent: 1, high: 1, medium: 0, low: 0 });
  });

  it('keeps the other source when one fails', async () => {
    api.listFaults.mockRejectedValue(new Error('403'));
    api.listBuildings.mockResolvedValue({ items: [{ id: 'b1' }], total: 1 });
    api.listPendingBookings.mockResolvedValue({
      items: [
        {
          id: 'k1',
          facility_name: 'Gym',
          building_id: 'b1',
          start_time: '2026-11-10T10:00:00Z',
          created_at: '2026-11-02T00:00:00Z',
        },
      ],
      total: 1,
    });
    const queue = await buildManagerQueue();
    expect(queue.items.map((i) => i.id)).toEqual(['booking:k1']);
  });
});

describe('useActionQueue — real sources (mock disabled)', () => {
  it('approves a pending booking inline', async () => {
    const { result } = renderHook(() => useActionQueue('manager'), { wrapper: createWrapper() });
    await waitFor(() => expect(result.current.isLoading).toBe(false));

    result.current.handleAction('booking:k1', 'approve');

    await waitFor(() => expect(api.approveBooking).toHaveBeenCalledWith('k1'));
  });
});

describe('useActionQueue — no backend / mock disabled', () => {
  it('resolves an empty queue instead of fabricated data', async () => {
    const { result } = renderHook(() => useActionQueue('resident'), { wrapper: createWrapper() });

    await waitFor(() => expect(result.current.isLoading).toBe(false));

    expect(result.current.items).toEqual([]);
    expect(result.current.total).toBe(0);
    expect(result.current.stats).toEqual({ total: 0, urgent: 0, high: 0, medium: 0, low: 0 });
  });

  it('fails loudly on an action instead of a silent no-op', async () => {
    const { result } = renderHook(() => useActionQueue('resident'), { wrapper: createWrapper() });
    await waitFor(() => expect(result.current.isLoading).toBe(false));

    result.current.handleAction('some-item', 'approve');

    await waitFor(() => expect(result.current.isExecuteError).toBe(true));
    expect((result.current.executeError as Error).message).toMatch(/not supported/i);
  });
});

describe('useActionQueue — mock enabled (DEV)', () => {
  it('serves mock manager items and computes stats', async () => {
    enableMock();
    const { result } = renderHook(() => useActionQueue('manager'), { wrapper: createWrapper() });

    await waitFor(() => expect(result.current.items.length).toBeGreaterThan(0));
    expect(result.current.total).toBe(result.current.items.length);
    expect(result.current.stats.total).toBe(result.current.items.length);
  });

  it('applies filters to the mock data', async () => {
    enableMock();
    const { result } = renderHook(
      () => useActionQueue('manager', { types: ['approval_pending'] }),
      { wrapper: createWrapper() }
    );

    await waitFor(() => expect(result.current.items.length).toBeGreaterThan(0));
    expect(result.current.items.every((i) => i.type === 'approval_pending')).toBe(true);
  });

  it('executes an action without error under the mock', async () => {
    enableMock();
    const { result } = renderHook(() => useActionQueue('manager'), { wrapper: createWrapper() });
    await waitFor(() => expect(result.current.items.length).toBeGreaterThan(0));

    result.current.handleAction(result.current.items[0].id, 'dismiss');

    await waitFor(() => expect(result.current.isExecuting).toBe(false));
    expect(result.current.isExecuteError).toBe(false);
  });
});
