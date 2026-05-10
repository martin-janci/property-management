/**
 * Building Registries TanStack Query Hooks
 *
 * React hooks for managing building registries with server state caching (Epic 57).
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import type { RegistryApi } from './api';
import type {
  CreateParkingSpotRequest,
  CreatePetRegistrationRequest,
  CreateVehicleRegistrationRequest,
  ListParkingSpotsParams,
  ListRegistrationsParams,
  ReviewRegistrationRequest,
  UpdatePetRegistrationRequest,
  UpdateRegistryRulesRequest,
  UpdateVehicleRegistrationRequest,
} from './types';

// Query keys factory for cache management
export const registryKeys = {
  all: ['registries'] as const,
  pets: () => [...registryKeys.all, 'pets'] as const,
  petsList: (params?: ListRegistrationsParams) => [...registryKeys.pets(), 'list', params] as const,
  petDetail: (id: string) => [...registryKeys.pets(), 'detail', id] as const,
  myPets: () => [...registryKeys.pets(), 'my'] as const,
  vehicles: () => [...registryKeys.all, 'vehicles'] as const,
  vehiclesList: (params?: ListRegistrationsParams) =>
    [...registryKeys.vehicles(), 'list', params] as const,
  vehicleDetail: (id: string) => [...registryKeys.vehicles(), 'detail', id] as const,
  myVehicles: () => [...registryKeys.vehicles(), 'my'] as const,
  parkingSpots: (buildingId: string) => [...registryKeys.all, 'parking', buildingId] as const,
  parkingSpotsList: (buildingId: string, params?: ListParkingSpotsParams) =>
    [...registryKeys.parkingSpots(buildingId), params] as const,
  rules: (buildingId: string) => [...registryKeys.all, 'rules', buildingId] as const,
};

export const createRegistryHooks = (api: RegistryApi) => ({
  // ========================================================================
  // Pet Registration Queries
  // ========================================================================

  /**
   * List all pet registrations (managers)
   */
  usePetsList: (params?: ListRegistrationsParams, enabled = true) =>
    useQuery({
      queryKey: registryKeys.petsList(params),
      queryFn: () => api.pets.list(params),
      enabled,
    }),

  /**
   * Get pet registration details
   */
  usePet: (id: string, enabled = true) =>
    useQuery({
      queryKey: registryKeys.petDetail(id),
      queryFn: () => api.pets.get(id),
      enabled: enabled && !!id,
    }),

  /**
   * List my pet registrations
   */
  useMyPets: (params?: { page?: number; pageSize?: number; status?: string }) =>
    useQuery({
      queryKey: [...registryKeys.myPets(), params],
      queryFn: () => api.pets.listMy(params),
    }),

  // ========================================================================
  // Pet Registration Mutations
  // ========================================================================

  /**
   * Create pet registration
   */
  useCreatePet: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: (data: CreatePetRegistrationRequest) => api.pets.create(data),
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: registryKeys.pets() });
        queryClient.invalidateQueries({ queryKey: registryKeys.myPets() });
      },
    });
  },

  /**
   * Update pet registration
   */
  useUpdatePet: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({ id, data }: { id: string; data: UpdatePetRegistrationRequest }) =>
        api.pets.update(id, data),
      onSuccess: (_, { id }) => {
        queryClient.invalidateQueries({ queryKey: registryKeys.petDetail(id) });
        queryClient.invalidateQueries({ queryKey: registryKeys.pets() });
        queryClient.invalidateQueries({ queryKey: registryKeys.myPets() });
      },
    });
  },

  /**
   * Delete pet registration
   */
  useDeletePet: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: (id: string) => api.pets.delete(id),
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: registryKeys.pets() });
        queryClient.invalidateQueries({ queryKey: registryKeys.myPets() });
      },
    });
  },

  /**
   * Review pet registration
   */
  useReviewPet: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({ id, data }: { id: string; data: ReviewRegistrationRequest }) =>
        api.pets.review(id, data),
      onSuccess: (_, { id }) => {
        queryClient.invalidateQueries({ queryKey: registryKeys.petDetail(id) });
        queryClient.invalidateQueries({ queryKey: registryKeys.pets() });
      },
    });
  },

  // ========================================================================
  // Vehicle Registration Queries
  // ========================================================================

  /**
   * List all vehicle registrations (managers)
   */
  useVehiclesList: (params?: ListRegistrationsParams, enabled = true) =>
    useQuery({
      queryKey: registryKeys.vehiclesList(params),
      queryFn: () => api.vehicles.list(params),
      enabled,
    }),

  /**
   * Get vehicle registration details
   */
  useVehicle: (id: string, enabled = true) =>
    useQuery({
      queryKey: registryKeys.vehicleDetail(id),
      queryFn: () => api.vehicles.get(id),
      enabled: enabled && !!id,
    }),

  /**
   * List my vehicle registrations
   */
  useMyVehicles: (params?: { page?: number; pageSize?: number; status?: string }) =>
    useQuery({
      queryKey: [...registryKeys.myVehicles(), params],
      queryFn: () => api.vehicles.listMy(params),
    }),

  // ========================================================================
  // Vehicle Registration Mutations
  // ========================================================================

  /**
   * Create vehicle registration
   */
  useCreateVehicle: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: (data: CreateVehicleRegistrationRequest) => api.vehicles.create(data),
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: registryKeys.vehicles() });
        queryClient.invalidateQueries({ queryKey: registryKeys.myVehicles() });
      },
    });
  },

  /**
   * Update vehicle registration
   */
  useUpdateVehicle: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({ id, data }: { id: string; data: UpdateVehicleRegistrationRequest }) =>
        api.vehicles.update(id, data),
      onSuccess: (_, { id }) => {
        queryClient.invalidateQueries({ queryKey: registryKeys.vehicleDetail(id) });
        queryClient.invalidateQueries({ queryKey: registryKeys.vehicles() });
        queryClient.invalidateQueries({ queryKey: registryKeys.myVehicles() });
      },
    });
  },

  /**
   * Delete vehicle registration
   */
  useDeleteVehicle: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: (id: string) => api.vehicles.delete(id),
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: registryKeys.vehicles() });
        queryClient.invalidateQueries({ queryKey: registryKeys.myVehicles() });
      },
    });
  },

  /**
   * Review vehicle registration
   */
  useReviewVehicle: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({ id, data }: { id: string; data: ReviewRegistrationRequest }) =>
        api.vehicles.review(id, data),
      onSuccess: (_, { id }) => {
        queryClient.invalidateQueries({ queryKey: registryKeys.vehicleDetail(id) });
        queryClient.invalidateQueries({ queryKey: registryKeys.vehicles() });
      },
    });
  },

  // ========================================================================
  // Parking Spots
  // ========================================================================

  /**
   * List parking spots in building
   */
  useParkingSpots: (buildingId: string, params?: ListParkingSpotsParams, enabled = true) =>
    useQuery({
      queryKey: registryKeys.parkingSpotsList(buildingId, params),
      queryFn: () => api.parkingSpots.list(buildingId, params),
      enabled: enabled && !!buildingId,
    }),

  /**
   * Create parking spot
   */
  useCreateParkingSpot: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({ buildingId, data }: { buildingId: string; data: CreateParkingSpotRequest }) =>
        api.parkingSpots.create(buildingId, data),
      onSuccess: (_, { buildingId }) => {
        queryClient.invalidateQueries({ queryKey: registryKeys.parkingSpots(buildingId) });
      },
    });
  },

  /**
   * Delete parking spot
   */
  useDeleteParkingSpot: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({ buildingId, spotId }: { buildingId: string; spotId: string }) =>
        api.parkingSpots.delete(buildingId, spotId),
      onSuccess: (_, { buildingId }) => {
        queryClient.invalidateQueries({ queryKey: registryKeys.parkingSpots(buildingId) });
      },
    });
  },

  // ========================================================================
  // Building Registry Rules
  // ========================================================================

  /**
   * Get building registry rules
   */
  useRegistryRules: (buildingId: string, enabled = true) =>
    useQuery({
      queryKey: registryKeys.rules(buildingId),
      queryFn: () => api.rules.get(buildingId),
      enabled: enabled && !!buildingId,
    }),

  /**
   * Update building registry rules
   */
  useUpdateRegistryRules: () => {
    const queryClient = useQueryClient();
    return useMutation({
      mutationFn: ({
        buildingId,
        data,
      }: {
        buildingId: string;
        data: UpdateRegistryRulesRequest;
      }) => api.rules.update(buildingId, data),
      onSuccess: (_, { buildingId }) => {
        queryClient.invalidateQueries({ queryKey: registryKeys.rules(buildingId) });
      },
    });
  },
});

export type RegistryHooks = ReturnType<typeof createRegistryHooks>;
