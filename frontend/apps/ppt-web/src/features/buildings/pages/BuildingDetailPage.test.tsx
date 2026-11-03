/**
 * The backend has no common-areas route, so `listCommonAreas` resolves `null`
 * on 404. The detail page must then hide the Common Areas section entirely
 * rather than render an error or an empty state.
 */
/// <reference types="vitest/globals" />

import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const commonAreasResult = vi.hoisted(() => ({
  current: { data: null as unknown, isLoading: false },
}));

vi.mock('../hooks', () => ({
  useBuilding: () => ({
    data: {
      id: 'b1',
      name: 'Test Building',
      address: { street: 'Main 1', city: 'Bratislava', postalCode: '81101', country: 'SK' },
      type: 'residential',
      status: 'active',
      floorCount: 3,
      unitCount: 6,
    },
    isLoading: false,
    error: null,
  }),
  useBuildingFloors: () => ({ data: [], isLoading: false }),
  useBuildingCommonAreas: () => commonAreasResult.current,
}));

vi.mock('../components', () => ({
  BuildingLocationMap: () => null,
  UnitsSection: () => null,
}));

import { BuildingDetailPage } from './BuildingDetailPage';

function renderPage() {
  return render(
    <MemoryRouter>
      <BuildingDetailPage buildingId="b1" />
    </MemoryRouter>
  );
}

describe('BuildingDetailPage — common areas', () => {
  beforeEach(() => {
    commonAreasResult.current = { data: null, isLoading: false };
  });

  it('hides the Common Areas section when the feature is unavailable (404 → null)', () => {
    renderPage();
    expect(screen.getByText('Test Building')).toBeInTheDocument();
    expect(screen.queryByText('Common Areas')).not.toBeInTheDocument();
    expect(screen.queryByText(/No common areas defined/)).not.toBeInTheDocument();
  });

  it('shows the section when the list loads', () => {
    commonAreasResult.current = {
      data: [{ id: 'ca1', buildingId: 'b1', name: 'Garden', type: 'garden' }],
      isLoading: false,
    };
    renderPage();
    expect(screen.getByText('Common Areas')).toBeInTheDocument();
    expect(screen.getByText('Garden')).toBeInTheDocument();
  });
});
