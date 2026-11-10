/**
 * Regression tests for the rentals API↔UI mapper functions.
 *
 * The mappers translate the real api-server `/api/v1/rentals` payloads
 * (snake_case, decimal amounts as strings) into the page-level feature types.
 * They carry a few lossy, non-obvious folds a future refactor could break:
 *
 *   - The backend `rental_platform` has 'vrbo', which the page `BookingSource`
 *     lacks, so 'vrbo' (and anything unknown) collapses to 'other'.
 *   - The page `PlatformType` only has 'airbnb' | 'booking', so every
 *     non-'booking' platform collapses to 'airbnb'.
 *   - Connection status is derived from `is_active`, `sync_error` and
 *     `last_sync_at` (the summary row has no explicit sync status).
 *   - Guest 'registered' and 'reported' both read as registered; 'expired'
 *     reads as failed.
 *   - Occupancy arrives as a percentage but the page formats a fraction.
 *   - The guest-registration page only lists bookings with a pending guest.
 */
import type {
  RentalBookingRecord,
  RentalBookingSummary,
  RentalConnectionSummary,
  RentalGuestListItem,
} from '@ppt/api-client';
import { describe, expect, it } from 'vitest';
import {
  groupGuestsByBooking,
  mapBookingRecordToUi,
  mapBookingSummaryToUi,
  mapConnectionSummaryToUi,
  mapGuestFormToApi,
  mapGuestRecordToUi,
  mapGuestStatusToUi,
  mapRentalPlatformToSource,
  mapRentalPlatformToType,
  mapStatisticsToUi,
  parseRentalAmount,
} from './rentals';

describe('mapRentalPlatformToSource', () => {
  it('passes through the sources the page understands', () => {
    expect(mapRentalPlatformToSource('airbnb')).toBe('airbnb');
    expect(mapRentalPlatformToSource('booking')).toBe('booking');
    expect(mapRentalPlatformToSource('direct')).toBe('direct');
  });

  it("folds 'vrbo' and unknown values to 'other'", () => {
    expect(mapRentalPlatformToSource('vrbo')).toBe('other');
    expect(mapRentalPlatformToSource('something')).toBe('other');
  });
});

describe('mapRentalPlatformToType', () => {
  it("maps 'booking' to 'booking'", () => {
    expect(mapRentalPlatformToType('booking')).toBe('booking');
  });

  it.each(['airbnb', 'vrbo', 'direct'])("collapses '%s' to 'airbnb'", (platform) => {
    expect(mapRentalPlatformToType(platform)).toBe('airbnb');
  });
});

describe('parseRentalAmount', () => {
  it('parses decimal strings and drops empty/invalid values', () => {
    expect(parseRentalAmount('120.50')).toBe(120.5);
    expect(parseRentalAmount(null)).toBeUndefined();
    expect(parseRentalAmount(undefined)).toBeUndefined();
    expect(parseRentalAmount('')).toBeUndefined();
    expect(parseRentalAmount('abc')).toBeUndefined();
  });
});

const summary: RentalBookingSummary = {
  id: 'b-1',
  unit_id: 'u-1',
  unit_name: 'A-12',
  building_name: 'Main St 1',
  platform: 'airbnb',
  external_booking_id: 'ext-42',
  guest_name: 'Jane Doe',
  guest_count: 2,
  check_in: '2026-09-01',
  check_out: '2026-09-05',
  nights: 4,
  total_amount: '480.00',
  currency: 'EUR',
  status: 'confirmed',
  guest_registration_status: 'pending',
};

describe('mapBookingSummaryToUi', () => {
  it('maps a bookings-list row onto the page RentalBooking', () => {
    expect(mapBookingSummaryToUi(summary)).toEqual({
      id: 'b-1',
      unitId: 'u-1',
      unitName: 'A-12',
      buildingId: '',
      buildingName: 'Main St 1',
      guestName: 'Jane Doe',
      checkIn: '2026-09-01',
      checkOut: '2026-09-05',
      status: 'confirmed',
      source: 'airbnb',
      platformBookingId: 'ext-42',
      totalPrice: 480,
      currency: 'EUR',
      guestCount: 2,
      createdAt: '',
      updatedAt: '',
    });
  });

  it("routes 'vrbo' through the source fold", () => {
    expect(mapBookingSummaryToUi({ ...summary, platform: 'vrbo' }).source).toBe('other');
  });
});

describe('mapBookingRecordToUi', () => {
  const record: RentalBookingRecord = {
    id: 'b-1',
    organization_id: 'o-1',
    unit_id: 'u-1',
    platform: 'booking',
    external_booking_id: null,
    guest_name: 'Jane Doe',
    guest_email: 'jane@example.com',
    guest_phone: null,
    guest_count: 1,
    check_in: '2026-09-01',
    check_out: '2026-09-02',
    total_amount: null,
    currency: null,
    status: 'checked_in',
    guest_notes: 'late arrival',
    internal_notes: null,
    created_at: '2026-08-01T00:00:00Z',
    updated_at: '2026-08-02T00:00:00Z',
  };

  it('maps the full record, turning nulls into undefined', () => {
    const ui = mapBookingRecordToUi(record);
    expect(ui.source).toBe('booking');
    expect(ui.status).toBe('checked_in');
    expect(ui.guestEmail).toBe('jane@example.com');
    expect(ui.guestPhone).toBeUndefined();
    expect(ui.totalPrice).toBeUndefined();
    expect(ui.notes).toBe('late arrival');
    expect(ui.createdAt).toBe('2026-08-01T00:00:00Z');
  });
});

describe('mapConnectionSummaryToUi', () => {
  const base: RentalConnectionSummary = {
    id: 'c-1',
    unit_id: 'u-1',
    unit_name: 'A-12',
    platform: 'booking',
    is_active: true,
    last_sync_at: '2026-08-10T00:00:00Z',
    sync_error: null,
  };

  it('maps a synced active connection as connected', () => {
    expect(mapConnectionSummaryToUi(base)).toEqual({
      id: 'c-1',
      unitId: 'u-1',
      unitName: 'A-12',
      platform: 'booking',
      status: 'connected',
      lastSyncAt: '2026-08-10T00:00:00Z',
      syncErrorMessage: undefined,
      isAutoSyncEnabled: true,
      createdAt: '',
    });
  });

  it('derives disconnected / error / pending', () => {
    expect(mapConnectionSummaryToUi({ ...base, is_active: false }).status).toBe('disconnected');
    expect(mapConnectionSummaryToUi({ ...base, sync_error: 'boom' }).status).toBe('error');
    expect(mapConnectionSummaryToUi({ ...base, last_sync_at: null }).status).toBe('pending');
  });
});

describe('mapGuestStatusToUi', () => {
  it('folds the backend registration statuses', () => {
    expect(mapGuestStatusToUi('pending')).toBe('pending');
    expect(mapGuestStatusToUi('registered')).toBe('registered');
    expect(mapGuestStatusToUi('reported')).toBe('registered');
    expect(mapGuestStatusToUi('expired')).toBe('failed');
  });
});

function guestRow(overrides: Partial<RentalGuestListItem>): RentalGuestListItem {
  return {
    id: 'g-1',
    organization_id: 'o-1',
    booking_id: 'b-1',
    first_name: 'Jane',
    last_name: 'Doe',
    id_type: 'passport',
    id_number: 'P123',
    status: 'pending',
    is_primary: true,
    created_at: '2026-08-01T00:00:00Z',
    updated_at: '2026-08-01T00:00:00Z',
    booking_guest_name: 'Jane Doe',
    unit_id: 'u-1',
    unit_name: 'A-12',
    check_in: '2026-09-01',
    check_out: '2026-09-05',
    booking_status: 'confirmed',
    ...overrides,
  };
}

describe('mapGuestRecordToUi', () => {
  it('maps identity fields onto the page guest', () => {
    const ui = mapGuestRecordToUi(guestRow({}));
    expect(ui.bookingId).toBe('b-1');
    expect(ui.documentType).toBe('passport');
    expect(ui.documentNumber).toBe('P123');
    expect(ui.isPrimary).toBe(true);
    expect(ui.registrationStatus).toBe('pending');
  });
});

describe('groupGuestsByBooking', () => {
  it('groups by booking with booking context and drops fully registered bookings', () => {
    const rows = [
      guestRow({ id: 'g-1', booking_id: 'b-1' }),
      guestRow({ id: 'g-2', booking_id: 'b-1', is_primary: false, status: 'registered' }),
      guestRow({ id: 'g-3', booking_id: 'b-2', status: 'registered', booking_guest_name: 'X' }),
    ];
    const grouped = groupGuestsByBooking(rows);
    expect(grouped).toHaveLength(1);
    expect(grouped[0]).toMatchObject({
      id: 'b-1',
      guestName: 'Jane Doe',
      unitName: 'A-12',
      checkIn: '2026-09-01',
      checkOut: '2026-09-05',
    });
    expect(grouped[0].guests.map((g) => g.id)).toEqual(['g-1', 'g-2']);
  });
});

describe('mapGuestFormToApi', () => {
  it('renames page fields to the backend guest columns', () => {
    expect(
      mapGuestFormToApi({
        firstName: 'Jane',
        documentType: 'passport',
        documentNumber: 'P1',
        documentExpiry: '2030-01-01',
        address: 'Main 1',
        city: 'Bratislava',
        country: 'SVK',
      })
    ).toMatchObject({
      first_name: 'Jane',
      id_type: 'passport',
      id_number: 'P1',
      id_expiry_date: '2030-01-01',
      address_street: 'Main 1',
      address_city: 'Bratislava',
      address_country: 'SVK',
    });
  });
});

describe('mapStatisticsToUi', () => {
  it('converts occupancy percent to a fraction and derives the average stay', () => {
    const ui = mapStatisticsToUi(
      {
        total_units: 4,
        connected_units: 2,
        active_bookings: 1,
        upcoming_bookings: 3,
        pending_registrations: 5,
        occupancy_rate: 25,
        revenue_this_month: '1500.00',
        revenue_last_month: '900.00',
      },
      [summary, { ...summary, id: 'b-2', nights: 2 }],
      7
    );
    expect(ui).toEqual({
      totalBookings: 7,
      activeBookings: 1,
      upcomingBookings: 3,
      totalRevenue: 1500,
      occupancyRate: 0.25,
      averageStayDuration: 3,
      pendingGuestRegistrations: 5,
      currency: 'EUR',
    });
  });

  it('defaults to zeros while statistics are loading', () => {
    const ui = mapStatisticsToUi(undefined, [], 0);
    expect(ui.occupancyRate).toBe(0);
    expect(ui.totalRevenue).toBe(0);
    expect(ui.currency).toBe('EUR');
  });
});
