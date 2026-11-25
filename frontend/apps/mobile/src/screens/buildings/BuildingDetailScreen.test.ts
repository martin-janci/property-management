/** The building detail reads the bare snake_case `BuildingResponse`. */
import { parseBuildingDetail } from './BuildingDetailScreen';

const response = {
  id: 'b-1',
  organization_id: 'o-1',
  street: 'Hlavna 1',
  city: 'Bratislava',
  postal_code: '811 01',
  country: 'SK',
  name: null,
  description: 'Brick house',
  year_built: 1985,
  total_floors: 8,
  total_entrances: 2,
  amenities: ['elevator', '', 7, 'parking'],
  status: 'active',
};

describe('parseBuildingDetail', () => {
  it('maps the response and falls back to the street for a missing name', () => {
    expect(parseBuildingDetail(response)).toEqual({
      id: 'b-1',
      name: 'Hlavna 1',
      address: 'Hlavna 1, 811 01 Bratislava',
      description: 'Brick house',
      yearBuilt: 1985,
      floors: 8,
      entrances: 2,
      amenities: ['elevator', 'parking'],
      status: 'active',
    });
  });

  it('tolerates missing optional fields', () => {
    expect(parseBuildingDetail({ id: 'b-2', street: 'Dlha 5', name: 'Dlha' })).toEqual({
      id: 'b-2',
      name: 'Dlha',
      address: 'Dlha 5',
      description: '',
      yearBuilt: null,
      floors: null,
      entrances: null,
      amenities: [],
      status: '',
    });
  });

  it('returns null for anything that is not a building', () => {
    expect(parseBuildingDetail(undefined)).toBeNull();
    expect(parseBuildingDetail({ buildings: [] })).toBeNull();
    expect(parseBuildingDetail({ id: 'b-3' })).toBeNull();
  });
});
