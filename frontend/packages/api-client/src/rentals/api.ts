/**
 * Short-Term Rentals API client (Epic 18, UC-29/30).
 *
 * Thin wrappers over the real api-server routes mounted at `/api/v1/rentals`
 * (`backend/servers/api-server/src/routes/rentals.rs`). Every route resolves
 * the tenant from `X-Tenant-ID`, which `authenticatedFetchJson` does NOT add on
 * its own (#3006) — so each call takes the caller's auth headers explicitly.
 */

import { authenticatedFetchJson } from '../lib/fetch';
import type {
  RentalBookingListQuery,
  RentalBookingRecord,
  RentalBookingsPage,
  RentalBookingWithGuests,
  RentalConnectionSummary,
  RentalCreateConnectionBody,
  RentalCreateGuestBody,
  RentalDashboardStatistics,
  RentalGuestListItem,
  RentalGuestListQuery,
  RentalGuestRecord,
  RentalUpdateBookingStatusBody,
  RentalUpdateGuestBody,
} from './types';

const RENTALS_BASE = '/api/v1/rentals';

/** Auth + tenant headers every rentals call must carry. */
export type RentalsRequestHeaders = Record<string, string>;

function qs(params: object): string {
  const search = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== null && value !== '') {
      search.append(key, String(value));
    }
  }
  const s = search.toString();
  return s ? `?${s}` : '';
}

export function getRentalStatistics(
  headers: RentalsRequestHeaders
): Promise<RentalDashboardStatistics> {
  return authenticatedFetchJson(`${RENTALS_BASE}/statistics`, { headers });
}

export function listRentalBookings(
  query: RentalBookingListQuery,
  headers: RentalsRequestHeaders
): Promise<RentalBookingsPage> {
  return authenticatedFetchJson(`${RENTALS_BASE}/bookings${qs(query)}`, { headers });
}

export function getRentalBookingWithGuests(
  id: string,
  headers: RentalsRequestHeaders
): Promise<RentalBookingWithGuests> {
  return authenticatedFetchJson(`${RENTALS_BASE}/bookings/${encodeURIComponent(id)}/guests`, {
    headers,
  });
}

export function updateRentalBookingStatus(
  id: string,
  body: RentalUpdateBookingStatusBody,
  headers: RentalsRequestHeaders
): Promise<RentalBookingRecord> {
  return authenticatedFetchJson(`${RENTALS_BASE}/bookings/${encodeURIComponent(id)}/status`, {
    method: 'PUT',
    body: JSON.stringify(body),
    headers,
  });
}

export function listRentalGuests(
  query: RentalGuestListQuery,
  headers: RentalsRequestHeaders
): Promise<RentalGuestListItem[]> {
  return authenticatedFetchJson(`${RENTALS_BASE}/guests${qs(query)}`, { headers });
}

export function createRentalGuest(
  body: RentalCreateGuestBody,
  headers: RentalsRequestHeaders
): Promise<RentalGuestRecord> {
  return authenticatedFetchJson(`${RENTALS_BASE}/guests`, {
    method: 'POST',
    body: JSON.stringify(body),
    headers,
  });
}

export function updateRentalGuest(
  id: string,
  body: RentalUpdateGuestBody,
  headers: RentalsRequestHeaders
): Promise<RentalGuestRecord> {
  return authenticatedFetchJson(`${RENTALS_BASE}/guests/${encodeURIComponent(id)}`, {
    method: 'PUT',
    body: JSON.stringify(body),
    headers,
  });
}

export function registerRentalGuest(
  id: string,
  headers: RentalsRequestHeaders
): Promise<RentalGuestRecord> {
  return authenticatedFetchJson(`${RENTALS_BASE}/guests/${encodeURIComponent(id)}/register`, {
    method: 'POST',
    headers,
  });
}

export function listRentalConnections(
  headers: RentalsRequestHeaders
): Promise<RentalConnectionSummary[]> {
  return authenticatedFetchJson(`${RENTALS_BASE}/connections`, { headers });
}

export function createRentalConnection(
  body: RentalCreateConnectionBody,
  headers: RentalsRequestHeaders
): Promise<unknown> {
  return authenticatedFetchJson(`${RENTALS_BASE}/connections`, {
    method: 'POST',
    body: JSON.stringify(body),
    headers,
  });
}

export function deleteRentalGuest(id: string, headers: RentalsRequestHeaders): Promise<void> {
  return authenticatedFetchJson(`${RENTALS_BASE}/guests/${encodeURIComponent(id)}`, {
    method: 'DELETE',
    headers,
  });
}
