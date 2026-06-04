import { type BuildingsPaginatedResponse, selectBuilding } from './NeighborsScreen';

// Regression for PR #918 (High #5): `GET /api/v1/buildings` returns
// `{ buildings, total }`, not `{ items }`. The screen used to read
// `data.items[0]`, which was always undefined, so no building id was ever
// resolved and the neighbours list never loaded. selectBuilding pins the
// corrected `buildings[0]` extraction.

const response: BuildingsPaginatedResponse = {
  buildings: [
    { id: 'b-1', name: 'Maple Court' },
    { id: 'b-2', name: 'Oak House' },
  ],
  total: 2,
};

describe('selectBuilding (PR #918 reads `buildings`, not `items`)', () => {
  it('resolves the first building id and name', () => {
    expect(selectBuilding(response)).toEqual({
      buildingId: 'b-1',
      buildingName: 'Maple Court',
    });
  });

  it('coerces a null building name to undefined', () => {
    expect(selectBuilding({ buildings: [{ id: 'b-1', name: null }], total: 1 })).toEqual({
      buildingId: 'b-1',
      buildingName: undefined,
    });
  });

  it('returns undefined ids for an empty or absent list', () => {
    expect(selectBuilding({ buildings: [], total: 0 })).toEqual({
      buildingId: undefined,
      buildingName: undefined,
    });
    expect(selectBuilding(undefined)).toEqual({
      buildingId: undefined,
      buildingName: undefined,
    });
  });

  it('does NOT read the legacy `items` key', () => {
    const legacy = { items: [{ id: 'x' }], total: 1 } as unknown as BuildingsPaginatedResponse;
    expect(selectBuilding(legacy).buildingId).toBeUndefined();
  });
});
