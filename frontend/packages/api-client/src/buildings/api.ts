/**
 * Buildings API Client
 *
 * API functions for managing buildings (UC-15).
 */

import { getToken } from '../auth';
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

function getAuthHeaders(): HeadersInit {
  const token = getToken();
  return token ? { Authorization: `Bearer ${token}` } : {};
}

async function apiRequest<T>(url: string, options: RequestInit = {}): Promise<T> {
  const response = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...getAuthHeaders(),
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

/** Normalize every building in a paginated list response. */
function normalizeBuildingList(
  response: BuildingsPaginatedResponse<Building>
): BuildingsPaginatedResponse<Building> {
  if (!response || !Array.isArray(response.items)) return response;
  return { ...response, items: response.items.map(normalizeBuilding) };
}

/**
 * List buildings with optional filters.
 */
export async function listBuildings(
  params?: ListBuildingsParams,
  signal?: AbortSignal
): Promise<BuildingsPaginatedResponse<Building>> {
  const qs = buildQueryString(params || {});
  const response = await apiRequest<BuildingsPaginatedResponse<Building>>(`${API_BASE}${qs}`, {
    signal,
  });
  return normalizeBuildingList(response);
}

/**
 * Get building by ID.
 */
export async function getBuilding(id: string, signal?: AbortSignal): Promise<Building> {
  const building = await apiRequest<Building>(`${API_BASE}/${id}`, { signal });
  return normalizeBuilding(building);
}

// ============================================
// Factory-based API Client (legacy pattern)
// ============================================

const buildHeaders = (config: ApiConfig): HeadersInit => ({
  'Content-Type': 'application/json',
  ...(config.accessToken && { Authorization: `Bearer ${config.accessToken}` }),
  ...(config.tenantId && { 'X-Tenant-ID': config.tenantId }),
});

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
      const searchParams = new URLSearchParams();
      if (params?.page) searchParams.set('page', params.page.toString());
      if (params?.pageSize) searchParams.set('pageSize', params.pageSize.toString());
      if (params?.status) searchParams.set('status', params.status);
      if (params?.type) searchParams.set('type', params.type);

      const url = searchParams.toString() ? `${baseUrl}?${searchParams}` : baseUrl;
      const response = await fetch(url, { headers });
      return normalizeBuildingList(
        await handleResponse<BuildingsPaginatedResponse<Building>>(response)
      );
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
      return normalizeBuilding(await handleResponse<Building>(response));
    },

    /**
     * Get building by ID.
     */
    get: async (id: string): Promise<Building> => {
      const response = await fetch(`${baseUrl}/${id}`, { headers });
      return normalizeBuilding(await handleResponse<Building>(response));
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
      return normalizeBuilding(await handleResponse<Building>(response));
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
      const response = await fetch(`${baseUrl}/${buildingId}/floors`, { headers });
      return handleResponse(response);
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
    listCommonAreas: async (buildingId: string): Promise<CommonArea[]> => {
      const response = await fetch(`${baseUrl}/${buildingId}/common-areas`, { headers });
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
