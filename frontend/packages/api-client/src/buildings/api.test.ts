/**
 * The backend has no `/buildings/{id}/floors` route (a floor is just the
 * `floor` column of a unit), so `listFloors` derives the floor list from the
 * building's units. These tests pin that it calls the existing paginated units
 * endpoint, pages until `total`, and groups/sorts/counts by floor. Common areas
 * are not implemented server-side either, so `listCommonAreas` maps 404 → null.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createBuildingsApi } from './api';

const BUILDING_ID = '22222222-2222-2222-2222-222222222222';

function okJson(body: unknown): Response {
  return { ok: true, status: 200, json: async () => body } as Response;
}

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
