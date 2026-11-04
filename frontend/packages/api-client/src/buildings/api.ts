/**
 * Buildings API Client
 *
 * API functions for managing buildings (UC-15).
 */

import { bearerAuthHeader, getOrg } from '../auth';
import type { ApiConfig } from '../index';
import type {
  Building,
  BuildingDocument,
  BuildingsPaginatedResponse,
  CommonArea,
  CreateBuildingRequest,
  CreateCommonAreaRequest,
  CreateFloorRequest,
  CreateUnitRequest,
  Floor,
  ListBuildingDocumentsParams,
  ListBuildingsParams,
  ListUnitsParams,
  Unit,
  UnitsListResponse,
  UpdateBuildingRequest,
  UpdateUnitRequest,
  UploadDocumentRequest,
} from './types';

// ============================================
// Direct API Functions (using token provider)
// ============================================

// Use configurable base URL from environment, falling back to relative path
const _win = typeof window !== 'undefined' ? (window as unknown as Record<string, unknown>) : {};
const API_BASE = `${_win.__API_BASE_URL__ ? String(_win.__API_BASE_URL__) : ''}/api/v1/buildings`;

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

function buildQueryString(params: object): string {
  const searchParams = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== null) {
      searchParams.append(key, String(value));
    }
  }
  const queryString = searchParams.toString();
  return queryString ? `?${queryString}` : '';
}

/**
 * Coerce an unknown coordinate value to a finite number, or null.
 *
 * Tolerates numeric strings because `rust_decimal::Decimal` serializes as a
 * JSON string by default.
 */
function toCoordinate(value: unknown): number | null {
  if (value == null || value === '') return null;
  const n = Number(value);
  return Number.isFinite(n) ? n : null;
}

/**
 * Normalize a raw building payload so `location` is populated whenever the
 * backend has resolved coordinates.
 *
 * The buildings API serializes geocoded coordinates as top-level `latitude`/
 * `longitude` fields (Story 3.1 AC3), while the client `Building` model exposes
 * them as a nested `location: GeoLocation`. This bridges the two shapes so
 * consumers can rely on `building.location` regardless of transport, and stays
 * a no-op when coordinates are absent/unresolved.
 */
function normalizeBuilding<T extends Partial<Building>>(building: T): T {
  if (!building || typeof building !== 'object' || building.location) return building;
  const raw = building as Record<string, unknown>;
  const latitude = toCoordinate(raw.latitude);
  const longitude = toCoordinate(raw.longitude);
  if (
    latitude !== null &&
    longitude !== null &&
    Math.abs(latitude) <= 90 &&
    Math.abs(longitude) <= 180
  ) {
    return { ...building, location: { latitude, longitude } };
  }
  return building;
}

/**
 * Map an api-server building payload onto the client `Building` shape.
 *
 * `GET /api/v1/buildings` and `GET /api/v1/buildings/{id}` return flat
 * snake_case rows (`street`, `postal_code`, `total_floors`, `unit_count`,
 * `organization_id`, …) while the UI reads `address.*`, `floorCount`,
 * `unitCount`, … Without this mapping every building rendered with an empty
 * address and "undefined" counts. Payloads that already carry `address` are
 * passed through untouched (mocks, future camelCase responses).
 */
export function fromApiBuilding(raw: unknown): Building {
  if (!raw || typeof raw !== 'object') return raw as Building;
  const r = raw as Record<string, unknown>;
  if (r.address && typeof r.address === 'object')
    return normalizeBuilding(r as unknown as Building);
  const str = (v: unknown) => (typeof v === 'string' ? v : undefined);
  const num = (v: unknown) => (typeof v === 'number' && Number.isFinite(v) ? v : undefined);
  const building = {
    id: String(r.id ?? ''),
    organizationId: str(r.organization_id) ?? str(r.organizationId) ?? '',
    name: str(r.name) ?? '',
    address: {
      street: str(r.street) ?? '',
      city: str(r.city) ?? '',
      postalCode: str(r.postal_code) ?? str(r.postalCode) ?? '',
      country: str(r.country) ?? '',
    },
    type: (str(r.type) ?? str(r.building_type) ?? 'residential') as Building['type'],
    floorCount: num(r.total_floors) ?? num(r.floorCount) ?? 0,
    unitCount: num(r.unit_count) ?? num(r.unitCount) ?? 0,
    yearBuilt: num(r.year_built) ?? num(r.yearBuilt),
    totalAreaM2: num(r.total_area_m2) ?? num(r.totalAreaM2),
    photoUrl: str(r.photo_url) ?? str(r.photoUrl),
    description: str(r.description),
    status: (str(r.status) ?? 'active') as Building['status'],
    createdAt: str(r.created_at) ?? str(r.createdAt) ?? '',
    updatedAt: str(r.updated_at) ?? str(r.updatedAt) ?? '',
    latitude: r.latitude,
    longitude: r.longitude,
  };
  const { latitude: _lat, longitude: _lng, ...rest } = normalizeBuilding(building);
  return rest as Building;
}

/**
 * Normalize a list response. api-server returns
 * `{ buildings, total, offset, limit }`; the client contract is
 * `{ items, total, page, pageSize, totalPages }`.
 */
function normalizeBuildingList(response: unknown): BuildingsPaginatedResponse<Building> {
  const r = (response ?? {}) as Record<string, unknown>;
  const rows = Array.isArray(r.items) ? r.items : Array.isArray(r.buildings) ? r.buildings : [];
  const total = typeof r.total === 'number' ? r.total : rows.length;
  const limit =
    typeof r.pageSize === 'number'
      ? r.pageSize
      : typeof r.limit === 'number' && r.limit > 0
        ? r.limit
        : Math.max(rows.length, 1);
  const page =
    typeof r.page === 'number'
      ? r.page
      : typeof r.offset === 'number'
        ? Math.floor(r.offset / limit) + 1
        : 1;
  return {
    items: rows.map(fromApiBuilding),
    total,
    page,
    pageSize: limit,
    totalPages:
      typeof r.totalPages === 'number' ? r.totalPages : Math.max(1, Math.ceil(total / limit)),
  };
}

/**
 * List buildings with optional filters.
 *
 * api-server requires `organization_id` (it must equal the active tenant) and
 * paginates with `offset` / `limit`; translate the client's page-based params.
 */
export async function listBuildings(
  params?: ListBuildingsParams,
  signal?: AbortSignal
): Promise<BuildingsPaginatedResponse<Building>> {
  const { page, pageSize, ...filters } = params ?? {};
  const limit = pageSize ?? 50;
  const qs = buildQueryString({
    organization_id: getOrg() ?? undefined,
    offset: page && page > 1 ? (page - 1) * limit : undefined,
    limit: pageSize,
    ...filters,
  });
  const response = await apiRequest<unknown>(`${API_BASE}${qs}`, { signal });
  return normalizeBuildingList(response);
}

/**
 * Get building by ID.
 */
export async function getBuilding(id: string, signal?: AbortSignal): Promise<Building> {
  const building = await apiRequest<unknown>(`${API_BASE}/${id}`, { signal });
  return fromApiBuilding(building);
}

/**
 * List units in a building (`GET /api/v1/buildings/{id}/units`, paginated).
 * Archived units are excluded unless `params.include_archived` is set.
 */
export async function listBuildingUnits(
  buildingId: string,
  params?: ListUnitsParams,
  signal?: AbortSignal
): Promise<UnitsListResponse> {
  const qs = buildQueryString(params || {});
  return apiRequest<UnitsListResponse>(`${API_BASE}/${buildingId}/units${qs}`, { signal });
}

// ============================================
// Factory-based API Client (legacy pattern)
// ============================================

const buildHeaders = (config: ApiConfig): HeadersInit => ({
  'Content-Type': 'application/json',
  ...(config.accessToken && { Authorization: `Bearer ${config.accessToken}` }),
  ...(config.tenantId && { 'X-Tenant-ID': config.tenantId }),
});

/** Page size for unit pagination (the backend caps `limit` at 100). */
const UNIT_PAGE_SIZE = 100;
/** Safety cap on unit pages fetched when deriving floors (10 000 units). */
const MAX_UNIT_PAGES = 100;

const handleResponse = async <T>(response: Response): Promise<T> => {
  if (!response.ok) {
    const error = await response.json().catch(() => ({ message: 'Unknown error' }));
    throw new Error(error.message || `HTTP ${response.status}`);
  }
  return response.json();
};

/**
 * Creates a buildings API client.
 *
 * @param config - API configuration including base URL and auth token
 * @returns Buildings API methods
 */
export const createBuildingsApi = (config: ApiConfig) => {
  const baseUrl = `${config.baseUrl}/api/v1/buildings`;
  const headers = buildHeaders(config);

  return {
    /**
     * List buildings with optional filters.
     */
    list: async (params?: ListBuildingsParams): Promise<BuildingsPaginatedResponse<Building>> => {
      const { page, pageSize, ...filters } = params ?? {};
      const limit = pageSize ?? 50;
      const qs = buildQueryString({
        organization_id: config.tenantId ?? getOrg() ?? undefined,
        offset: page && page > 1 ? (page - 1) * limit : undefined,
        limit: pageSize,
        ...filters,
      });
      const response = await fetch(`${baseUrl}${qs}`, { headers });
      return normalizeBuildingList(await handleResponse<unknown>(response));
    },

    /**
     * Create a new building.
     */
    create: async (data: CreateBuildingRequest): Promise<Building> => {
      const response = await fetch(baseUrl, {
        method: 'POST',
        headers,
        body: JSON.stringify(data),
      });
      return fromApiBuilding(await handleResponse<unknown>(response));
    },

    /**
     * Get building by ID.
     */
    get: async (id: string): Promise<Building> => {
      const response = await fetch(`${baseUrl}/${id}`, { headers });
      return fromApiBuilding(await handleResponse<unknown>(response));
    },

    /**
     * Update a building.
     */
    update: async (id: string, data: UpdateBuildingRequest): Promise<Building> => {
      const response = await fetch(`${baseUrl}/${id}`, {
        method: 'PATCH',
        headers,
        body: JSON.stringify(data),
      });
      return fromApiBuilding(await handleResponse<unknown>(response));
    },

    /**
     * Delete a building.
     */
    delete: async (id: string): Promise<void> => {
      const response = await fetch(`${baseUrl}/${id}`, {
        method: 'DELETE',
        headers,
      });
      if (!response.ok) {
        const error = await response.json().catch(() => ({ message: 'Unknown error' }));
        throw new Error(error.message || `HTTP ${response.status}`);
      }
    },

    /**
     * List floors in a building.
     */
    listFloors: async (buildingId: string): Promise<Floor[]> => {
      // The backend has no floors table: a floor is the `floor` column of the
      // building's units, so derive the list from every (non-archived) unit.
      const counts = new Map<number, number>();
      let offset = 0;
      for (let page = 0; page < MAX_UNIT_PAGES; page++) {
        const response = await fetch(
          `${baseUrl}/${buildingId}/units?offset=${offset}&limit=${UNIT_PAGE_SIZE}`,
          { headers }
        );
        const body = await handleResponse<UnitsListResponse>(response);
        for (const unit of body.units) {
          counts.set(unit.floor, (counts.get(unit.floor) ?? 0) + 1);
        }
        offset += body.units.length;
        if (body.units.length === 0 || offset >= body.total) break;
      }
      return [...counts.entries()]
        .sort(([a], [b]) => a - b)
        .map(([number, unitCount]) => ({
          id: `${buildingId}:floor:${number}`,
          buildingId,
          number,
          unitCount,
        }));
    },

    /**
     * Create a floor in a building.
     */
    createFloor: async (buildingId: string, data: CreateFloorRequest): Promise<Floor> => {
      const response = await fetch(`${baseUrl}/${buildingId}/floors`, {
        method: 'POST',
        headers,
        body: JSON.stringify(data),
      });
      return handleResponse(response);
    },

    /**
     * List common areas in a building.
     */
    listCommonAreas: async (buildingId: string): Promise<CommonArea[] | null> => {
      const response = await fetch(`${baseUrl}/${buildingId}/common-areas`, { headers });
      // The backend does not implement common areas yet: a 404 means the
      // feature is unavailable (resolve `null`, so the query neither errors
      // nor retries), not that the request failed.
      if (response.status === 404) return null;
      return handleResponse(response);
    },

    /**
     * Create a common area in a building.
     */
    createCommonArea: async (
      buildingId: string,
      data: CreateCommonAreaRequest
    ): Promise<CommonArea> => {
      const response = await fetch(`${baseUrl}/${buildingId}/common-areas`, {
        method: 'POST',
        headers,
        body: JSON.stringify(data),
      });
      return handleResponse(response);
    },

    /**
     * List documents for a building.
     */
    listDocuments: async (
      buildingId: string,
      params?: ListBuildingDocumentsParams
    ): Promise<BuildingsPaginatedResponse<BuildingDocument>> => {
      const searchParams = new URLSearchParams();
      if (params?.page) searchParams.set('page', params.page.toString());
      if (params?.pageSize) searchParams.set('pageSize', params.pageSize.toString());
      if (params?.category) searchParams.set('category', params.category);

      const url = searchParams.toString()
        ? `${baseUrl}/${buildingId}/documents?${searchParams}`
        : `${baseUrl}/${buildingId}/documents`;
      const response = await fetch(url, { headers });
      return handleResponse(response);
    },

    /**
     * Upload a document to a building.
     */
    uploadDocument: async (
      buildingId: string,
      data: UploadDocumentRequest
    ): Promise<BuildingDocument> => {
      const response = await fetch(`${baseUrl}/${buildingId}/documents`, {
        method: 'POST',
        headers,
        body: JSON.stringify(data),
      });
      return handleResponse(response);
    },

    /**
     * List units in a building (paginated). Archived units are excluded unless
     * `params.include_archived` is set.
     */
    listUnits: async (buildingId: string, params?: ListUnitsParams): Promise<UnitsListResponse> => {
      const searchParams = new URLSearchParams();
      if (params?.offset !== undefined) searchParams.set('offset', params.offset.toString());
      if (params?.limit !== undefined) searchParams.set('limit', params.limit.toString());
      if (params?.include_archived) searchParams.set('include_archived', 'true');
      if (params?.unit_type) searchParams.set('unit_type', params.unit_type);
      if (params?.floor !== undefined) searchParams.set('floor', params.floor.toString());

      const url = searchParams.toString()
        ? `${baseUrl}/${buildingId}/units?${searchParams}`
        : `${baseUrl}/${buildingId}/units`;
      const response = await fetch(url, { headers });
      return handleResponse(response);
    },

    /**
     * Get a single unit by ID.
     */
    getUnit: async (buildingId: string, unitId: string): Promise<Unit> => {
      const response = await fetch(`${baseUrl}/${buildingId}/units/${unitId}`, { headers });
      return handleResponse(response);
    },

    /**
     * Create a unit in a building.
     */
    createUnit: async (buildingId: string, data: CreateUnitRequest): Promise<Unit> => {
      const response = await fetch(`${baseUrl}/${buildingId}/units`, {
        method: 'POST',
        headers,
        body: JSON.stringify(data),
      });
      return handleResponse(response);
    },

    /**
     * Update a unit.
     */
    updateUnit: async (
      buildingId: string,
      unitId: string,
      data: UpdateUnitRequest
    ): Promise<Unit> => {
      const response = await fetch(`${baseUrl}/${buildingId}/units/${unitId}`, {
        method: 'PUT',
        headers,
        body: JSON.stringify(data),
      });
      return handleResponse(response);
    },

    /**
     * Archive (soft-delete) a unit. Returns the archived unit.
     */
    archiveUnit: async (buildingId: string, unitId: string): Promise<Unit> => {
      const response = await fetch(`${baseUrl}/${buildingId}/units/${unitId}`, {
        method: 'DELETE',
        headers,
      });
      return handleResponse(response);
    },

    /**
     * Restore a previously archived unit. Returns the restored unit.
     */
    restoreUnit: async (buildingId: string, unitId: string): Promise<Unit> => {
      const response = await fetch(`${baseUrl}/${buildingId}/units/${unitId}/restore`, {
        method: 'POST',
        headers,
      });
      return handleResponse(response);
    },
  };
};

export type BuildingsApi = ReturnType<typeof createBuildingsApi>;
