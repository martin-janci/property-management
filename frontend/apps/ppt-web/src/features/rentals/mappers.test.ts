/**
 * Tests for the rentals calendar / daňový výkaz helpers.
 */
import type {
  RentalBuildingListItem,
  RentalCalendarEvent,
  RentalReportPreview,
  RentalReportSummary,
} from '@ppt/api-client';
import { describe, expect, it } from 'vitest';
import {
  addDaysIso,
  blockReasonFromTitle,
  buildingDisplayName,
  buildPropertyOptions,
  calendarFetchRange,
  computeReportPeriod,
  eventCoversDay,
  getMonthGridDays,
  localizedCountryName,
  mapCalendarEventToUi,
  mapReportPreviewToUi,
  mapReportSummaryToUi,
  parseIsoDate,
  toIsoDate,
} from './mappers';

describe('local date helpers', () => {
  it('parses ISO dates as local midnight (no UTC day shift)', () => {
    const d = parseIsoDate('2026-11-01');
    expect([d.getFullYear(), d.getMonth(), d.getDate(), d.getHours()]).toEqual([2026, 10, 1, 0]);
    expect(toIsoDate(d)).toBe('2026-11-01');
    expect(toIsoDate(parseIsoDate('2026-11-01T23:30:00Z'))).toBe('2026-11-01');
  });

  it('adds days across month and year boundaries', () => {
    expect(addDaysIso('2026-12-31', 1)).toBe('2027-01-01');
    expect(addDaysIso('2026-03-01', -1)).toBe('2026-02-28');
  });

  it('builds a Monday-first grid covering whole weeks', () => {
    // November 2026 starts on a Sunday → 6 leading days from October.
    const days = getMonthGridDays(new Date(2026, 10, 15));
    expect(days.length % 7).toBe(0);
    expect(days[0].getDay()).toBe(1);
    expect(toIsoDate(days[0])).toBe('2026-10-26');
    expect(toIsoDate(days[days.length - 1])).toBe('2026-12-06');
  });

  it('fetch range is half-open over the visible grid', () => {
    expect(calendarFetchRange(new Date(2026, 10, 4))).toEqual({
      startDate: '2026-10-26',
      endDate: '2026-12-07',
    });
  });

  it('treats events as half-open (check-out day is free)', () => {
    const ev = { startDate: '2026-11-03', endDate: '2026-11-05' };
    expect(eventCoversDay(ev, new Date(2026, 10, 2))).toBe(false);
    expect(eventCoversDay(ev, new Date(2026, 10, 3))).toBe(true);
    expect(eventCoversDay(ev, new Date(2026, 10, 4))).toBe(true);
    expect(eventCoversDay(ev, new Date(2026, 10, 5))).toBe(false);
    // zero-length event still shows on its day
    expect(
      eventCoversDay({ startDate: '2026-11-07', endDate: '2026-11-07' }, new Date(2026, 10, 7))
    ).toBe(true);
  });
});

describe('mapCalendarEventToUi', () => {
  const base: RentalCalendarEvent = {
    id: 'blk-1',
    unit_id: 'u1',
    start_date: '2026-11-03',
    end_date: '2026-11-06',
    event_type: 'booking',
    title: 'Ján Novák',
    platform: 'airbnb',
    booking_status: 'confirmed',
    color: '#22c55e',
  };

  it('maps a reservation with guest, source and status', () => {
    expect(mapCalendarEventToUi(base)).toMatchObject({
      id: 'blk-1',
      unitId: 'u1',
      type: 'booking',
      guestName: 'Ján Novák',
      source: 'airbnb',
      status: 'confirmed',
      startDate: '2026-11-03',
      endDate: '2026-11-06',
    });
  });

  it('folds unknown platforms to other and drops unknown statuses', () => {
    const ui = mapCalendarEventToUi({ ...base, platform: 'vrbo', booking_status: 'weird' });
    expect(ui.source).toBe('other');
    expect(ui.status).toBeUndefined();
  });

  it('recovers the block reason from the server title', () => {
    expect(blockReasonFromTitle('Maintenance')).toBe('maintenance');
    expect(blockReasonFromTitle('Owner Use')).toBe('owner_use');
    expect(blockReasonFromTitle('Blocked')).toBe('blocked');
    const m = mapCalendarEventToUi({
      ...base,
      event_type: 'block',
      title: 'Maintenance',
      platform: null,
    });
    expect(m).toMatchObject({ type: 'maintenance', blockReason: 'maintenance' });
    const o = mapCalendarEventToUi({
      ...base,
      event_type: 'block',
      title: 'Owner Use',
      platform: null,
    });
    expect(o).toMatchObject({ type: 'block', blockReason: 'owner_use' });
  });
});

describe('buildPropertyOptions', () => {
  const b = (id: string, name: string | null): RentalBuildingListItem => ({
    id,
    name,
    street: 'Hlavná 1',
    city: 'Košice',
    postal_code: '04001',
    status: 'active',
  });

  it('falls back to the address when a building has no name', () => {
    expect(buildingDisplayName(b('x', null))).toBe('Hlavná 1, Košice');
    expect(buildingDisplayName(b('x', '  '))).toBe('Hlavná 1, Košice');
  });

  it('sorts buildings and units naturally and flattens units', () => {
    const unit = (id: string, building_id: string, designation: string) => ({
      id,
      building_id,
      designation,
      floor: 1,
      unit_type: 'apartment',
      status: 'active',
    });
    const res = buildPropertyOptions([b('b2', 'Zelená'), b('b1', 'Astra')], {
      b1: [unit('u10', 'b1', '10'), unit('u2', 'b1', '2')],
      b2: [unit('u1', 'b2', 'A')],
    });
    expect(res.buildings).toEqual([
      { id: 'b1', name: 'Astra' },
      { id: 'b2', name: 'Zelená' },
    ]);
    expect(res.units.map((u) => `${u.buildingName}/${u.name}`)).toEqual([
      'Astra/2',
      'Astra/10',
      'Zelená/A',
    ]);
    expect(res.units[0]).toEqual({ id: 'u2', name: '2', buildingId: 'b1', buildingName: 'Astra' });
  });

  it('handles buildings without units', () => {
    expect(buildPropertyOptions([b('b1', 'A')], {}).units).toEqual([]);
  });
});

describe('computeReportPeriod', () => {
  it('monthly covers the calendar month, leap years included', () => {
    expect(computeReportPeriod('monthly', 2028, 2)).toEqual({
      periodStart: '2028-02-01',
      periodEnd: '2028-02-29',
    });
    expect(computeReportPeriod('monthly', 2026, 12)).toEqual({
      periodStart: '2026-12-01',
      periodEnd: '2026-12-31',
    });
  });

  it('quarterly and annual', () => {
    expect(computeReportPeriod('quarterly', 2026, 3)).toEqual({
      periodStart: '2026-07-01',
      periodEnd: '2026-09-30',
    });
    expect(computeReportPeriod('annual', 2026, 7)).toEqual({
      periodStart: '2026-01-01',
      periodEnd: '2026-12-31',
    });
  });

  it('clamps out-of-range indices', () => {
    expect(computeReportPeriod('quarterly', 2026, 9).periodStart).toBe('2026-10-01');
    expect(computeReportPeriod('monthly', 2026, 0).periodStart).toBe('2026-01-01');
  });
});

describe('report mappers', () => {
  it('maps a preview and sorts nationalities by count', () => {
    const p: RentalReportPreview = {
      building_id: 'b1',
      building_name: 'Astra',
      period_start: '2026-10-01',
      period_end: '2026-10-31',
      total_guests: 6,
      bookings_count: 3,
      by_nationality: [
        { nationality: 'DE', country_name: 'Germany', count: 1, percentage: 16.67 },
        { nationality: 'SK', country_name: 'Slovakia', count: 4, percentage: 66.67 },
        { nationality: 'AT', country_name: 'Austria', count: 1, percentage: 16.67 },
      ],
    };
    const ui = mapReportPreviewToUi(p);
    expect(ui).toMatchObject({ buildingName: 'Astra', totalGuests: 6, bookingsCount: 3 });
    expect(ui.byNationality.map((r) => r.nationality)).toEqual(['SK', 'AT', 'DE']);
  });

  it('maps a stored report summary', () => {
    const r: RentalReportSummary = {
      id: 'r1',
      building_id: 'b1',
      building_name: 'Astra',
      report_type: 'monthly',
      period_start: '2026-10-01',
      period_end: '2026-10-31',
      authority_code: 'SK_UHUL',
      authority_name: 'UHUL Slovakia',
      total_guests: 6,
      status: 'generated',
      report_file_url: null,
      submitted_at: null,
    };
    expect(mapReportSummaryToUi(r)).toEqual({
      id: 'r1',
      buildingId: 'b1',
      buildingName: 'Astra',
      periodType: 'monthly',
      periodStart: '2026-10-01',
      periodEnd: '2026-10-31',
      authorityCode: 'SK_UHUL',
      authorityName: 'UHUL Slovakia',
      totalGuests: 6,
      status: 'generated',
      fileUrl: undefined,
      submittedAt: undefined,
    });
  });

  it('localises country names and hides unknown ones', () => {
    expect(localizedCountryName('DE', 'Germany', 'sk')).toBe('Nemecko');
    expect(localizedCountryName('DE', 'Germany', 'en')).toBe('Germany');
    expect(localizedCountryName('UNK', 'Unknown', 'sk')).toBeNull();
  });
});
