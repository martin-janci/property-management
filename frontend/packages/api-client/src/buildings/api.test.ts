/**
 * Contract tests for the buildings transport layer against the real
 * api-server payloads (flat snake_case rows + `{ buildings, total, offset,
 * limit }` envelope).
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { clearOrgProvider, setOrgProvider } from '../auth/org-provider';
import { createBuildingsApi, fromApiBuilding, getBuilding, listBuildings } from './api';

const ORG = '0ff32ca6-f9b2-4abf-a6af-f86418892190';

const apiRow = {
  id: 'b-1',
  organization_id: ORG,
  name: 'Bytový dom Hlavná',
  street: 'Hlavná 123',
  city: 'Bratislava',
  postal_code: '81101',
  country: 'SK',
  year_built: 2010,
  total_floors: 5,
  unit_count: 8,
  status: 'active',
  latitude: 48.14,
  longitude: 17.1,
  created_at: '2026-11-03T21:39:41Z',
  updated_at: '2026-11-03T21:39:41Z',
};

function okJson(body: unknown): Response {
  return { ok: true, status: 200, json: async () => body } as Response;
}

describe('fromApiBuilding', () => {
  it('maps the flat api-server row onto the client Building shape', () => {
    expect(fromApiBuilding(apiRow)).toMatchObject({
      id: 'b-1',
      organizationId: ORG,
      address: { street: 'Hlavná 123', city: 'Bratislava', postalCode: '81101', country: 'SK' },
      floorCount: 5,
      unitCount: 8,
      yearBuilt: 2010,
      status: 'active',
      type: 'residential',
      location: { latitude: 48.14, longitude: 17.1 },
    });
  });

  it('passes an already client-shaped building through', () => {
    const b = { id: 'x', address: { street: 's', city: 'c', postalCode: 'p', country: 'SK' } };
    expect(fromApiBuilding(b)).toMatchObject(b);
  });
});

describe('listBuildings', () => {
  beforeEach(() => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      okJson({ buildings: [apiRow], total: 3, offset: 2, limit: 2 })
    );
    setOrgProvider(() => ORG);
  });
  afterEach(() => {
    clearOrgProvider();
    vi.restoreAllMocks();
  });

  it('sends the required organization_id and offset/limit pagination', async () => {
    await listBuildings({ page: 2, pageSize: 2, status: 'active' });
    const url = new URL(String(vi.mocked(fetch).mock.calls[0][0]), 'http://o.invalid');
    expect(url.pathname).toBe('/api/v1/buildings');
    expect(url.searchParams.get('organization_id')).toBe(ORG);
    expect(url.searchParams.get('offset')).toBe('2');
    expect(url.searchParams.get('limit')).toBe('2');
    expect(url.searchParams.get('status')).toBe('active');
    expect(url.searchParams.has('page')).toBe(false);
  });

  it('converts the { buildings, total, offset, limit } envelope', async () => {
    const res = await listBuildings();
    expect(res).toMatchObject({ total: 3, page: 2, pageSize: 2, totalPages: 2 });
    expect(res.items[0].address.city).toBe('Bratislava');
  });
});

describe('getBuilding', () => {
  afterEach(() => vi.restoreAllMocks());

  it('maps the detail payload', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(okJson(apiRow));
    const b = await getBuilding('b-1');
    expect(b.address.postalCode).toBe('81101');
    expect(b.floorCount).toBe(5);
  });
});

describe('createBuildingsApi (factory used by ppt-web hooks)', () => {
  beforeEach(() => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      okJson({ buildings: [apiRow], total: 1, offset: 0, limit: 12 })
    );
    setOrgProvider(() => ORG);
  });
  afterEach(() => {
    clearOrgProvider();
    vi.restoreAllMocks();
  });

  it('lists with organization_id + limit and maps rows', async () => {
    const api = createBuildingsApi({ baseUrl: '', accessToken: 't' });
    const res = await api.list({ page: 1, pageSize: 12 });
    const url = new URL(String(vi.mocked(fetch).mock.calls[0][0]), 'http://o.invalid');
    expect(url.searchParams.get('organization_id')).toBe(ORG);
    expect(url.searchParams.get('limit')).toBe('12');
    expect(res.items[0].unitCount).toBe(8);
  });
});

// --- floors / common areas (derived from the units endpoint) ---

const BUILDING_ID = '22222222-2222-2222-2222-222222222222';

function unit(id: string, floor: number) {
  return {
    id,
    building_id: BUILDING_ID,
    designation: id,
    floor,
    unit_type: 'apartment',
    occupancy_status: 'unknown',
    status: 'active',
  };
}

describe('createBuildingsApi().listFloors', () => {
  const fetchMock = vi.fn();

  beforeEach(() => {
    fetchMock.mockReset();
    vi.stubGlobal('fetch', fetchMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('derives floors from the units endpoint, paging until total', async () => {
    const firstPage = Array.from({ length: 100 }, (_, i) => unit(`u${i}`, i % 2 === 0 ? 1 : -1));
    fetchMock
      .mockResolvedValueOnce(okJson({ units: firstPage, total: 101, offset: 0, limit: 100 }))
      .mockResolvedValueOnce(
        okJson({ units: [unit('u100', 0)], total: 101, offset: 100, limit: 100 })
      );

    const api = createBuildingsApi({ baseUrl: '' });
    const floors = await api.listFloors(BUILDING_ID);

    expect(fetchMock).toHaveBeenCalledTimes(2);
    expect(fetchMock.mock.calls[0][0]).toBe(
      `/api/v1/buildings/${BUILDING_ID}/units?offset=0&limit=100`
    );
    expect(fetchMock.mock.calls[1][0]).toBe(
      `/api/v1/buildings/${BUILDING_ID}/units?offset=100&limit=100`
    );
    expect(floors).toEqual([
      { id: `${BUILDING_ID}:floor:-1`, buildingId: BUILDING_ID, number: -1, unitCount: 50 },
      { id: `${BUILDING_ID}:floor:0`, buildingId: BUILDING_ID, number: 0, unitCount: 1 },
      { id: `${BUILDING_ID}:floor:1`, buildingId: BUILDING_ID, number: 1, unitCount: 50 },
    ]);
  });

  it('returns an empty list for a building without units', async () => {
    fetchMock.mockResolvedValueOnce(okJson({ units: [], total: 0, offset: 0, limit: 100 }));

    const api = createBuildingsApi({ baseUrl: '' });
    await expect(api.listFloors(BUILDING_ID)).resolves.toEqual([]);
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });
});

describe('createBuildingsApi().listCommonAreas', () => {
  const fetchMock = vi.fn();

  beforeEach(() => {
    fetchMock.mockReset();
    vi.stubGlobal('fetch', fetchMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('resolves null on 404 (backend has no common-areas route) instead of throwing', async () => {
    fetchMock.mockResolvedValueOnce({
      ok: false,
      status: 404,
      json: async () => ({ message: 'Not Found' }),
    } as Response);

    const api = createBuildingsApi({ baseUrl: '' });
    await expect(api.listCommonAreas(BUILDING_ID)).resolves.toBeNull();
    expect(fetchMock.mock.calls[0][0]).toBe(`/api/v1/buildings/${BUILDING_ID}/common-areas`);
  });

  it('still throws on other errors', async () => {
    fetchMock.mockResolvedValueOnce({
      ok: false,
      status: 500,
      json: async () => ({ message: 'boom' }),
    } as Response);

    const api = createBuildingsApi({ baseUrl: '' });
    await expect(api.listCommonAreas(BUILDING_ID)).rejects.toThrow('boom');
  });
});
