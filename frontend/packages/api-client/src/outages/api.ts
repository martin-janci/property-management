/**
 * Outages API Client
 *
 * API functions for Outages (UC-12).
 */

import { bearerAuthHeader } from '../auth';
import { authenticatedFetch } from '../lib/fetch';
import type {
  CancelOutageRequest,
  CreateOutageRequest,
  CreateOutageResponse,
  Outage,
  OutageActionResponse,
  OutageBuilding,
  OutageCommodity,
  OutageDashboard,
  OutageListQuery,
  OutageSeverity,
  OutageStatistics,
  OutageStatus,
  OutageSummary,
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
  const response = await authenticatedFetch(url, {
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
// Response mapping
// ============================================

/**
 * The api-server outage models (`db::models::outage`) serialize snake_case
 * (only `creatorName` is renamed), while the client types are camelCase. The
 * mappers below read either spelling so list and detail views get their
 * fields; without them `scheduledStart`, `buildingIds`, etc. were undefined.
 */
type RawRecord = Record<string, unknown>;

function asRecord(value: unknown): RawRecord {
  return value && typeof value === 'object' ? (value as RawRecord) : {};
}

function pickOutageField<T>(raw: RawRecord, camel: string, snake: string): T | undefined {
  const value = raw[camel] ?? raw[snake];
  return value === null || value === undefined ? undefined : (value as T);
}

function pickOutageNumber(raw: RawRecord, ...keys: string[]): number {
  for (const key of keys) {
    const value = raw[key];
    if (typeof value === 'number') return value;
    if (typeof value === 'string' && value.trim() !== '' && !Number.isNaN(Number(value))) {
      return Number(value);
    }
  }
  return 0;
}

/** `building_ids` is a JSONB value on the server: usually an array, possibly a JSON string. */
function toOutageBuildingIds(value: unknown): string[] {
  let parsed = value;
  if (typeof parsed === 'string') {
    try {
      parsed = JSON.parse(parsed);
    } catch {
      return [];
    }
  }
  return Array.isArray(parsed) ? parsed.map(String) : [];
}

export function normalizeOutage(input: unknown): Outage {
  const raw = asRecord(input);
  return {
    id: String(raw.id ?? ''),
    organizationId: pickOutageField<string>(raw, 'organizationId', 'organization_id') ?? '',
    title: String(raw.title ?? ''),
    description: pickOutageField<string>(raw, 'description', 'description'),
    commodity: raw.commodity as OutageCommodity,
    severity: raw.severity as OutageSeverity,
    status: raw.status as OutageStatus,
    buildingIds: toOutageBuildingIds(raw.buildingIds ?? raw.building_ids),
    scheduledStart: pickOutageField<string>(raw, 'scheduledStart', 'scheduled_start') ?? '',
    scheduledEnd: pickOutageField<string>(raw, 'scheduledEnd', 'scheduled_end'),
    actualStart: pickOutageField<string>(raw, 'actualStart', 'actual_start'),
    actualEnd: pickOutageField<string>(raw, 'actualEnd', 'actual_end'),
    externalReference: pickOutageField<string>(raw, 'externalReference', 'external_reference'),
    supplierName: pickOutageField<string>(raw, 'supplierName', 'supplier_name'),
    resolutionNotes: pickOutageField<string>(raw, 'resolutionNotes', 'resolution_notes'),
    cancelReason: pickOutageField<string>(raw, 'cancelReason', 'cancel_reason'),
    createdBy: pickOutageField<string>(raw, 'createdBy', 'created_by') ?? '',
    createdAt: pickOutageField<string>(raw, 'createdAt', 'created_at') ?? '',
    updatedAt: pickOutageField<string>(raw, 'updatedAt', 'updated_at') ?? '',
  };
}

export function normalizeOutageSummary(input: unknown): OutageSummary {
  const raw = asRecord(input);
  const buildingIds = raw.buildingIds ?? raw.building_ids;
  return {
    id: String(raw.id ?? ''),
    title: String(raw.title ?? ''),
    commodity: raw.commodity as OutageCommodity,
    severity: raw.severity as OutageSeverity,
    status: raw.status as OutageStatus,
    scheduledStart: pickOutageField<string>(raw, 'scheduledStart', 'scheduled_start') ?? '',
    scheduledEnd: pickOutageField<string>(raw, 'scheduledEnd', 'scheduled_end'),
    actualStart: pickOutageField<string>(raw, 'actualStart', 'actual_start'),
    actualEnd: pickOutageField<string>(raw, 'actualEnd', 'actual_end'),
    affectedBuildingsCount:
      raw.affectedBuildingsCount !== undefined || raw.affected_buildings_count !== undefined
        ? pickOutageNumber(raw, 'affectedBuildingsCount', 'affected_buildings_count')
        : toOutageBuildingIds(buildingIds).length,
    createdAt: pickOutageField<string>(raw, 'createdAt', 'created_at') ?? '',
  };
}

export function normalizeOutageDetails(input: unknown): OutageWithDetails {
  const raw = asRecord(input);
  const outage = normalizeOutage(raw);
  let buildings: OutageBuilding[];
  if (Array.isArray(raw.buildings)) {
    buildings = raw.buildings.map((b) => {
      const rb = asRecord(b);
      return {
        id: String(rb.id ?? ''),
        name: String(rb.name ?? ''),
        address: String(rb.address ?? ''),
      };
    });
  } else {
    const names = raw.building_names ?? raw.buildingNames;
    buildings = (Array.isArray(names) ? names : []).map((name, i) => ({
      id: outage.buildingIds[i] ?? '',
      name: String(name),
      address: '',
    }));
  }
  const readStatus = raw.readStatus ?? raw.read_status;
  return {
    ...outage,
    buildings,
    creatorName: pickOutageField<string>(raw, 'creatorName', 'creator_name'),
    ...(readStatus ? { readStatus: readStatus as OutageWithDetails['readStatus'] } : {}),
  };
}

export function normalizeOutageList(input: unknown): PaginatedOutages {
  const raw = asRecord(input);
  const outages = (Array.isArray(raw.outages) ? raw.outages : []).map(normalizeOutageSummary);
  return {
    outages,
    count: pickOutageNumber(raw, 'count') || outages.length,
    total: pickOutageNumber(raw, 'total') || outages.length,
  };
}

export function normalizeOutageStatistics(input: unknown): OutageStatistics {
  const raw = asRecord(input);
  return {
    totalOutages: pickOutageNumber(raw, 'totalOutages', 'total'),
    activeOutages: pickOutageNumber(raw, 'activeOutages', 'ongoing'),
    plannedOutages: pickOutageNumber(raw, 'plannedOutages', 'planned'),
    resolvedOutages: pickOutageNumber(raw, 'resolvedOutages', 'resolved'),
    cancelledOutages: pickOutageNumber(raw, 'cancelledOutages', 'cancelled'),
    outagesByCommodity: asRecord(raw.outagesByCommodity) as OutageStatistics['outagesByCommodity'],
    outagesBySeverity: asRecord(raw.outagesBySeverity) as OutageStatistics['outagesBySeverity'],
    averageResolutionHours: pickOutageField<number>(
      raw,
      'averageResolutionHours',
      'average_resolution_hours'
    ),
  };
}

export function normalizeOutageDashboard(input: unknown): OutageDashboard {
  const raw = asRecord(input);
  const list = (value: unknown) => (Array.isArray(value) ? value.map(normalizeOutageSummary) : []);
  const byCommodity: Record<string, number> = {};
  const rawByCommodity = raw.by_commodity ?? raw.outagesByCommodity;
  if (Array.isArray(rawByCommodity)) {
    for (const entry of rawByCommodity) {
      const e = asRecord(entry);
      byCommodity[String(e.commodity)] = pickOutageNumber(e, 'count');
    }
  } else {
    Object.assign(byCommodity, asRecord(rawByCommodity));
  }
  const ongoing = pickOutageNumber(raw, 'ongoing_count', 'ongoingCount');
  const planned = pickOutageNumber(raw, 'planned_count', 'plannedCount');
  const statistics = raw.statistics
    ? normalizeOutageStatistics(raw.statistics)
    : {
        totalOutages: pickOutageNumber(raw, 'active_count', 'activeCount') || ongoing + planned,
        activeOutages: ongoing,
        plannedOutages: planned,
        resolvedOutages: 0,
        cancelledOutages: 0,
        outagesByCommodity: byCommodity as OutageStatistics['outagesByCommodity'],
        outagesBySeverity: {} as OutageStatistics['outagesBySeverity'],
      };
  return {
    currentOutages: list(raw.currentOutages ?? raw.current_outages),
    upcomingOutages: list(raw.upcomingOutages ?? raw.upcoming),
    recentlyResolved: list(raw.recentlyResolved ?? raw.recently_resolved),
    statistics,
  };
}

function normalizeOutageActionResponse(input: unknown): OutageActionResponse {
  const raw = asRecord(input);
  return { message: String(raw.message ?? ''), outage: normalizeOutage(raw.outage) };
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
  return normalizeOutageList(await apiRequest<unknown>(`${API_BASE}${qs}`));
}

export async function listActiveOutages(): Promise<PaginatedOutages> {
  return normalizeOutageList(await apiRequest<unknown>(`${API_BASE}/active`));
}

export async function getOutage(id: string): Promise<OutageWithDetails> {
  const response = await apiRequest<{ outage: unknown }>(`${API_BASE}/${id}`);
  return normalizeOutageDetails(response.outage);
}

export async function getOutageStatistics(): Promise<OutageStatistics> {
  const response = await apiRequest<{ statistics: unknown }>(`${API_BASE}/statistics`);
  return normalizeOutageStatistics(response.statistics);
}

export async function getOutageDashboard(): Promise<OutageDashboard> {
  const response = await apiRequest<{ dashboard: unknown }>(`${API_BASE}/dashboard`);
  return normalizeOutageDashboard(response.dashboard);
}

export async function getUnreadCount(): Promise<UnreadOutagesCount> {
  const raw = asRecord(await apiRequest<unknown>(`${API_BASE}/unread-count`));
  return { unreadCount: pickOutageNumber(raw, 'unreadCount', 'unread_count') };
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
  const response = await apiRequest<unknown>(`${API_BASE}/${id}`, {
    method: 'PUT',
    body: JSON.stringify(toOutageWriteBody(data)),
  });
  return normalizeOutageActionResponse(response);
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
  const response = await apiRequest<unknown>(`${API_BASE}/${id}/start`, {
    method: 'POST',
    body: JSON.stringify(toOutageActionBody(data)),
  });
  return normalizeOutageActionResponse(response);
}

export async function resolveOutage(
  id: string,
  data?: ResolveOutageRequest
): Promise<OutageActionResponse> {
  const response = await apiRequest<unknown>(`${API_BASE}/${id}/resolve`, {
    method: 'POST',
    body: JSON.stringify(toOutageActionBody(data)),
  });
  return normalizeOutageActionResponse(response);
}

export async function cancelOutage(
  id: string,
  data?: CancelOutageRequest
): Promise<OutageActionResponse> {
  const response = await apiRequest<unknown>(`${API_BASE}/${id}/cancel`, {
    method: 'POST',
    body: JSON.stringify(toOutageActionBody(data)),
  });
  return normalizeOutageActionResponse(response);
}

// ============================================
// Read Tracking
// ============================================

export async function markOutageRead(id: string): Promise<void> {
  return apiRequest<void>(`${API_BASE}/${id}/read`, {
    method: 'POST',
  });
}
