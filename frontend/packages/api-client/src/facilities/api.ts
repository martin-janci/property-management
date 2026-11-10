/**
 * Facilities API client (Epic 56: Facility Booking).
 *
 * Client functions for facility management and booking.
 */

import { authenticatedFetch } from '../lib/fetch';
import type {
  AvailabilityQuery,
  AvailableSlot,
  BookingListResponse,
  BookingWithDetails,
  CancelBookingRequest,
  CreateBookingRequest,
  CreateFacilityRequest,
  Facility,
  FacilityListResponse,
  ListBookingsQuery,
  ListFacilitiesQuery,
  ListPendingBookingsQuery,
  MyBookingsQuery,
  RejectBookingRequest,
  UpdateBookingRequest,
  UpdateFacilityRequest,
} from './types';

const API_BASE = '/api/v1';

// Authorization and X-Tenant-ID come from the shared token/org providers via
// `authenticatedFetch`, the same as the generated SDK.

async function fetchApi<T>(url: string, options?: RequestInit): Promise<T> {
  const response = await authenticatedFetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
  });

  if (!response.ok) {
    let errorMessage = `HTTP ${response.status}`;
    try {
      const error = await response.json();
      errorMessage = error.message || errorMessage;
    } catch {
      // Non-JSON response, use status text
      errorMessage = response.statusText || errorMessage;
    }
    throw new Error(errorMessage);
  }

  return response.json();
}

// ============================================================================
// RESPONSE NORMALIZATION
// ============================================================================

/**
 * The api-server list endpoints (`GET /buildings/{id}/facilities`,
 * `/buildings/{id}/bookings/pending`, `/bookings/my`, facility bookings) return
 * a bare JSON array, while the client types promise `{ items, total }`. Accept
 * both shapes so pages can always read `response.items`.
 */
export function normalizeFacilityList<T>(raw: unknown): { items: T[]; total: number } {
  if (Array.isArray(raw)) {
    return { items: raw as T[], total: raw.length };
  }
  const obj = (raw ?? {}) as { items?: unknown; total?: unknown };
  const items = Array.isArray(obj.items) ? (obj.items as T[]) : [];
  return { items, total: typeof obj.total === 'number' ? obj.total : items.length };
}

/**
 * Turn a `YYYY-MM-DD` calendar day into the `[from, to)` RFC 3339 range the
 * availability endpoint requires. The range covers the whole day in the
 * browser's local time zone, which is how the booking form interprets times.
 */
export function facilityDayRange(date: string): { from: string; to: string } {
  const [y, m, d] = date.split('-').map((part) => Number.parseInt(part, 10));
  const start = new Date(y, (m ?? 1) - 1, d ?? 1);
  const end = new Date(y, (m ?? 1) - 1, (d ?? 1) + 1);
  return { from: start.toISOString(), to: end.toISOString() };
}

// ============================================================================
// FACILITY MANAGEMENT
// ============================================================================

// The UI edits opening days as a Mon..Sun bitmask (1 = Monday ... 64 = Sunday),
// while the server stores `available_days` as an array of weekday numbers
// (0 = Sunday, 1 = Monday ... 6 = Saturday; `null` = every day). Sending the
// bitmask as-is made every create/update fail with 422, and reading the array
// as a number broke the edit form, so convert at this boundary.
const DAY_BITS: ReadonlyArray<readonly [bit: number, weekday: number]> = [
  [1, 1],
  [2, 2],
  [4, 3],
  [8, 4],
  [16, 5],
  [32, 6],
  [64, 0],
];

export function daysMaskToWire(mask: number | undefined | null): number[] | undefined {
  if (mask === undefined || mask === null) return undefined;
  return DAY_BITS.filter(([bit]) => (mask & bit) !== 0).map(([, weekday]) => weekday);
}

export function daysWireToMask(days: unknown): number | null {
  if (typeof days === 'number') return days;
  if (!Array.isArray(days)) return null;
  return DAY_BITS.reduce((mask, [bit, weekday]) => (days.includes(weekday) ? mask | bit : mask), 0);
}

function facilityFromWire(raw: Facility): Facility {
  return { ...raw, available_days: daysWireToMask(raw.available_days) };
}

function facilityToWire<T extends { available_days?: number }>(data: T) {
  return { ...data, available_days: daysMaskToWire(data.available_days) };
}

/**
 * List facilities in a building.
 */
export async function listFacilities(
  buildingId: string,
  params?: ListFacilitiesQuery
): Promise<FacilityListResponse> {
  const searchParams = new URLSearchParams();
  if (params?.facility_type) searchParams.set('facility_type', params.facility_type);
  if (params?.is_bookable !== undefined)
    searchParams.set('is_bookable', String(params.is_bookable));
  if (params?.is_active !== undefined) searchParams.set('is_active', String(params.is_active));
  if (params?.limit !== undefined) searchParams.set('limit', String(params.limit));
  if (params?.offset !== undefined) searchParams.set('offset', String(params.offset));
  const query = searchParams.toString();
  return normalizeFacilityList(
    await fetchApi<unknown>(
      `${API_BASE}/buildings/${buildingId}/facilities${query ? `?${query}` : ''}`
    )
  );
}

/**
 * Get a single facility by ID.
 */
export async function getFacility(buildingId: string, facilityId: string): Promise<Facility> {
  return facilityFromWire(
    await fetchApi<Facility>(`${API_BASE}/buildings/${buildingId}/facilities/${facilityId}`)
  );
}

/**
 * Create a new facility.
 */
export async function createFacility(
  buildingId: string,
  data: CreateFacilityRequest
): Promise<Facility> {
  return facilityFromWire(
    await fetchApi<Facility>(`${API_BASE}/buildings/${buildingId}/facilities`, {
      method: 'POST',
      body: JSON.stringify(facilityToWire(data)),
    })
  );
}

/**
 * Update a facility.
 */
export async function updateFacility(
  buildingId: string,
  facilityId: string,
  data: UpdateFacilityRequest
): Promise<Facility> {
  return facilityFromWire(
    await fetchApi<Facility>(`${API_BASE}/buildings/${buildingId}/facilities/${facilityId}`, {
      method: 'PUT',
      body: JSON.stringify(facilityToWire(data)),
    })
  );
}

/**
 * Delete a facility.
 */
export async function deleteFacility(buildingId: string, facilityId: string): Promise<void> {
  await fetchApi(`${API_BASE}/buildings/${buildingId}/facilities/${facilityId}`, {
    method: 'DELETE',
  });
}

// ============================================================================
// FACILITY AVAILABILITY
// ============================================================================

/**
 * Check availability for a facility on a specific date.
 */
export async function checkAvailability(
  buildingId: string,
  facilityId: string,
  params: AvailabilityQuery
): Promise<AvailableSlot[]> {
  const range =
    params.from && params.to ? { from: params.from, to: params.to } : facilityDayRange(params.date);
  const searchParams = new URLSearchParams();
  searchParams.set('from', range.from);
  searchParams.set('to', range.to);
  return fetchApi(
    `${API_BASE}/buildings/${buildingId}/facilities/${facilityId}/availability?${searchParams}`
  );
}

// ============================================================================
// FACILITY BOOKINGS
// ============================================================================

/**
 * List bookings for a facility.
 */
export async function listFacilityBookings(
  buildingId: string,
  facilityId: string,
  params?: ListBookingsQuery
): Promise<BookingListResponse> {
  const searchParams = new URLSearchParams();
  if (params?.from_date) searchParams.set('from_date', params.from_date);
  if (params?.to_date) searchParams.set('to_date', params.to_date);
  if (params?.status) searchParams.set('status', params.status);
  const query = searchParams.toString();
  return normalizeFacilityList(
    await fetchApi<unknown>(
      `${API_BASE}/buildings/${buildingId}/facilities/${facilityId}/bookings${query ? `?${query}` : ''}`
    )
  );
}

/**
 * Create a new booking for a facility.
 */
export async function createBooking(
  buildingId: string,
  facilityId: string,
  data: CreateBookingRequest
): Promise<BookingWithDetails> {
  return fetchApi(`${API_BASE}/buildings/${buildingId}/facilities/${facilityId}/bookings`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

/**
 * Get current user's bookings.
 */
export async function getMyBookings(params?: MyBookingsQuery): Promise<BookingListResponse> {
  const searchParams = new URLSearchParams();
  if (params?.status) searchParams.set('status', params.status);
  if (params?.limit !== undefined) searchParams.set('limit', String(params.limit));
  if (params?.offset !== undefined) searchParams.set('offset', String(params.offset));
  const query = searchParams.toString();
  return normalizeFacilityList(
    await fetchApi<unknown>(`${API_BASE}/bookings/my${query ? `?${query}` : ''}`)
  );
}

/**
 * Get a booking by ID.
 */
export async function getBooking(bookingId: string): Promise<BookingWithDetails> {
  return fetchApi(`${API_BASE}/bookings/${bookingId}`);
}

/**
 * Update a booking.
 */
export async function updateBooking(
  bookingId: string,
  data: UpdateBookingRequest
): Promise<BookingWithDetails> {
  return fetchApi(`${API_BASE}/bookings/${bookingId}`, {
    method: 'PUT',
    body: JSON.stringify(data),
  });
}

/**
 * Cancel a booking.
 */
export async function cancelBooking(
  bookingId: string,
  data?: CancelBookingRequest
): Promise<BookingWithDetails> {
  return fetchApi(`${API_BASE}/bookings/${bookingId}/cancel`, {
    method: 'POST',
    body: JSON.stringify(data || {}),
  });
}

// ============================================================================
// BOOKING APPROVAL WORKFLOW
// ============================================================================

/**
 * List pending bookings for a building (for managers).
 */
export async function listPendingBookings(
  buildingId: string,
  params?: ListPendingBookingsQuery
): Promise<BookingListResponse> {
  const searchParams = new URLSearchParams();
  if (params?.limit !== undefined) searchParams.set('limit', String(params.limit));
  if (params?.offset !== undefined) searchParams.set('offset', String(params.offset));
  const query = searchParams.toString();
  return normalizeFacilityList(
    await fetchApi<unknown>(
      `${API_BASE}/buildings/${buildingId}/bookings/pending${query ? `?${query}` : ''}`
    )
  );
}

/**
 * Approve a pending booking.
 */
export async function approveBooking(bookingId: string): Promise<BookingWithDetails> {
  return fetchApi(`${API_BASE}/bookings/${bookingId}/approve`, {
    method: 'POST',
  });
}

/**
 * Reject a pending booking.
 */
export async function rejectBooking(
  bookingId: string,
  data: RejectBookingRequest
): Promise<BookingWithDetails> {
  return fetchApi(`${API_BASE}/bookings/${bookingId}/reject`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}
