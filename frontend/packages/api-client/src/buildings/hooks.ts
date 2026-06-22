/**
 * Buildings TanStack Query Hooks
 *
 * React hooks for managing buildings with server state caching.
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { type BuildingsApi, getBuilding, listBuildings } from './api';
import type {
  CreateBuildingRequest,
  CreateCommonAreaRequest,
  CreateFloorRequest,
  CreateUnitRequest,
  ListBuildingDocumentsParams,
  ListBuildingsParams,
  ListUnitsParams,
  UpdateBuildingRequest,
  UpdateUnitRequest,
  UploadDocumentRequest,
} from './types';

// ============================================
// Direct Hooks (using token provider)
// ============================================

/**
 * List buildings with optional filters.
 */
export function useBuildings(params?: ListBuildingsParams) {
  return useQuery({
    queryKey: buildingKeys.list(params),
    queryFn: ({ signal }) => listBuildings(params, signal),
    staleTime: 30_000,
  });
}

/**
 * Get building by ID.
 */
export function useBuilding(id: string) {
  return useQuery({
    queryKey: buildingKeys.detail(id),
    queryFn: ({ signal }) => getBuilding(id, signal),
    enabled: !!id,
    staleTime: 60_000,
  });
}

// ============================================
// Query Key Factory
// ============================================

// Query keys factory for cache management
export const buildingKeys = {
  all: ['buildings'] as const,
  lists: () => [...buildingKeys.all, 'list'] as const,
  list: (params?: ListBuildingsParams) => [...buildingKeys.lists(), params] as const,
  details: () => [...buildingKeys.all, 'detail'] as const,
  detail: (id: string) => [...buildingKeys.details(), id] as const,
  floors: (buildingId: string) => [...buildingKeys.detail(buildingId), 'floors'] as const,
  commonAreas: (buildingId: string) =>
    [...buildingKeys.detail(buildingId), 'common-areas'] as const,
  documents: (buildingId: string, params?: ListBuildingDocumentsParams) =>
    [...buildingKeys.detail(buildingId), 'documents', params] as const,
  units: (buildingId: string, params?: ListUnitsParams) =>
    [...buildingKeys.detail(buildingId), 'units', params] as const,
  unit: (buildingId: string, unitId: string) =>
    [...buildingKeys.detail(buildingId), 'units', unitId] as const,
};

/**
 * Creates TanStack Query hooks for buildings API.
 *
 * @param api - Buildings API client instance
 * @returns Object containing all building-related hooks
 */
export const createBuildingHooks = (api: BuildingsApi) => ({
  /**
   * List buildings with optional filters.
   */
  useList: (params?: ListBuildingsParams) =>
    useQuery({
      queryKey: buildingKeys.list(params),
      queryFn: () => api.list(params),
    }),

  /**
   * Get building by ID.
   */
  useGet: (id: string, enabled = true) =>
    useQuery({
      queryKey: buildingKeys.detail(id),
      queryFn: () => api.get(id),
      enabled: enabled && !!id,
    }),

  /**
   * List floors in a building.
   */
  useFloors: (buildingId: string, enabled = true) =>
    useQuery({
      queryKey: buildingKeys.floors(buildingId),
      queryFn: () => api.listFloors(buildingId),
      enabled: enabled && !!buildingId,
    }),

  /**
   * List common areas in a building.
   */
  useCommonAreas: (buildingId: string, enabled = true) =>
    useQuery({
      queryKey: buildingKeys.commonAreas(buildingId),
      queryFn: () => api.listCommonAreas(buildingId),
      enabled: enabled && !!buildingId,
    }),

  /**
   * List units in a building.
   */
  useUnits: (buildingId: string, params?: ListUnitsParams, enabled = true) =>
    useQuery({
      queryKey: buildingKeys.units(buildingId, params),
      queryFn: () => api.listUnits(buildingId, params),
      enabled: enabled && !!buildingId,
    }),

  /**
   * Get a single unit by ID.
   */
  useUnit: (buildingId: string, unitId: string, enabled = true) =>
    useQuery({
      queryKey: buildingKeys.unit(buildingId, unitId),
      queryFn: () => api.getUnit(buildingId, unitId),
      enabled: enabled && !!buildingId && !!unitId,
    }),

  /**
   * List documents for a building.
   */
  useDocuments: (buildingId: string, params?: ListBuildingDocumentsParams, enabled = true) =>
    useQuery({
      queryKey: buildingKeys.documents(buildingId, params),
      queryFn: () => api.listDocuments(buildingId, params),
      enabled: enabled && !!buildingId,
    }),

  /**
   * Create building mutation.
   */
  useCreate: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: (data: CreateBuildingRequest) => api.create(data),
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: buildingKeys.lists() });
      },
    });
  },

  /**
   * Update building mutation.
   */
  useUpdate: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({ id, data }: { id: string; data: UpdateBuildingRequest }) =>
        api.update(id, data),
      onSuccess: (_, { id }) => {
        queryClient.invalidateQueries({ queryKey: buildingKeys.detail(id) });
        queryClient.invalidateQueries({ queryKey: buildingKeys.lists() });
      },
    });
  },

  /**
   * Delete building mutation.
   */
  useDelete: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: (id: string) => api.delete(id),
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: buildingKeys.lists() });
      },
    });
  },

  /**
   * Create floor mutation.
   */
  useCreateFloor: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({ buildingId, data }: { buildingId: string; data: CreateFloorRequest }) =>
        api.createFloor(buildingId, data),
      onSuccess: (_, { buildingId }) => {
        queryClient.invalidateQueries({ queryKey: buildingKeys.floors(buildingId) });
        queryClient.invalidateQueries({ queryKey: buildingKeys.detail(buildingId) });
      },
    });
  },

  /**
   * Create common area mutation.
   */
  useCreateCommonArea: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({ buildingId, data }: { buildingId: string; data: CreateCommonAreaRequest }) =>
        api.createCommonArea(buildingId, data),
      onSuccess: (_, { buildingId }) => {
        queryClient.invalidateQueries({ queryKey: buildingKeys.commonAreas(buildingId) });
      },
    });
  },

  /**
   * Create unit mutation.
   */
  useCreateUnit: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({ buildingId, data }: { buildingId: string; data: CreateUnitRequest }) =>
        api.createUnit(buildingId, data),
      onSuccess: (_, { buildingId }) => {
        queryClient.invalidateQueries({ queryKey: buildingKeys.detail(buildingId) });
      },
    });
  },

  /**
   * Update unit mutation.
   */
  useUpdateUnit: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({
        buildingId,
        unitId,
        data,
      }: {
        buildingId: string;
        unitId: string;
        data: UpdateUnitRequest;
      }) => api.updateUnit(buildingId, unitId, data),
      onSuccess: (_, { buildingId }) => {
        queryClient.invalidateQueries({ queryKey: buildingKeys.detail(buildingId) });
      },
    });
  },

  /**
   * Archive unit mutation (soft-delete).
   */
  useArchiveUnit: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({ buildingId, unitId }: { buildingId: string; unitId: string }) =>
        api.archiveUnit(buildingId, unitId),
      onSuccess: (_, { buildingId }) => {
        queryClient.invalidateQueries({ queryKey: buildingKeys.detail(buildingId) });
      },
    });
  },

  /**
   * Restore unit mutation.
   */
  useRestoreUnit: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({ buildingId, unitId }: { buildingId: string; unitId: string }) =>
        api.restoreUnit(buildingId, unitId),
      onSuccess: (_, { buildingId }) => {
        queryClient.invalidateQueries({ queryKey: buildingKeys.detail(buildingId) });
      },
    });
  },

  /**
   * Upload document mutation.
   */
  useUploadDocument: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({ buildingId, data }: { buildingId: string; data: UploadDocumentRequest }) =>
        api.uploadDocument(buildingId, data),
      onSuccess: (_, { buildingId }) => {
        queryClient.invalidateQueries({ queryKey: buildingKeys.documents(buildingId) });
      },
    });
  },
});

export type BuildingHooks = ReturnType<typeof createBuildingHooks>;
