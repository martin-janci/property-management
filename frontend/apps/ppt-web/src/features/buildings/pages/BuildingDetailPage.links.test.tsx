/// <reference types="vitest/globals" />
/**
 * BuildingDetailPage quick links (nav-fixes): the building facilities page
 * (`/buildings/:id/facilities`) had no inbound link anywhere in the app.
 */
import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { BuildingDetailPage } from './BuildingDetailPage';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (k: string) => k }),
}));

vi.mock('../../../contexts', () => ({
  useAuth: () => ({ user: { role: 'manager', organizationId: 'org-1' } }),
}));

vi.mock('../hooks', () => ({
  useBuilding: () => ({
    data: {
      id: 'b-1',
      name: 'Tower A',
      status: 'active',
      type: 'residential',
      unitCount: 10,
      floorCount: 4,
      address: { street: 'Main 1', city: 'Bratislava', postalCode: '81101', country: 'SK' },
    },
    isLoading: false,
    error: null,
  }),
  useBuildingFloors: () => ({ data: [], isLoading: false }),
  useBuildingCommonAreas: () => ({ data: [], isLoading: false }),
}));

vi.mock('../components', () => ({
  BuildingLocationMap: () => null,
  UnitsSection: () => null,
}));

describe('BuildingDetailPage quick links', () => {
  it('links to the building facilities page', () => {
    render(
      <MemoryRouter>
        <BuildingDetailPage buildingId="b-1" />
      </MemoryRouter>
    );
    expect(screen.getByRole('link', { name: 'facilities.nav.buildingFacilities' })).toHaveAttribute(
      'href',
      '/buildings/b-1/facilities'
    );
    expect(screen.getByRole('link', { name: 'buildings.detail.personMonths' })).toHaveAttribute(
      'href',
      '/buildings/b-1/person-months'
    );
  });
});
