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
  type ListUnitsParams,
} from '@ppt/api-client';
import { useMemo } from 'react';

// API base URL from environment
const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? '';

/**
 * Creates a buildings API client with current auth token.
 *
 * `createBuildingsApi` bakes the `Authorization: Bearer <token>` header into
 * the returned client at construction time, so the client must be re-created
 * whenever the token rotates — see {@link useBuildingsApi}.
 */
function getBuildingsApi(accessToken: string | undefined) {
  return createBuildingsApi({
    baseUrl: API_BASE_URL,
    accessToken,
  });
}

/**
 * Hook to get buildings API hooks with current auth context.
 *
 * Creates a memoized API client and returns TanStack Query hooks for buildings
 * operations. The memo is keyed on the current access token so a token refresh
 * (rotation) rebuilds the client with the fresh Bearer header instead of
 * sending a stale token for the rest of the component's lifetime.
 */
export function useBuildingsApi() {
  const token = getToken() ?? undefined;
  const api = useMemo(() => getBuildingsApi(token), [token]);
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
 * Hook to list units in a building (Story 3.2).
 */
export function useUnits(buildingId: string, params?: ListUnitsParams, enabled = true) {
  const hooks = useBuildingsApi();
  return hooks.useUnits(buildingId, params, enabled);
}

/**
 * Hook to get a single unit by ID (Story 3.2).
 */
export function useUnit(buildingId: string, unitId: string, enabled = true) {
  const hooks = useBuildingsApi();
  return hooks.useUnit(buildingId, unitId, enabled);
}

/**
 * Hook for unit mutations (create, update, archive, restore) — Story 3.2.
 */
export function useUnitMutations() {
  const hooks = useBuildingsApi();
  return {
    useCreate: hooks.useCreateUnit,
    useUpdate: hooks.useUpdateUnit,
    useArchive: hooks.useArchiveUnit,
    useRestore: hooks.useRestoreUnit,
  };
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
