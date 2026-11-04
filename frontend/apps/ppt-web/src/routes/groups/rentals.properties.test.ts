/**
 * `loadRentalPropertyOptions` feeds the rentals unit/building pickers from
 * `GET /buildings` + `GET /buildings/{id}/units`.
 */
import { listRentalBuildings, listRentalBuildingUnits } from '@ppt/api-client';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { loadRentalPropertyOptions } from './rentals';

vi.mock('@ppt/api-client', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@ppt/api-client')>()),
  listRentalBuildings: vi.fn(),
  listRentalBuildingUnits: vi.fn(),
}));

const HEADERS = { Authorization: 'Bearer t', 'X-Tenant-ID': 'org-1' };
const building = (id: string, name: string) => ({
  id,
  name,
  street: 'Hlavná 1',
  city: 'Košice',
  postal_code: '04001',
  status: 'active',
});
const unit = (id: string, building_id: string, designation: string) => ({
  id,
  building_id,
  designation,
  floor: 0,
  unit_type: 'apartment',
  status: 'active',
});

beforeEach(() => {
  vi.mocked(listRentalBuildings).mockReset();
  vi.mocked(listRentalBuildingUnits).mockReset();
});

describe('loadRentalPropertyOptions', () => {
  it('loads units per building and flattens them for the pickers', async () => {
    vi.mocked(listRentalBuildings).mockResolvedValue({
      buildings: [building('b1', 'Astra'), building('b2', 'Borovica')],
      total: 2,
      offset: 0,
      limit: 100,
    });
    vi.mocked(listRentalBuildingUnits).mockImplementation(async (id) => ({
      units: id === 'b1' ? [unit('u1', 'b1', '1A')] : [unit('u2', 'b2', '2B')],
      total: 1,
      offset: 0,
      limit: 100,
    }));

    const res = await loadRentalPropertyOptions('org-1', HEADERS);
    expect(listRentalBuildings).toHaveBeenCalledWith('org-1', HEADERS);
    expect(listRentalBuildingUnits).toHaveBeenCalledTimes(2);
    expect(res.buildings.map((b) => b.name)).toEqual(['Astra', 'Borovica']);
    expect(res.units.map((u) => `${u.buildingName}/${u.name}`)).toEqual([
      'Astra/1A',
      'Borovica/2B',
    ]);
  });

  it('keeps a building whose units failed to load, without units', async () => {
    vi.mocked(listRentalBuildings).mockResolvedValue({
      buildings: [building('b1', 'Astra'), building('b2', 'Borovica')],
      total: 2,
      offset: 0,
      limit: 100,
    });
    vi.mocked(listRentalBuildingUnits).mockImplementation(async (id) => {
      if (id === 'b2') throw new Error('403');
      return { units: [unit('u1', 'b1', '1A')], total: 1, offset: 0, limit: 100 };
    });

    const res = await loadRentalPropertyOptions('org-1', HEADERS);
    expect(res.buildings).toHaveLength(2);
    expect(res.units.map((u) => u.id)).toEqual(['u1']);
  });

  it('propagates a failed buildings request', async () => {
    vi.mocked(listRentalBuildings).mockRejectedValue(new Error('boom'));
    await expect(loadRentalPropertyOptions('org-1', HEADERS)).rejects.toThrow('boom');
  });
});
