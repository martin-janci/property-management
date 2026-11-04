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
  RentalAvailability,
  RentalBookingListQuery,
  RentalBookingRecord,
  RentalBookingsPage,
  RentalBookingWithGuests,
  RentalBuildingsPage,
  RentalBuildingUnitsPage,
  RentalCalendarBlock,
  RentalCalendarEvent,
  RentalCalendarQuery,
  RentalConnectionSummary,
  RentalCreateCalendarBlockBody,
  RentalCreateConnectionBody,
  RentalCreateGuestBody,
  RentalDashboardStatistics,
  RentalGenerateReportBody,
  RentalGuestListItem,
  RentalGuestListQuery,
  RentalGuestRecord,
  RentalGuestReport,
  RentalReportListQuery,
  RentalReportPreview,
  RentalReportPreviewBody,
  RentalReportSummary,
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

// ============================================
// Calendar
// ============================================

/** `GET /rentals/calendar/{unit_id}` — bookings + blocks overlapping the range. */
export function getRentalCalendar(
  unitId: string,
  query: RentalCalendarQuery,
  headers: RentalsRequestHeaders
): Promise<RentalCalendarEvent[]> {
  return authenticatedFetchJson(
    `${RENTALS_BASE}/calendar/${encodeURIComponent(unitId)}${qs(query)}`,
    { headers }
  );
}

/**
 * `GET /rentals/calendar/{unit_id}/availability` — whether the half-open range
 * `[start_date, end_date)` is free of reservations and blocks.
 */
export function checkRentalAvailability(
  unitId: string,
  query: RentalCalendarQuery,
  headers: RentalsRequestHeaders
): Promise<RentalAvailability> {
  return authenticatedFetchJson(
    `${RENTALS_BASE}/calendar/${encodeURIComponent(unitId)}/availability${qs(query)}`,
    { headers }
  );
}

/** `POST /rentals/calendar/blocks` — block dates for owner use / maintenance. */
export function createRentalCalendarBlock(
  body: RentalCreateCalendarBlockBody,
  headers: RentalsRequestHeaders
): Promise<RentalCalendarBlock> {
  return authenticatedFetchJson(`${RENTALS_BASE}/calendar/blocks`, {
    method: 'POST',
    body: JSON.stringify(body),
    headers,
  });
}

/** `DELETE /rentals/calendar/blocks/{id}` — 404s for booking-linked blocks. */
export function deleteRentalCalendarBlock(
  id: string,
  headers: RentalsRequestHeaders
): Promise<void> {
  return authenticatedFetchJson(`${RENTALS_BASE}/calendar/blocks/${encodeURIComponent(id)}`, {
    method: 'DELETE',
    headers,
  });
}

// ============================================
// Authority guest reports
// ============================================

/** `GET /rentals/reports` — saved reports, newest first. */
export function listRentalReports(
  query: RentalReportListQuery,
  headers: RentalsRequestHeaders
): Promise<RentalReportSummary[]> {
  return authenticatedFetchJson(`${RENTALS_BASE}/reports${qs(query)}`, { headers });
}

/** `POST /rentals/reports/preview` — guest counts for a building + period, nothing stored. */
export function previewRentalReport(
  body: RentalReportPreviewBody,
  headers: RentalsRequestHeaders
): Promise<RentalReportPreview> {
  return authenticatedFetchJson(`${RENTALS_BASE}/reports/preview`, {
    method: 'POST',
    body: JSON.stringify(body),
    headers,
  });
}

/** `POST /rentals/reports` — store a report (status `generated`). */
export function createRentalReport(
  body: RentalGenerateReportBody,
  headers: RentalsRequestHeaders
): Promise<RentalGuestReport> {
  return authenticatedFetchJson(`${RENTALS_BASE}/reports`, {
    method: 'POST',
    body: JSON.stringify(body),
    headers,
  });
}

/** `POST /rentals/reports/{id}/submit` — mark a stored report as submitted. */
export function submitRentalReport(
  id: string,
  headers: RentalsRequestHeaders
): Promise<RentalGuestReport> {
  return authenticatedFetchJson(`${RENTALS_BASE}/reports/${encodeURIComponent(id)}/submit`, {
    method: 'POST',
    headers,
  });
}

// ============================================
// Building / unit pickers for the rentals pages
// ============================================

/**
 * `GET /api/v1/buildings?organization_id=…` — the server rejects a missing or
 * foreign `organization_id`, and caps `limit` at 100.
 */
export function listRentalBuildings(
  organizationId: string,
  headers: RentalsRequestHeaders
): Promise<RentalBuildingsPage> {
  return authenticatedFetchJson(
    `/api/v1/buildings${qs({ organization_id: organizationId, limit: 100 })}`,
    { headers }
  );
}

/** `GET /api/v1/buildings/{id}/units` — active units, `limit` capped at 100. */
export function listRentalBuildingUnits(
  buildingId: string,
  headers: RentalsRequestHeaders
): Promise<RentalBuildingUnitsPage> {
  return authenticatedFetchJson(
    `/api/v1/buildings/${encodeURIComponent(buildingId)}/units${qs({ limit: 100 })}`,
    { headers }
  );
}
