/// <reference types="vitest/globals" />
/**
 * `useAllMeters()` fans out one meters query per building derived from
 * `useBuildings()`. Two cold-load defects are pinned here:
 *
 *  1. While `useBuildings()` is still pending, `buildingsData` is undefined so
 *     the fan-out list is empty — the hook must still report `isLoading: true`
 *     rather than a settled empty list (which flashes a false "No meters"
 *     empty-state). The old `buildingIds.length > 0 && …` guard short-circuited
 *     to `isLoading: false` here.
 *  2. On a buildings-fetch ERROR, `buildingsData` stays undefined forever, so a
 *     naive hook would resolve to a permanent done-loading empty list and
 *     swallow the error as "zero meters". The hook must surface `isError`.
 */
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderHook } from '@testing-library/react';
import type { ReactNode } from 'react';

const h = vi.hoisted(() => ({
  useBuildings: vi.fn(),
  listMetersForBuilding: vi.fn(),
}));

vi.mock('@ppt/api-client', () => ({
  useBuildings: () => h.useBuildings(),
  listMetersForBuilding: (...a: unknown[]) => h.listMetersForBuilding(...a),
  meterKeys: {
    buildingList: (id: string, q: unknown) => ['meters', 'building', id, q],
    consumption: (id: string, q: unknown) => ['meters', 'consumption', id, q],
  },
  // Unused by useAllMeters but imported at module load.
  getConsumptionHistory: vi.fn(),
  useMeter: () => ({ data: undefined, isLoading: true }),
  usePendingReadings: () => ({ data: [], isLoading: false }),
  useReading: () => ({ data: undefined, isLoading: true }),
  useSubmitReading: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useValidateReading: () => ({ mutateAsync: vi.fn(), isPending: false }),
}));

import { useAllMeters } from './meters';

function wrapper({ children }: { children: ReactNode }) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return <QueryClientProvider client={qc}>{children}</QueryClientProvider>;
}

beforeEach(() => {
  vi.clearAllMocks();
  h.listMetersForBuilding.mockResolvedValue({ meters: [], total: 0 });
});

describe('useAllMeters', () => {
  it('reports isLoading while useBuildings() is still pending (no false empty)', () => {
    h.useBuildings.mockReturnValue({ data: undefined, isLoading: true, isError: false });

    const { result } = renderHook(() => useAllMeters(), { wrapper });

    // The building list is empty only because buildings haven't arrived yet —
    // the hook must NOT present that as a settled "zero meters" result.
    expect(result.current.isLoading).toBe(true);
    expect(result.current.meters).toEqual([]);
  });

  it('surfaces isError on a buildings-fetch failure instead of a done-loading empty list', () => {
    h.useBuildings.mockReturnValue({ data: undefined, isLoading: false, isError: true });

    const { result } = renderHook(() => useAllMeters(), { wrapper });

    // A permanent undefined buildingsData must not masquerade as "0 meters,
    // finished loading, no error".
    expect(result.current.isError).toBe(true);
    const settledEmpty =
      result.current.isLoading === false &&
      result.current.meters.length === 0 &&
      !result.current.isError;
    expect(settledEmpty).toBe(false);
  });
});
