/**
 * Wire-contract tests for the rentals calendar / reports / picker helpers.
 * Pins the paths, methods and query strings the api-server routes expect.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  checkRentalAvailability,
  createRentalBooking,
  createRentalCalendarBlock,
  createRentalReport,
  deleteRentalCalendarBlock,
  deleteRentalConnection,
  getRentalCalendar,
  getRentalConnection,
  listRentalBuildings,
  listRentalBuildingUnits,
  listRentalReports,
  previewRentalReport,
  submitRentalReport,
  updateRentalBooking,
  updateRentalConnection,
} from './api';

const HEADERS = { Authorization: 'Bearer t', 'X-Tenant-ID': 'org-1' };

function ok(body: unknown, status = 200): Response {
  return { ok: true, status, json: async () => body } as Response;
}

let fetchSpy: ReturnType<typeof vi.spyOn>;

beforeEach(() => {
  fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValue(ok([]));
});

afterEach(() => {
  vi.restoreAllMocks();
});

function lastCall(): { url: string; init: RequestInit } {
  const [url, init] = fetchSpy.mock.calls.at(-1) as [string, RequestInit];
  return { url, init };
}

describe('rentals calendar', () => {
  it('GETs the unit calendar with both date bounds', async () => {
    await getRentalCalendar(
      'unit 1',
      { start_date: '2026-11-01', end_date: '2026-12-01' },
      HEADERS
    );
    const { url, init } = lastCall();
    expect(url).toBe('/api/v1/rentals/calendar/unit%201?start_date=2026-11-01&end_date=2026-12-01');
    expect(init.method ?? 'GET').toBe('GET');
    expect(new Headers(init.headers).get('X-Tenant-ID')).toBe('org-1');
  });

  it('GETs availability for a half-open range', async () => {
    fetchSpy.mockResolvedValueOnce(ok({ available: true }));
    const res = await checkRentalAvailability(
      'u1',
      { start_date: '2026-11-03', end_date: '2026-11-05' },
      HEADERS
    );
    expect(res.available).toBe(true);
    expect(lastCall().url).toBe(
      '/api/v1/rentals/calendar/u1/availability?start_date=2026-11-03&end_date=2026-11-05'
    );
  });

  it('POSTs a block and DELETEs it by id', async () => {
    fetchSpy.mockResolvedValueOnce(ok({ id: 'b1' }));
    await createRentalCalendarBlock(
      { unit_id: 'u1', block_start: '2026-11-03', block_end: '2026-11-05', reason: 'owner_use' },
      HEADERS
    );
    let call = lastCall();
    expect(call.url).toBe('/api/v1/rentals/calendar/blocks');
    expect(call.init.method).toBe('POST');
    expect(JSON.parse(String(call.init.body))).toMatchObject({ reason: 'owner_use' });

    fetchSpy.mockResolvedValueOnce({ ok: true, status: 204, json: async () => null } as Response);
    await deleteRentalCalendarBlock('b1', HEADERS);
    call = lastCall();
    expect(call.url).toBe('/api/v1/rentals/calendar/blocks/b1');
    expect(call.init.method).toBe('DELETE');
  });
});

describe('rentals reports', () => {
  it('lists reports with optional filters', async () => {
    await listRentalReports({ building_id: 'b1', limit: 50 }, HEADERS);
    expect(lastCall().url).toBe('/api/v1/rentals/reports?building_id=b1&limit=50');
    await listRentalReports({}, HEADERS);
    expect(lastCall().url).toBe('/api/v1/rentals/reports');
  });

  it('previews, creates and submits', async () => {
    fetchSpy.mockResolvedValue(ok({}));
    await previewRentalReport(
      { building_id: 'b1', period_start: '2026-10-01', period_end: '2026-10-31' },
      HEADERS
    );
    expect(lastCall().url).toBe('/api/v1/rentals/reports/preview');
    expect(lastCall().init.method).toBe('POST');

    await createRentalReport(
      {
        building_id: 'b1',
        report_type: 'monthly',
        period_start: '2026-10-01',
        period_end: '2026-10-31',
        authority_code: 'SK_UHUL',
      },
      HEADERS
    );
    expect(lastCall().url).toBe('/api/v1/rentals/reports');
    expect(JSON.parse(String(lastCall().init.body))).toMatchObject({ authority_code: 'SK_UHUL' });

    await submitRentalReport('r1', HEADERS);
    expect(lastCall().url).toBe('/api/v1/rentals/reports/r1/submit');
    expect(lastCall().init.method).toBe('POST');
  });
});

describe('building / unit pickers', () => {
  it('passes organization_id (required by the server) and the max limit', async () => {
    await listRentalBuildings('org-1', HEADERS);
    expect(lastCall().url).toBe('/api/v1/buildings?organization_id=org-1&limit=100');
    await listRentalBuildingUnits('b1', HEADERS);
    expect(lastCall().url).toBe('/api/v1/buildings/b1/units?limit=100');
  });
});

describe('rentals connections', () => {
  it('GETs, PUTs and DELETEs one connection by id', async () => {
    fetchSpy.mockResolvedValueOnce(ok({ id: 'c1', sync_calendar: true }));
    await getRentalConnection('c 1', HEADERS);
    expect(lastCall().url).toBe('/api/v1/rentals/connections/c%201');
    expect(lastCall().init.method ?? 'GET').toBe('GET');

    fetchSpy.mockResolvedValueOnce(ok({ id: 'c1', is_active: false }));
    await updateRentalConnection('c1', { is_active: false }, HEADERS);
    let call = lastCall();
    expect(call.url).toBe('/api/v1/rentals/connections/c1');
    expect(call.init.method).toBe('PUT');
    expect(JSON.parse(String(call.init.body))).toEqual({ is_active: false });
    expect(new Headers(call.init.headers).get('X-Tenant-ID')).toBe('org-1');

    fetchSpy.mockResolvedValueOnce({ ok: true, status: 204, json: async () => null } as Response);
    await deleteRentalConnection('c1', HEADERS);
    call = lastCall();
    expect(call.url).toBe('/api/v1/rentals/connections/c1');
    expect(call.init.method).toBe('DELETE');
  });
});

describe('rentals bookings', () => {
  it('POSTs a new booking and PUTs an update by id', async () => {
    fetchSpy.mockResolvedValue(ok({ id: 'bk1' }));
    await createRentalBooking(
      { unit_id: 'u1', guest_name: 'Jana', check_in: '2026-11-03', check_out: '2026-11-05' },
      HEADERS
    );
    let call = lastCall();
    expect(call.url).toBe('/api/v1/rentals/bookings');
    expect(call.init.method).toBe('POST');
    expect(JSON.parse(String(call.init.body))).toMatchObject({ unit_id: 'u1', guest_name: 'Jana' });

    await updateRentalBooking('bk1', { guest_count: 3 }, HEADERS);
    call = lastCall();
    expect(call.url).toBe('/api/v1/rentals/bookings/bk1');
    expect(call.init.method).toBe('PUT');
    expect(JSON.parse(String(call.init.body))).toEqual({ guest_count: 3 });
  });
});
