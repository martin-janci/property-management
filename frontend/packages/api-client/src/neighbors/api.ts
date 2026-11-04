/**
 * Neighbor API Client
 *
 * API functions for neighbor information (Epic 6, Story 6.6).
 */

import { getOrg, getToken } from '../auth';
import type { ApiConfig } from '../index';
import type {
  NeighborsResponse,
  NeighborView,
  PrivacySettingsResponse,
  UpdatePrivacySettingsRequest,
} from './types';

/**
 * Build request headers at request time. The token comes from the registered
 * token provider first, so a client created before sign-in finished (or
 * before a refresh) never sends a missing or stale bearer token; the
 * construction-time `config` values are only the fallback.
 */
const buildHeaders = (config: ApiConfig): Record<string, string> => {
  const token = getToken() ?? config.accessToken;
  const tenantId = config.tenantId ?? getOrg() ?? undefined;
  return {
    'Content-Type': 'application/json',
    ...(token && { Authorization: `Bearer ${token}` }),
    ...(tenantId && { 'X-Tenant-ID': tenantId }),
  };
};

const handleResponse = async <T>(response: Response): Promise<T> => {
  if (!response.ok) {
    const error = await response.json().catch(() => ({ message: 'Unknown error' }));
    throw new Error(error.message || `HTTP ${response.status}`);
  }
  return response.json();
};

type RawRecord = Record<string, unknown>;

function field<T>(raw: RawRecord, snake: string, camel: string): T | undefined {
  const value = raw[snake] ?? raw[camel];
  return value === null ? undefined : (value as T | undefined);
}

/**
 * The api-server serializes `NeighborView` in snake_case (`user_id`,
 * `display_name`, `is_visible`, …) while the client type is camelCase. Without
 * this mapping every neighbor looked hidden and nameless, so the messaging
 * recipient picker had nobody to offer.
 */
export function normalizeNeighbor(raw: RawRecord): NeighborView {
  return {
    userId: field<string>(raw, 'user_id', 'userId') ?? '',
    displayName: field<string>(raw, 'display_name', 'displayName') ?? '',
    unitLabel: field<string>(raw, 'unit_label', 'unitLabel') ?? '',
    isVisible: Boolean(field<boolean>(raw, 'is_visible', 'isVisible')),
    email: field<string>(raw, 'email', 'email') ?? null,
    phone: field<string>(raw, 'phone', 'phone') ?? null,
    residentType: field<string>(raw, 'resident_type', 'residentType') ?? '',
  };
}

export function normalizeNeighborsResponse(raw: unknown): NeighborsResponse {
  const obj = (raw ?? {}) as { neighbors?: unknown; count?: unknown; total?: unknown };
  const neighbors = Array.isArray(obj.neighbors)
    ? (obj.neighbors as RawRecord[]).map(normalizeNeighbor)
    : [];
  return {
    neighbors,
    count: typeof obj.count === 'number' ? obj.count : neighbors.length,
    total: typeof obj.total === 'number' ? obj.total : neighbors.length,
  };
}

export const createNeighborsApi = (config: ApiConfig) => {
  const baseUrl = config.baseUrl;

  return {
    // ========================================================================
    // Neighbor Operations
    // ========================================================================

    /**
     * List neighbors in the same building
     */
    listNeighbors: async (buildingId: string): Promise<NeighborsResponse> => {
      const response = await fetch(`${baseUrl}/api/v1/buildings/${buildingId}/neighbors`, {
        headers: buildHeaders(config),
      });
      return normalizeNeighborsResponse(await handleResponse<unknown>(response));
    },

    // ========================================================================
    // Privacy Settings
    // ========================================================================

    /**
     * Get current user's privacy settings
     */
    getPrivacySettings: async (): Promise<PrivacySettingsResponse> => {
      const response = await fetch(`${baseUrl}/api/v1/users/me/privacy`, {
        headers: buildHeaders(config),
      });
      return handleResponse(response);
    },

    /**
     * Update current user's privacy settings
     */
    updatePrivacySettings: async (
      data: UpdatePrivacySettingsRequest
    ): Promise<PrivacySettingsResponse> => {
      const response = await fetch(`${baseUrl}/api/v1/users/me/privacy`, {
        method: 'PUT',
        headers: buildHeaders(config),
        body: JSON.stringify(data),
      });
      return handleResponse(response);
    },
  };
};

export type NeighborsApi = ReturnType<typeof createNeighborsApi>;
