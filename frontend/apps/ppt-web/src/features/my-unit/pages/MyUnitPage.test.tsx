/**
 * MyUnitPage tests (Epic 3, Story 3.6).
 *
 * Pins the resident-facing behaviour:
 *  (a) loading / error / empty states render
 *  (b) a single unit shows its details + the caller's association status
 *  (c) the unit switcher appears for >1 unit and switching swaps the detail
 *  (d) privacy: the page only ever renders what the (PII-free) API returns —
 *      there is no field or code path that surfaces another resident/owner.
 *
 * `useMyUnits` is mocked so the test exercises the component in isolation.
 */

/// <reference types="vitest/globals" />
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { MyUnitPage } from './MyUnitPage';

const mockUseMyUnits = vi.fn();
vi.mock('@ppt/api-client', () => ({
  useMyUnits: () => mockUseMyUnits(),
}));

interface MyUnitOverrides {
  unitId?: string;
  designation?: string;
  street?: string;
  residentType?: string;
}

function makeUnit(o: MyUnitOverrides = {}) {
  return {
    association: {
      residentId: `res-${o.unitId ?? '1'}`,
      residentType: o.residentType ?? 'tenant',
      residentTypeDisplay: o.residentType === 'owner' ? 'Owner' : 'Tenant',
      isPrimary: true,
      startDate: '2024-01-01',
      endDate: null,
      isActive: true,
      receivesNotifications: true,
      receivesMail: true,
    },
    unit: {
      id: o.unitId ?? 'unit-1',
      buildingId: 'bld-1',
      entrance: null,
      designation: o.designation ?? '3B',
      floor: 3,
      floorDisplay: '3',
      unitType: 'apartment',
      sizeSqm: '72.5',
      rooms: 3,
      ownershipShare: '100.00',
      occupancyStatus: 'owner_occupied',
      description: null,
      status: 'active',
    },
    building: {
      id: 'bld-1',
      name: 'My Building',
      street: o.street ?? 'Rezidentská 7',
      city: 'Bratislava',
      postalCode: '81101',
    },
  };
}

describe('MyUnitPage', () => {
  beforeEach(() => {
    mockUseMyUnits.mockReset();
  });

  it('renders a loading state', () => {
    mockUseMyUnits.mockReturnValue({ isLoading: true, isError: false, data: undefined });
    render(<MyUnitPage />);
    expect(screen.getByText(/loading your unit/i)).toBeInTheDocument();
  });

  it('renders an error state', () => {
    mockUseMyUnits.mockReturnValue({
      isLoading: false,
      isError: true,
      error: new Error('boom'),
      data: undefined,
    });
    render(<MyUnitPage />);
    expect(screen.getByRole('alert')).toHaveTextContent(/boom/);
  });

  it('renders an empty state when the resident has no units', () => {
    mockUseMyUnits.mockReturnValue({ isLoading: false, isError: false, data: [] });
    render(<MyUnitPage />);
    expect(screen.getByText(/not currently registered as a resident/i)).toBeInTheDocument();
  });

  it('shows unit details and the caller association status for one unit', () => {
    mockUseMyUnits.mockReturnValue({
      isLoading: false,
      isError: false,
      data: [makeUnit({ residentType: 'owner' })],
    });
    render(<MyUnitPage />);

    expect(screen.getByRole('heading', { name: /unit 3B/i })).toBeInTheDocument();
    expect(screen.getByText('Owner')).toBeInTheDocument();
    expect(screen.getByText('Bratislava', { exact: false })).toBeInTheDocument();
    // Privacy affordance is always present.
    expect(screen.getByText(/other residents and owners.*are not shown/i)).toBeInTheDocument();
    // No switcher for a single unit.
    expect(screen.queryByLabelText(/choose a unit/i)).not.toBeInTheDocument();
  });

  it('renders a switcher for multiple units and switching swaps the detail', async () => {
    const user = userEvent.setup();
    mockUseMyUnits.mockReturnValue({
      isLoading: false,
      isError: false,
      data: [
        makeUnit({ unitId: 'unit-1', designation: '1A', street: 'Alpha 1' }),
        makeUnit({ unitId: 'unit-2', designation: '2C', street: 'Beta 2' }),
      ],
    });
    render(<MyUnitPage />);

    // Defaults to the first unit.
    expect(screen.getByRole('heading', { name: /unit 1A/i })).toBeInTheDocument();

    const switcher = screen.getByLabelText(/choose a unit/i);
    await user.selectOptions(switcher, 'unit-2');

    expect(screen.getByRole('heading', { name: /unit 2C/i })).toBeInTheDocument();
  });
});
