/**
 * Facilities client vs api-server response/query shapes (round-3 audit #4, #21).
 *
 * - The list endpoints return a bare JSON array, but the pages read
 *   `response.items`; the client must normalize so the pages don't crash.
 * - `GET .../availability` requires `from` and `to` (RFC 3339), not `date`.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  checkAvailability,
  facilityDayRange,
  getMyBookings,
  listFacilities,
  listPendingBookings,
  normalizeFacilityList,
} from './api';

function okJson(body: unknown): Response {
  return { ok: true, status: 200, json: async () => body } as Response;
}

function lastUrl(): URL {
  const calls = vi.mocked(fetch).mock.calls;
  const call = calls[calls.length - 1];
  if (!call) throw new Error('fetch was not called');
  return new URL(String(call[0]), 'http://origin.invalid');
}

beforeEach(() => {
  vi.stubGlobal('fetch', vi.fn());
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('normalizeFacilityList', () => {
  it('wraps a bare array', () => {
    expect(normalizeFacilityList([{ id: 'a' }, { id: 'b' }])).toEqual({
      items: [{ id: 'a' }, { id: 'b' }],
      total: 2,
    });
  });

  it('keeps an { items, total } envelope', () => {
    expect(normalizeFacilityList({ items: [{ id: 'a' }], total: 7 })).toEqual({
      items: [{ id: 'a' }],
      total: 7,
    });
  });

  it('falls back to an empty list for anything else', () => {
    expect(normalizeFacilityList(null)).toEqual({ items: [], total: 0 });
    expect(normalizeFacilityList({})).toEqual({ items: [], total: 0 });
  });
});

describe('list endpoints return { items } for a bare-array server response', () => {
  it('listFacilities', async () => {
    vi.mocked(fetch).mockResolvedValue(okJson([{ id: 'f1' }]));
    const res = await listFacilities('b1');
    expect(res.items).toEqual([{ id: 'f1' }]);
    expect(res.total).toBe(1);
  });

  it('listPendingBookings', async () => {
    vi.mocked(fetch).mockResolvedValue(okJson([]));
    const res = await listPendingBookings('b1');
    expect(res.items).toEqual([]);
    expect(res.total).toBe(0);
  });

  it('getMyBookings', async () => {
    vi.mocked(fetch).mockResolvedValue(okJson([{ id: 'bk1' }, { id: 'bk2' }]));
    const res = await getMyBookings();
    expect(res.items).toHaveLength(2);
  });
});

describe('checkAvailability', () => {
  it('sends a from/to range covering the selected local day', async () => {
    vi.mocked(fetch).mockResolvedValue(okJson([]));
    await checkAvailability('b1', 'f1', { date: '2026-03-10' });
    const url = lastUrl();
    expect(url.pathname).toBe('/api/v1/buildings/b1/facilities/f1/availability');
    expect(url.searchParams.has('date')).toBe(false);
    const from = new Date(url.searchParams.get('from') ?? '');
    const to = new Date(url.searchParams.get('to') ?? '');
    expect(from.getFullYear()).toBe(2026);
    expect(from.getMonth()).toBe(2);
    expect(from.getDate()).toBe(10);
    expect(from.getHours()).toBe(0);
    expect(to.getTime() - from.getTime()).toBeGreaterThanOrEqual(23 * 3600 * 1000);
  });

  it('passes an explicit range through unchanged', async () => {
    vi.mocked(fetch).mockResolvedValue(okJson([]));
    await checkAvailability('b1', 'f1', {
      date: '2026-03-10',
      from: '2026-03-10T08:00:00.000Z',
      to: '2026-03-10T12:00:00.000Z',
    });
    const url = lastUrl();
    expect(url.searchParams.get('from')).toBe('2026-03-10T08:00:00.000Z');
    expect(url.searchParams.get('to')).toBe('2026-03-10T12:00:00.000Z');
  });
});

describe('facilityDayRange', () => {
  it('starts at local midnight and ends at the next local midnight', () => {
    const { from, to } = facilityDayRange('2026-12-31');
    const start = new Date(from);
    const end = new Date(to);
    expect([start.getFullYear(), start.getMonth(), start.getDate(), start.getHours()]).toEqual([
      2026, 11, 31, 0,
    ]);
    expect([end.getFullYear(), end.getMonth(), end.getDate(), end.getHours()]).toEqual([
      2027, 0, 1, 0,
    ]);
  });
});
