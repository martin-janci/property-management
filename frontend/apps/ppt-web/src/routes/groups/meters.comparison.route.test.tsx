/**
 * Meters route group — ReadingComparisonPageRoute wiring.
 *
 * "Compare" used to be a no-op with no data. It now fans out one
 * `GET /meters/{id}/consumption?from&to` (via `getConsumptionHistory`) per
 * selected meter and feeds the series to the comparison chart. Pins: one call
 * per selected meter with the chosen range, the legend shows each series, an
 * empty result shows the "no data" message, and a failure toasts.
 */

/// <reference types="vitest/globals" />
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Suspense } from 'react';
import { MemoryRouter, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: unknown) =>
      typeof opts === 'string'
        ? opts
        : ((opts as { defaultValue?: string } | undefined)?.defaultValue ?? key),
    i18n: { language: 'en' },
  }),
}));

const mockShowToast = vi.hoisted(() => vi.fn());
vi.mock('../../components', () => ({
  useToast: () => ({ showToast: mockShowToast }),
  ProtectedRoute: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

vi.mock('../../hooks/useOrganization', () => ({
  useOrganization: () => ({ organizationId: 'org-1' }),
}));

const { mockGetConsumption, mockListMeters } = vi.hoisted(() => ({
  mockGetConsumption: vi.fn(),
  mockListMeters: vi.fn(),
}));

vi.mock('@ppt/api-client', () => ({
  getConsumptionHistory: (...a: unknown[]) => mockGetConsumption(...a),
  listMetersForBuilding: (...a: unknown[]) => mockListMeters(...a),
  meterKeys: {
    buildingList: (id: string, q: unknown) => ['meters', 'building', id, q],
    consumption: (id: string, q: unknown) => ['meters', 'consumption', id, q],
  },
  useBuildings: () => ({ data: { items: [{ id: 'b-1', name: 'Hlavná 5' }] } }),
  useMeter: () => ({ data: undefined, isLoading: true }),
  usePendingReadings: () => ({ data: [], isLoading: false }),
  useReading: () => ({ data: undefined, isLoading: true }),
  useSubmitReading: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useValidateReading: () => ({ mutateAsync: vi.fn(), isPending: false }),
}));

import { mapConsumptionToChartData, meterRoutes } from './meters';

function apiMeter(id: string, number: string, type = 'water') {
  return {
    id,
    organization_id: 'org-1',
    building_id: 'b-1',
    meter_number: number,
    meter_type: type,
    unit_of_measure: 'm3',
    location: null,
    is_active: true,
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
  };
}

function renderComparison() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter initialEntries={['/meters/comparison']}>
        <Suspense fallback={<div>loading…</div>}>
          <Routes>{meterRoutes()}</Routes>
        </Suspense>
      </MemoryRouter>
    </QueryClientProvider>
  );
}

async function selectBothAndCompare() {
  renderComparison();
  await screen.findByText('WM-001');
  const boxes = screen.getAllByRole('checkbox');
  await userEvent.click(boxes[0]);
  await userEvent.click(boxes[1]);
  fireEvent.change(screen.getByLabelText('meters.export.startDate'), {
    target: { value: '2026-01-01' },
  });
  fireEvent.change(screen.getByLabelText('meters.export.endDate'), {
    target: { value: '2026-06-30' },
  });
  await userEvent.click(screen.getByRole('button', { name: /meters\.comparison\.compare/ }));
}

describe('ReadingComparisonPageRoute', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockListMeters.mockResolvedValue({
      meters: [apiMeter('m-1', 'WM-001'), apiMeter('m-2', 'WM-002')],
      total: 2,
    });
  });

  it('fetches consumption per selected meter for the range and charts both series', async () => {
    mockGetConsumption.mockImplementation(async (id: string) => ({
      meter_id: id,
      meter_type: 'water',
      unit_of_measure: 'm3',
      history: [
        { date: '2026-02-01', reading: '10', consumption: id === 'm-1' ? '4.5' : '6' },
        { date: '2026-03-01', reading: '15', consumption: id === 'm-1' ? '5' : '7.25' },
      ],
    }));
    await selectBothAndCompare();

    await waitFor(() => expect(mockGetConsumption).toHaveBeenCalledTimes(2));
    expect(mockGetConsumption).toHaveBeenCalledWith('m-1', {
      from: '2026-01-01',
      to: '2026-06-30',
    });
    expect(mockGetConsumption).toHaveBeenCalledWith('m-2', {
      from: '2026-01-01',
      to: '2026-06-30',
    });
    // Legend entries ("<name> (<unit>)") for both series.
    expect(await screen.findByText('WM-001 (m3)')).toBeInTheDocument();
    expect(screen.getByText('WM-002 (m3)')).toBeInTheDocument();
    expect(screen.queryByText('meters.comparison.selectToCompare')).not.toBeInTheDocument();
  });

  it('shows the no-data message when the range has no consumption', async () => {
    mockGetConsumption.mockImplementation(async (id: string) => ({
      meter_id: id,
      meter_type: 'water',
      unit_of_measure: 'm3',
      history: [],
    }));
    await selectBothAndCompare();
    expect(await screen.findByText('meters.comparison.noData')).toBeInTheDocument();
  });

  it('toasts when a consumption request fails', async () => {
    mockGetConsumption.mockRejectedValue(new Error('HTTP 500'));
    await selectBothAndCompare();
    await waitFor(() =>
      expect(mockShowToast).toHaveBeenCalledWith(
        expect.objectContaining({ type: 'error', message: 'meters.comparison.loadFailed' })
      )
    );
    expect(mockShowToast).toHaveBeenCalledTimes(1);
  });
});

describe('mapConsumptionToChartData', () => {
  it('plots per-period consumption (not cumulative readings) as numbers', () => {
    const series = mapConsumptionToChartData(
      {
        meter_id: 'm-1',
        meter_type: 'heat',
        unit_of_measure: 'kWh',
        history: [{ date: '2026-02-01', reading: '1200.5', consumption: '80.25' }],
      },
      undefined,
      1
    );
    expect(series.dataPoints).toEqual([{ date: '2026-02-01', value: 80.25, consumption: 80.25 }]);
    expect(series.unit).toBe('kWh');
    expect(series.meterType).toBe('heat');
    expect(series.color).toBe('#10b981');
  });
});
