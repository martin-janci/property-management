/**
 * Meters API client functions (Epic 12: self-readings & consumption).
 *
 * Thin typed `fetch` wrappers over `/api/v1/meters/*`. Mirrors the structure of
 * the faults module (its own `fetchApi`/`buildQueryString` helpers) so the two
 * stay consistent.
 */

import type {
  ConsumptionHistoryResponse,
  ConsumptionQuery,
  ListMeterReadingsResponse,
  ListMetersQuery,
  ListMetersResponse,
  ListReadingsQuery,
  Meter,
  MeterReading,
  MeterResponse,
  PendingReadingsQuery,
  SubmitReadingRequest,
  ValidateReadingRequest,
} from './types';

const API_BASE = '/api/v1/meters';

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

// ============================================================================
// Meters
// ============================================================================

/** List meters for a building. */
export async function listMetersForBuilding(
  buildingId: string,
  query: ListMetersQuery = {}
): Promise<ListMetersResponse> {
  const qs = buildQueryString({ limit: query.limit, offset: query.offset });
  return fetchApi<ListMetersResponse>(`${API_BASE}/buildings/${buildingId}${qs}`);
}

/** List meters for a unit. */
export async function listMetersForUnit(unitId: string): Promise<Meter[]> {
  return fetchApi<Meter[]>(`${API_BASE}/units/${unitId}`);
}

/** Get a single meter with its recent readings. */
export async function getMeter(id: string): Promise<MeterResponse> {
  return fetchApi<MeterResponse>(`${API_BASE}/${id}`);
}

// ============================================================================
// Readings
// ============================================================================

/** Submit a meter reading. */
export async function submitReading(body: SubmitReadingRequest): Promise<MeterReading> {
  return fetchApi<MeterReading>(`${API_BASE}/readings`, {
    method: 'POST',
    body: JSON.stringify(body),
  });
}

/** Get a single reading by ID. */
export async function getReading(id: string): Promise<MeterReading> {
  return fetchApi<MeterReading>(`${API_BASE}/readings/${id}`);
}

/** List readings for a meter. */
export async function listReadings(
  meterId: string,
  query: ListReadingsQuery = {}
): Promise<ListMeterReadingsResponse> {
  const qs = buildQueryString({
    from: query.from,
    to: query.to,
    limit: query.limit,
    offset: query.offset,
  });
  return fetchApi<ListMeterReadingsResponse>(`${API_BASE}/${meterId}/readings${qs}`);
}

/** Manager queue: readings awaiting validation. Requires an organization scope. */
export async function getPendingReadings(query: PendingReadingsQuery): Promise<MeterReading[]> {
  const qs = buildQueryString({
    organization_id: query.organization_id,
    building_id: query.building_id,
  });
  return fetchApi<MeterReading[]>(`${API_BASE}/readings/pending${qs}`);
}

/** Validate (approve / reject / correct) a reading. */
export async function validateReading(
  id: string,
  body: ValidateReadingRequest
): Promise<MeterReading> {
  return fetchApi<MeterReading>(`${API_BASE}/readings/${id}/validate`, {
    method: 'PUT',
    body: JSON.stringify(body),
  });
}

// ============================================================================
// Consumption
// ============================================================================

/** Consumption history for a meter over a date range. */
export async function getConsumptionHistory(
  meterId: string,
  query: ConsumptionQuery
): Promise<ConsumptionHistoryResponse> {
  const qs = buildQueryString({ from: query.from, to: query.to });
  return fetchApi<ConsumptionHistoryResponse>(`${API_BASE}/${meterId}/consumption${qs}`);
}
