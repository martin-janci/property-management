/**
 * Person-months API client functions (Epic 3, Story 3.5).
 *
 * Thin typed `fetch` wrappers over the building-nested person-months endpoints
 * (`/api/v1/buildings/{building_id}/...`). Mirrors the structure of the meters
 * module (its own `fetchApi`/`buildQueryString` helpers) so the two stay
 * consistent.
 */

import type {
  BuildingPersonMonthsQuery,
  BulkPersonMonthUpsertResult,
  BulkUpsertPersonMonthsRequest,
  CalculatePersonMonthRequest,
  GetUnitPersonMonthsQuery,
  PersonMonth,
  PersonMonthBuildingSummary,
  PersonMonthWithUnit,
  UpdatePersonMonthRequest,
  UpsertPersonMonthRequest,
  YearlyPersonMonthSummary,
} from './types';

const API_BASE = '/api/v1/buildings';

/** Build a query string from params, skipping empty values. */
function buildQueryString(
  params: Record<string, string | number | boolean | undefined | null>
): string {
  const searchParams = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== null && value !== '') {
      searchParams.append(key, String(value));
    }
  }
  const query = searchParams.toString();
  return query ? `?${query}` : '';
}

/** Generic fetch wrapper with error handling. */
async function fetchApi<T>(url: string, options?: RequestInit): Promise<T> {
  const response = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ message: 'Request failed' }));
    throw new Error(error.message || `HTTP ${response.status}`);
  }

  return response.json();
}

/** Fetch wrapper for endpoints that return no body (e.g. delete → 200 OK). */
async function fetchVoid(url: string, options?: RequestInit): Promise<void> {
  const response = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ message: 'Request failed' }));
    throw new Error(error.message || `HTTP ${response.status}`);
  }
}

function unitBase(buildingId: string, unitId: string): string {
  return `${API_BASE}/${buildingId}/units/${unitId}/person-months`;
}

function buildingBase(buildingId: string): string {
  return `${API_BASE}/${buildingId}/person-months`;
}

// ============================================================================
// Unit-level
// ============================================================================

/** Get person-months for a unit (a whole year, or a single month). */
export async function getUnitPersonMonths(
  buildingId: string,
  unitId: string,
  query: GetUnitPersonMonthsQuery
): Promise<PersonMonth[]> {
  const qs = buildQueryString({ year: query.year, month: query.month });
  return fetchApi<PersonMonth[]>(`${unitBase(buildingId, unitId)}${qs}`);
}

/** Get a single person-month entry by id. */
export async function getPersonMonth(
  buildingId: string,
  unitId: string,
  id: string
): Promise<PersonMonth> {
  return fetchApi<PersonMonth>(`${unitBase(buildingId, unitId)}/${id}`);
}

/** Create or update a unit's person-month entry for a period. */
export async function upsertPersonMonth(
  buildingId: string,
  unitId: string,
  body: UpsertPersonMonthRequest
): Promise<PersonMonth> {
  return fetchApi<PersonMonth>(unitBase(buildingId, unitId), {
    method: 'POST',
    body: JSON.stringify(body),
  });
}

/** Update an existing person-month entry. */
export async function updatePersonMonth(
  buildingId: string,
  unitId: string,
  id: string,
  body: UpdatePersonMonthRequest
): Promise<PersonMonth> {
  return fetchApi<PersonMonth>(`${unitBase(buildingId, unitId)}/${id}`, {
    method: 'PUT',
    body: JSON.stringify(body),
  });
}

/** Delete a person-month entry. */
export async function deletePersonMonth(
  buildingId: string,
  unitId: string,
  id: string
): Promise<void> {
  return fetchVoid(`${unitBase(buildingId, unitId)}/${id}`, { method: 'DELETE' });
}

/** Get the yearly summary (per-month counts + total) for a unit. */
export async function getYearlySummary(
  buildingId: string,
  unitId: string,
  year: number
): Promise<YearlyPersonMonthSummary> {
  const qs = buildQueryString({ year });
  return fetchApi<YearlyPersonMonthSummary>(`${unitBase(buildingId, unitId)}/yearly${qs}`);
}

/**
 * Calculate a suggested person-month count from the unit's resident history for
 * the period (used to pre-populate the entry form). Returns the count.
 */
export async function calculateFromResidents(
  buildingId: string,
  unitId: string,
  body: CalculatePersonMonthRequest
): Promise<number> {
  return fetchApi<number>(`${unitBase(buildingId, unitId)}/calculate`, {
    method: 'POST',
    body: JSON.stringify(body),
  });
}

// ============================================================================
// Building-level
// ============================================================================

/** List per-unit person-months for a building for a given period. */
export async function listBuildingPersonMonths(
  buildingId: string,
  query: BuildingPersonMonthsQuery
): Promise<PersonMonthWithUnit[]> {
  const qs = buildQueryString({ year: query.year, month: query.month });
  return fetchApi<PersonMonthWithUnit[]>(`${buildingBase(buildingId)}${qs}`);
}

/** Bulk create-or-update person-months for many units in one period. */
export async function bulkUpsertPersonMonths(
  buildingId: string,
  body: BulkUpsertPersonMonthsRequest
): Promise<BulkPersonMonthUpsertResult> {
  return fetchApi<BulkPersonMonthUpsertResult>(`${buildingBase(buildingId)}/bulk`, {
    method: 'POST',
    body: JSON.stringify(body),
  });
}

/** Building-level aggregate summary for a period (total persons + unit count). */
export async function getBuildingSummary(
  buildingId: string,
  query: BuildingPersonMonthsQuery
): Promise<PersonMonthBuildingSummary> {
  const qs = buildQueryString({ year: query.year, month: query.month });
  return fetchApi<PersonMonthBuildingSummary>(`${buildingBase(buildingId)}/summary${qs}`);
}
