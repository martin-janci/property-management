/**
 * Tests for the connection-settings and booking-form mappers.
 */
import type { RentalBookingRecord, RentalConnectionDetail } from '@ppt/api-client';
import { describe, expect, it } from 'vitest';
import { formatLastSync } from './components/PlatformConnectionCard';
import {
  bookingFormToCreateBody,
  bookingFormToUpdateBody,
  bookingRecordToForm,
  emptyBookingForm,
  isValidListingUrl,
  mapConnectionDetailToSettings,
  mapConnectionSettingsToApi,
  validateBookingForm,
} from './mappers';

const detail: RentalConnectionDetail = {
  id: 'c1',
  organization_id: 'o1',
  unit_id: 'u1',
  platform: 'booking',
  is_connected: true,
  external_property_id: null,
  external_listing_url: 'https://booking.com/hotel/x',
  is_active: true,
  sync_calendar: false,
  sync_interval_minutes: 30,
  block_other_platforms: true,
  created_at: '2026-10-01T00:00:00Z',
  updated_at: '2026-10-01T00:00:00Z',
};

describe('connection settings mappers', () => {
  it('maps the detail into the form, nulls as empty strings', () => {
    expect(mapConnectionDetailToSettings(detail)).toEqual({
      externalPropertyId: '',
      externalListingUrl: 'https://booking.com/hotel/x',
      syncCalendar: false,
      syncIntervalMinutes: 30,
      blockOtherPlatforms: true,
    });
  });

  it('omits text fields that stay empty, trims and sends cleared ones as ""', () => {
    const prev = mapConnectionDetailToSettings(detail);
    expect(
      mapConnectionSettingsToApi(
        { ...prev, externalPropertyId: '  ', externalListingUrl: '' },
        prev
      )
    ).toEqual({
      sync_calendar: false,
      sync_interval_minutes: 30,
      block_other_platforms: true,
      external_listing_url: '',
    });
    expect(mapConnectionSettingsToApi({ ...prev, externalPropertyId: ' 77 ' }, prev)).toMatchObject(
      { external_property_id: '77' }
    );
  });

  it('accepts empty or http(s) listing URLs only', () => {
    expect(isValidListingUrl('')).toBe(true);
    expect(isValidListingUrl('https://airbnb.com/rooms/1')).toBe(true);
    expect(isValidListingUrl('http://example.com')).toBe(true);
    expect(isValidListingUrl('airbnb.com/rooms/1')).toBe(false);
    expect(isValidListingUrl('javascript:alert(1)')).toBe(false);
  });
});

describe('booking form mappers', () => {
  const valid = {
    ...emptyBookingForm('u1'),
    guestName: ' Jana Nováková ',
    checkIn: '2026-11-03',
    checkOut: '2026-11-05',
    totalPrice: '240,50',
  };

  it('validates required fields, date order, guest count and price', () => {
    expect(validateBookingForm(valid)).toEqual([]);
    expect(validateBookingForm(emptyBookingForm())).toEqual(['unit', 'guestName', 'dates']);
    expect(validateBookingForm({ ...valid, checkOut: '2026-11-03' })).toEqual(['dateOrder']);
    expect(validateBookingForm({ ...valid, guestCount: 0 })).toEqual(['guestCount']);
    expect(validateBookingForm({ ...valid, guestCount: Number.NaN })).toEqual(['guestCount']);
    expect(validateBookingForm({ ...valid, totalPrice: 'abc' })).toEqual(['price']);
    expect(validateBookingForm({ ...valid, totalPrice: '-5' })).toEqual(['price']);
  });

  it('builds the create body (decimal comma, no empty optionals, other → direct)', () => {
    expect(bookingFormToCreateBody({ ...valid, source: 'other' })).toEqual({
      unit_id: 'u1',
      platform: 'direct',
      guest_name: 'Jana Nováková',
      guest_email: undefined,
      guest_phone: undefined,
      guest_count: 1,
      check_in: '2026-11-03',
      check_out: '2026-11-05',
      total_amount: '240.50',
      currency: 'EUR',
      guest_notes: undefined,
      internal_notes: undefined,
    });
    expect(bookingFormToCreateBody({ ...valid, totalPrice: '' }).currency).toBeUndefined();
  });

  it('round-trips a record into the edit form and sends cleared notes as ""', () => {
    const rec = {
      id: 'b1',
      organization_id: 'o1',
      unit_id: 'u1',
      platform: 'airbnb',
      guest_name: 'Jana',
      guest_email: 'jana@example.sk',
      guest_phone: null,
      guest_count: 2,
      check_in: '2026-11-03',
      check_out: '2026-11-05',
      total_amount: '200.00',
      currency: 'EUR',
      status: 'confirmed',
      guest_notes: 'Late arrival',
      internal_notes: null,
      created_at: '',
      updated_at: '',
    } as RentalBookingRecord;
    const form = bookingRecordToForm(rec);
    expect(form).toMatchObject({ source: 'airbnb', guestEmail: 'jana@example.sk', guestPhone: '' });
    expect(bookingFormToUpdateBody({ ...form, guestNotes: '', guestCount: 3 }, form)).toEqual({
      guest_name: 'Jana',
      guest_email: 'jana@example.sk',
      guest_phone: undefined,
      guest_count: 3,
      check_in: '2026-11-03',
      check_out: '2026-11-05',
      total_amount: '200.00',
      currency: 'EUR',
      guest_notes: '',
      internal_notes: undefined,
    });
  });
});

describe('formatLastSync', () => {
  const now = new Date('2026-11-04T12:00:00Z');
  it('formats relative times in the UI language', () => {
    expect(formatLastSync(undefined, 'en', 'Just now', now)).toBe('-');
    expect(formatLastSync('2026-11-04T11:59:40Z', 'en', 'Just now', now)).toBe('Just now');
    expect(formatLastSync('2026-11-04T11:55:00Z', 'en', 'Just now', now)).toBe('5 minutes ago');
    expect(formatLastSync('2026-11-04T09:00:00Z', 'sk', 'Práve teraz', now)).toBe(
      'pred 3 hodinami'
    );
    expect(formatLastSync('2026-11-02T12:00:00Z', 'de', 'Gerade eben', now)).toBe('vorgestern');
  });
});
