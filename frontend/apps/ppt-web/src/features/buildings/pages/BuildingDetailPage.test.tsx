/**
 * The backend has no common-areas route. The detail page must not request it
 * (every load logged a 404, round-3 audit #39) and must hide the Common Areas
 * section entirely rather than render an error or an empty state.
 */
/// <reference types="vitest/globals" />

import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const commonAreasResult = vi.hoisted(() => ({
  current: { data: null as unknown, isLoading: false },
}));
const commonAreasCalls = vi.hoisted(() => [] as Array<[string, boolean | undefined]>);

vi.mock('../../../contexts', () => ({
  useAuth: () => ({ user: { role: 'manager', organizationId: 'org-1' } }),
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
  useBuildingCommonAreas: (buildingId: string, enabled?: boolean) => {
    commonAreasCalls.push([buildingId, enabled]);
    return commonAreasResult.current;
  },
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
    commonAreasCalls.length = 0;
  });

  it('does not request common areas while the endpoint does not exist', () => {
    renderPage();
    expect(commonAreasCalls.length).toBeGreaterThan(0);
    expect(commonAreasCalls.every(([, enabled]) => enabled === false)).toBe(true);
  });

  it('hides the Common Areas section when the feature is unavailable (404 → null)', () => {
    renderPage();
    expect(screen.getByText('Test Building')).toBeInTheDocument();
    expect(screen.queryByText('Common Areas')).not.toBeInTheDocument();
    expect(screen.queryByText(/No common areas defined/)).not.toBeInTheDocument();
  });

  it('keeps the section hidden even if a cached list is present', () => {
    commonAreasResult.current = {
      data: [{ id: 'ca1', buildingId: 'b1', name: 'Garden', type: 'garden' }],
      isLoading: false,
    };
    renderPage();
    expect(screen.queryByText('Common Areas')).not.toBeInTheDocument();
    expect(screen.queryByText('Garden')).not.toBeInTheDocument();
  });
});
