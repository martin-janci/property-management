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
 * List buildings with optional filters.
 */
export async function listBuildings(
  params?: ListBuildingsParams,
  signal?: AbortSignal
): Promise<BuildingsPaginatedResponse<Building>> {
  const qs = buildQueryString(params || {});
  return apiRequest<BuildingsPaginatedResponse<Building>>(`${API_BASE}${qs}`, { signal });
}

/**
 * Get building by ID.
 */
export async function getBuilding(id: string, signal?: AbortSignal): Promise<Building> {
  return apiRequest<Building>(`${API_BASE}/${id}`, { signal });
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
      return handleResponse(response);
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
      return handleResponse(response);
    },

    /**
     * Get building by ID.
     */
    get: async (id: string): Promise<Building> => {
      const response = await fetch(`${baseUrl}/${id}`, { headers });
      return handleResponse(response);
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
      return handleResponse(response);
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
     * List units in a building.
     *
     * Query keys use the backend's snake_case parameter names
     * (`include_archived`, `unit_type`) so the request matches the
     * `/buildings/{id}/units` handler contract.
     */
    listUnits: async (buildingId: string, params?: ListUnitsParams): Promise<UnitsListResponse> => {
      const searchParams = new URLSearchParams();
      if (params?.offset !== undefined) searchParams.set('offset', params.offset.toString());
      if (params?.limit !== undefined) searchParams.set('limit', params.limit.toString());
      if (params?.includeArchived) searchParams.set('include_archived', 'true');
      if (params?.unitType) searchParams.set('unit_type', params.unitType);
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
     * Archive (soft-delete) a unit.
     */
    archiveUnit: async (buildingId: string, unitId: string): Promise<void> => {
      const response = await fetch(`${baseUrl}/${buildingId}/units/${unitId}`, {
        method: 'DELETE',
        headers,
      });
      if (!response.ok) {
        const error = await response.json().catch(() => ({ message: 'Unknown error' }));
        throw new Error(error.message || `HTTP ${response.status}`);
      }
    },

    /**
     * Restore a previously archived unit.
     */
    restoreUnit: async (buildingId: string, unitId: string): Promise<Unit> => {
      const response = await fetch(`${baseUrl}/${buildingId}/units/${unitId}/restore`, {
        method: 'POST',
        headers,
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
  };
};

export type BuildingsApi = ReturnType<typeof createBuildingsApi>;
