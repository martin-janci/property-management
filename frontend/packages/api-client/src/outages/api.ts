/**
 * Outages API Client
 *
 * API functions for Outages (UC-12).
 */

import { bearerAuthHeader } from '../auth';
import type {
  CancelOutageRequest,
  CreateOutageRequest,
  CreateOutageResponse,
  OutageActionResponse,
  OutageDashboard,
  OutageListQuery,
  OutageStatistics,
  OutageWithDetails,
  PaginatedOutages,
  ResolveOutageRequest,
  StartOutageRequest,
  UnreadOutagesCount,
  UpdateOutageRequest,
} from './types';

const API_BASE = '/api/v1/outages';

// Helper function to build query string
function buildQueryString(params: object): string {
  const searchParams = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== null) {
      if (Array.isArray(value)) {
        for (const v of value) {
          searchParams.append(key, String(v));
        }
      } else {
        searchParams.append(key, String(value));
      }
    }
  }
  const queryString = searchParams.toString();
  return queryString ? `?${queryString}` : '';
}

// Helper for API requests
async function apiRequest<T>(url: string, options: RequestInit = {}): Promise<T> {
  const response = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...bearerAuthHeader(),
      ...options.headers,
    },
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({}));
    throw new Error(error.message || `HTTP error ${response.status}`);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return response.json();
}

// ============================================
// Request body mapping
// ============================================

function withoutUndefined(body: Record<string, unknown>): Record<string, unknown> {
  for (const key of Object.keys(body)) {
    if (body[key] === undefined) delete body[key];
  }
  return body;
}

/**
 * The server parses timestamps as RFC 3339 (`DateTime<Utc>`). A
 * `datetime-local` input yields `YYYY-MM-DDTHH:mm` with no offset, which the
 * server rejects, so such values are read as local time and sent as UTC.
 */
export function toOutageTimestamp(value: string | undefined): string | undefined {
  if (!value) return undefined;
  if (/(Z|[+-]\d{2}:?\d{2})$/i.test(value)) return value;
  const parsed = new Date(value);
  return Number.isNaN(parsed.getTime()) ? value : parsed.toISOString();
}

/**
 * The api-server outage request structs are snake_case with no serde rename;
 * the camelCase client shape made create fail with 422
 * `missing field scheduled_start`.
 */
export function toOutageWriteBody(
  data: CreateOutageRequest | UpdateOutageRequest
): Record<string, unknown> {
  return withoutUndefined({
    title: data.title,
    description: data.description,
    commodity: data.commodity,
    severity: data.severity,
    building_ids: data.buildingIds,
    scheduled_start: toOutageTimestamp(data.scheduledStart),
    scheduled_end: toOutageTimestamp(data.scheduledEnd),
    external_reference: data.externalReference,
    supplier_name: data.supplierName,
  });
}

/** Map start/resolve/cancel requests to the server's snake_case bodies. */
export function toOutageActionBody(
  data?: StartOutageRequest | ResolveOutageRequest | CancelOutageRequest
): Record<string, unknown> {
  const d = (data ?? {}) as StartOutageRequest & ResolveOutageRequest & CancelOutageRequest;
  return withoutUndefined({
    actual_start: toOutageTimestamp(d.actualStart),
    actual_end: toOutageTimestamp(d.actualEnd),
    resolution_notes: d.resolutionNotes,
    reason: d.reason,
  });
}

// ============================================
// List & Detail Queries
// ============================================

export async function listOutages(query?: OutageListQuery): Promise<PaginatedOutages> {
  const q = query ?? {};
  // The server's `ListOutagesQuery` is snake_case.
  const qs = buildQueryString({
    status: q.status,
    commodity: q.commodity,
    severity: q.severity,
    building_id: q.buildingId,
    from_date: q.fromDate,
    to_date: q.toDate,
    active_only: q.activeOnly,
    limit: q.limit,
    offset: q.offset,
  });
  return apiRequest<PaginatedOutages>(`${API_BASE}${qs}`);
}

export async function listActiveOutages(): Promise<PaginatedOutages> {
  return apiRequest<PaginatedOutages>(`${API_BASE}/active`);
}

export async function getOutage(id: string): Promise<OutageWithDetails> {
  const response = await apiRequest<{ outage: OutageWithDetails }>(`${API_BASE}/${id}`);
  return response.outage;
}

export async function getOutageStatistics(): Promise<OutageStatistics> {
  const response = await apiRequest<{ statistics: OutageStatistics }>(`${API_BASE}/statistics`);
  return response.statistics;
}

export async function getOutageDashboard(): Promise<OutageDashboard> {
  const response = await apiRequest<{ dashboard: OutageDashboard }>(`${API_BASE}/dashboard`);
  return response.dashboard;
}

export async function getUnreadCount(): Promise<UnreadOutagesCount> {
  return apiRequest<UnreadOutagesCount>(`${API_BASE}/unread-count`);
}

// ============================================
// Create & Update
// ============================================

export async function createOutage(data: CreateOutageRequest): Promise<CreateOutageResponse> {
  return apiRequest<CreateOutageResponse>(`${API_BASE}`, {
    method: 'POST',
    body: JSON.stringify(toOutageWriteBody(data)),
  });
}

export async function updateOutage(
  id: string,
  data: UpdateOutageRequest
): Promise<OutageActionResponse> {
  return apiRequest<OutageActionResponse>(`${API_BASE}/${id}`, {
    method: 'PUT',
    body: JSON.stringify(toOutageWriteBody(data)),
  });
}

export async function deleteOutage(id: string): Promise<void> {
  return apiRequest<void>(`${API_BASE}/${id}`, {
    method: 'DELETE',
  });
}

// ============================================
// Status Changes
// ============================================

export async function startOutage(
  id: string,
  data?: StartOutageRequest
): Promise<OutageActionResponse> {
  return apiRequest<OutageActionResponse>(`${API_BASE}/${id}/start`, {
    method: 'POST',
    body: JSON.stringify(toOutageActionBody(data)),
  });
}

export async function resolveOutage(
  id: string,
  data?: ResolveOutageRequest
): Promise<OutageActionResponse> {
  return apiRequest<OutageActionResponse>(`${API_BASE}/${id}/resolve`, {
    method: 'POST',
    body: JSON.stringify(toOutageActionBody(data)),
  });
}

export async function cancelOutage(
  id: string,
  data?: CancelOutageRequest
): Promise<OutageActionResponse> {
  return apiRequest<OutageActionResponse>(`${API_BASE}/${id}/cancel`, {
    method: 'POST',
    body: JSON.stringify(toOutageActionBody(data)),
  });
}

// ============================================
// Read Tracking
// ============================================

export async function markOutageRead(id: string): Promise<void> {
  return apiRequest<void>(`${API_BASE}/${id}/read`, {
    method: 'POST',
  });
}
