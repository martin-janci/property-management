/**
 * Buildings Hooks
 *
 * React hooks for buildings data using @ppt/api-client.
 *
 * @see Story 81.2 - Wire buildings page to API
 */

import {
  createBuildingHooks,
  createBuildingsApi,
  getToken,
  type ListBuildingsParams,
} from '@ppt/api-client';
import { useMemo } from 'react';

// API base URL from environment
const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? '';

/**
 * Creates a buildings API client with current auth token.
 */
function getBuildingsApi() {
  return createBuildingsApi({
    baseUrl: API_BASE_URL,
    accessToken: getToken() ?? undefined,
  });
}

/**
 * Hook to get buildings API hooks with current auth context.
 *
 * Creates memoized API client and returns TanStack Query hooks
 * for buildings operations.
 */
export function useBuildingsApi() {
  const api = useMemo(() => getBuildingsApi(), []);
  return useMemo(() => createBuildingHooks(api), [api]);
}

/**
 * Hook to list buildings with optional filters.
 */
export function useBuildings(params?: ListBuildingsParams) {
  const hooks = useBuildingsApi();
  return hooks.useList(params);
}

/**
 * Hook to get a single building by ID.
 */
export function useBuilding(id: string, enabled = true) {
  const hooks = useBuildingsApi();
  return hooks.useGet(id, enabled);
}

/**
 * Hook to get building floors.
 */
export function useBuildingFloors(buildingId: string, enabled = true) {
  const hooks = useBuildingsApi();
  return hooks.useFloors(buildingId, enabled);
}

/**
 * Hook to get building common areas.
 */
export function useBuildingCommonAreas(buildingId: string, enabled = true) {
  const hooks = useBuildingsApi();
  return hooks.useCommonAreas(buildingId, enabled);
}

/**
 * Hook for building mutations (create, update, delete).
 */
export function useBuildingMutations() {
  const hooks = useBuildingsApi();
  return {
    useCreate: hooks.useCreate,
    useUpdate: hooks.useUpdate,
    useDelete: hooks.useDelete,
    useCreateFloor: hooks.useCreateFloor,
    useCreateCommonArea: hooks.useCreateCommonArea,
    useUploadDocument: hooks.useUploadDocument,
  };
}
