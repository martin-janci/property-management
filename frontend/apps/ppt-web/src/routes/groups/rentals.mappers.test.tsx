/**
 * Regression tests for the rentals API↔UI mapper functions.
 *
 * `rentals.tsx` describes itself as owning "the rentals API↔UI mappers", yet
 * until now only the mutation auth-guard seam had coverage (see
 * `rentals.auth-guard.test.tsx`). This churn-hotspot file's mappers translate
 * the generated `@ppt/api-client` payloads into the page-level feature types,
 * and they carry a few *lossy, non-obvious* folds that a future refactor could
 * silently break:
 *
 *   - `RentalsRentalPlatform` has five variants ('airbnb' | 'booking' | 'vrbo' |
 *     'direct' | 'other') but the page `BookingSource` lacks 'vrbo', so 'vrbo'
 *     must collapse to 'other'.
 *   - The page `PlatformType` only has 'airbnb' | 'booking', so every non-'booking'
 *     platform (including 'vrbo' and 'direct') must collapse to 'airbnb'.
 *   - A platform connection reports 'disconnected' whenever it is inactive,
 *     regardless of the underlying `syncStatus`.
 *   - A guest's registration status is derived purely from
 *     `submittedToAuthorities`, and a guest with no `reservationId` groups under
 *     an empty booking id.
 *
 * These tests pin that behaviour so the folds cannot regress unnoticed.
 */
import type {
  RentalsGuestRegistration,
  RentalsPlatformConnection,
  RentalsRentalPlatform,
  RentalsReservation,
  RentalsSyncStatus,
} from '@ppt/api-client';
import { describe, expect, it } from 'vitest';
import {
  mapApiConnectionToUi,
  mapApiGuestToUi,
  mapRentalPlatformToSource,
  mapRentalPlatformToType,
  mapReservationToBooking,
  mapSyncStatusToConnectionStatus,
} from './rentals';

describe('mapRentalPlatformToSource', () => {
  it('passes through the sources the page understands', () => {
    expect(mapRentalPlatformToSource('airbnb')).toBe('airbnb');
    expect(mapRentalPlatformToSource('booking')).toBe('booking');
    expect(mapRentalPlatformToSource('direct')).toBe('direct');
  });

  it("folds 'vrbo' (absent from BookingSource) to 'other'", () => {
    expect(mapRentalPlatformToSource('vrbo')).toBe('other');
  });

  it("folds the generated 'other' platform to 'other'", () => {
    expect(mapRentalPlatformToSource('other')).toBe('other');
  });
});

describe('mapRentalPlatformToType', () => {
  it("maps 'booking' to 'booking'", () => {
    expect(mapRentalPlatformToType('booking')).toBe('booking');
  });

  it.each<RentalsRentalPlatform>(['airbnb', 'vrbo', 'direct', 'other'])(
    "collapses non-booking platform '%s' to 'airbnb' (PlatformType has only airbnb|booking)",
    (platform) => {
      expect(mapRentalPlatformToType(platform)).toBe('airbnb');
    }
  );
});

describe('mapSyncStatusToConnectionStatus', () => {
  it("reports 'disconnected' whenever the connection is inactive, regardless of syncStatus", () => {
    for (const status of ['synced', 'pending', 'error', 'disabled'] as RentalsSyncStatus[]) {
      expect(mapSyncStatusToConnectionStatus(status, false)).toBe('disconnected');
    }
  });

  it('maps active connections by their syncStatus', () => {
    expect(mapSyncStatusToConnectionStatus('synced', true)).toBe('connected');
    expect(mapSyncStatusToConnectionStatus('pending', true)).toBe('pending');
    expect(mapSyncStatusToConnectionStatus('error', true)).toBe('error');
  });

  it("falls an active-but-'disabled' connection back to 'disconnected'", () => {
    expect(mapSyncStatusToConnectionStatus('disabled', true)).toBe('disconnected');
  });
});

describe('mapReservationToBooking', () => {
  const base: RentalsReservation = {
    id: 'res-1',
    unitId: 'unit-1',
    platform: 'airbnb',
    externalId: 'ext-42',
    guestName: 'Jane Doe',
    guestEmail: 'jane@example.com',
    guestPhone: '+421900000000',
    guestCount: 2,
    checkIn: '2026-09-01T14:00:00Z',
    checkOut: '2026-09-05T10:00:00Z',
    status: 'confirmed',
    totalPrice: { amount: 12000, currency: 'EUR' },
    notes: 'late arrival',
    createdAt: '2026-08-01T00:00:00Z',
    updatedAt: '2026-08-02T00:00:00Z',
  };

  it('maps the reservation payload onto the page RentalBooking shape', () => {
    expect(mapReservationToBooking(base)).toEqual({
      id: 'res-1',
      unitId: 'unit-1',
      buildingId: '',
      guestName: 'Jane Doe',
      guestEmail: 'jane@example.com',
      guestPhone: '+421900000000',
      checkIn: '2026-09-01T14:00:00Z',
      checkOut: '2026-09-05T10:00:00Z',
      status: 'confirmed',
      source: 'airbnb',
      platformBookingId: 'ext-42',
      totalPrice: 12000,
      currency: 'EUR',
      guestCount: 2,
      notes: 'late arrival',
      createdAt: '2026-08-01T00:00:00Z',
      updatedAt: '2026-08-02T00:00:00Z',
    });
  });

  it("routes the platform through the source fold ('vrbo' → 'other')", () => {
    expect(mapReservationToBooking({ ...base, platform: 'vrbo' }).source).toBe('other');
  });

  it('leaves buildingId blank — the reservation payload carries no building context', () => {
    expect(mapReservationToBooking(base).buildingId).toBe('');
  });

  it('carries the external id through as the platform booking id', () => {
    expect(mapReservationToBooking(base).platformBookingId).toBe('ext-42');
  });
});

describe('mapApiConnectionToUi', () => {
  const base: RentalsPlatformConnection = {
    id: 'conn-1',
    unitId: 'unit-1',
    platform: 'booking',
    externalListingId: 'listing-9',
    isActive: true,
    lastSyncAt: '2026-08-10T00:00:00Z',
    syncStatus: 'synced',
    createdAt: '2026-08-01T00:00:00Z',
    updatedAt: '2026-08-02T00:00:00Z',
  };

  it('maps the connection payload onto the page PlatformConnection shape', () => {
    expect(mapApiConnectionToUi(base)).toEqual({
      id: 'conn-1',
      unitId: 'unit-1',
      platform: 'booking',
      status: 'connected',
      platformPropertyId: 'listing-9',
      lastSyncAt: '2026-08-10T00:00:00Z',
      isAutoSyncEnabled: true,
      createdAt: '2026-08-01T00:00:00Z',
    });
  });

  it('reports an inactive connection as disconnected even when synced', () => {
    const ui = mapApiConnectionToUi({ ...base, isActive: false });
    expect(ui.status).toBe('disconnected');
    expect(ui.isAutoSyncEnabled).toBe(false);
  });

  it("folds a 'vrbo' connection platform to the page 'airbnb' type", () => {
    expect(mapApiConnectionToUi({ ...base, platform: 'vrbo' }).platform).toBe('airbnb');
  });
});

describe('mapApiGuestToUi', () => {
  const base: RentalsGuestRegistration = {
    id: 'guest-1',
    reservationId: 'res-1',
    unitId: 'unit-1',
    firstName: 'Jane',
    lastName: 'Doe',
    dateOfBirth: '1990-01-01',
    nationality: 'SK',
    documentType: 'passport',
    documentNumber: 'P1234567',
    documentExpiry: '2030-01-01',
    arrivalDate: '2026-09-01',
    departureDate: '2026-09-05',
    submittedToAuthorities: true,
    submittedAt: '2026-08-11T00:00:00Z',
    createdAt: '2026-08-01T00:00:00Z',
    updatedAt: '2026-08-02T00:00:00Z',
  };

  it('maps a submitted guest to registered status with the submission timestamp', () => {
    expect(mapApiGuestToUi(base)).toEqual({
      id: 'guest-1',
      bookingId: 'res-1',
      firstName: 'Jane',
      lastName: 'Doe',
      dateOfBirth: '1990-01-01',
      nationality: 'SK',
      documentType: 'passport',
      documentNumber: 'P1234567',
      documentExpiry: '2030-01-01',
      registrationStatus: 'registered',
      registeredAt: '2026-08-11T00:00:00Z',
      isPrimary: false,
      createdAt: '2026-08-01T00:00:00Z',
    });
  });

  it("marks an un-submitted guest as 'pending'", () => {
    expect(
      mapApiGuestToUi({ ...base, submittedToAuthorities: false, submittedAt: undefined })
        .registrationStatus
    ).toBe('pending');
  });

  it('falls a guest with no reservationId back to an empty booking id', () => {
    expect(mapApiGuestToUi({ ...base, reservationId: undefined }).bookingId).toBe('');
  });
});
