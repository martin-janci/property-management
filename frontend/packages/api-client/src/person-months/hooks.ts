/**
 * TanStack Query hooks for the person-months API (Epic 3, Story 3.5).
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import * as api from './api';
import type {
  BuildingPersonMonthsQuery,
  BulkUpsertPersonMonthsRequest,
  CalculatePersonMonthRequest,
  GetUnitPersonMonthsQuery,
  UpdatePersonMonthRequest,
  UpsertPersonMonthRequest,
} from './types';

// ============================================================================
// Query keys
// ============================================================================

export const personMonthKeys = {
  all: ['person-months'] as const,
  unit: (buildingId: string, unitId: string) =>
    [...personMonthKeys.all, 'unit', buildingId, unitId] as const,
  unitList: (buildingId: string, unitId: string, query: GetUnitPersonMonthsQuery) =>
    [...personMonthKeys.unit(buildingId, unitId), 'list', query] as const,
  unitYearly: (buildingId: string, unitId: string, year: number) =>
    [...personMonthKeys.unit(buildingId, unitId), 'yearly', year] as const,
  entry: (buildingId: string, unitId: string, id: string) =>
    [...personMonthKeys.unit(buildingId, unitId), 'entry', id] as const,
  building: (buildingId: string) => [...personMonthKeys.all, 'building', buildingId] as const,
  buildingList: (buildingId: string, query: BuildingPersonMonthsQuery) =>
    [...personMonthKeys.building(buildingId), 'list', query] as const,
  buildingSummary: (buildingId: string, query: BuildingPersonMonthsQuery) =>
    [...personMonthKeys.building(buildingId), 'summary', query] as const,
};

// ============================================================================
// Queries
// ============================================================================

/** Person-months for a unit (whole year or a single month). */
export function useUnitPersonMonths(
  buildingId: string | undefined,
  unitId: string | undefined,
  query: GetUnitPersonMonthsQuery
) {
  return useQuery({
    queryKey: personMonthKeys.unitList(buildingId ?? '', unitId ?? '', query),
    queryFn: () => api.getUnitPersonMonths(buildingId as string, unitId as string, query),
    enabled: !!buildingId && !!unitId,
  });
}

/** Yearly summary (per-month counts + total) for a unit. */
export function useUnitYearlySummary(
  buildingId: string | undefined,
  unitId: string | undefined,
  year: number
) {
  return useQuery({
    queryKey: personMonthKeys.unitYearly(buildingId ?? '', unitId ?? '', year),
    queryFn: () => api.getYearlySummary(buildingId as string, unitId as string, year),
    enabled: !!buildingId && !!unitId,
  });
}

/** A single person-month entry by id. */
export function usePersonMonth(
  buildingId: string | undefined,
  unitId: string | undefined,
  id: string | undefined
) {
  return useQuery({
    queryKey: personMonthKeys.entry(buildingId ?? '', unitId ?? '', id ?? ''),
    queryFn: () => api.getPersonMonth(buildingId as string, unitId as string, id as string),
    enabled: !!buildingId && !!unitId && !!id,
  });
}

/** Per-unit person-months for a building for a given period. */
export function useBuildingPersonMonths(
  buildingId: string | undefined,
  query: BuildingPersonMonthsQuery
) {
  return useQuery({
    queryKey: personMonthKeys.buildingList(buildingId ?? '', query),
    queryFn: () => api.listBuildingPersonMonths(buildingId as string, query),
    enabled: !!buildingId,
  });
}

/** Building-level aggregate summary for a period. */
export function useBuildingPersonMonthSummary(
  buildingId: string | undefined,
  query: BuildingPersonMonthsQuery
) {
  return useQuery({
    queryKey: personMonthKeys.buildingSummary(buildingId ?? '', query),
    queryFn: () => api.getBuildingSummary(buildingId as string, query),
    enabled: !!buildingId,
  });
}

// ============================================================================
// Mutations
// ============================================================================

/** Upsert a unit's person-month; invalidates that unit's and building caches. */
export function useUpsertPersonMonth(buildingId: string, unitId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (body: UpsertPersonMonthRequest) => api.upsertPersonMonth(buildingId, unitId, body),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: personMonthKeys.unit(buildingId, unitId) });
      queryClient.invalidateQueries({ queryKey: personMonthKeys.building(buildingId) });
    },
  });
}

/** Update an existing person-month entry. */
export function useUpdatePersonMonth(buildingId: string, unitId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, body }: { id: string; body: UpdatePersonMonthRequest }) =>
      api.updatePersonMonth(buildingId, unitId, id, body),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: personMonthKeys.unit(buildingId, unitId) });
      queryClient.invalidateQueries({ queryKey: personMonthKeys.building(buildingId) });
    },
  });
}

/** Delete a person-month entry. */
export function useDeletePersonMonth(buildingId: string, unitId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.deletePersonMonth(buildingId, unitId, id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: personMonthKeys.unit(buildingId, unitId) });
      queryClient.invalidateQueries({ queryKey: personMonthKeys.building(buildingId) });
    },
  });
}

/** Bulk upsert person-months for a building; invalidates the building caches. */
export function useBulkUpsertPersonMonths(buildingId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (body: BulkUpsertPersonMonthsRequest) =>
      api.bulkUpsertPersonMonths(buildingId, body),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: personMonthKeys.all });
    },
  });
}

/** Calculate a suggested count from residents (for pre-populating the form). */
export function useCalculateFromResidents(buildingId: string, unitId: string) {
  return useMutation({
    mutationFn: (body: CalculatePersonMonthRequest) =>
      api.calculateFromResidents(buildingId, unitId, body),
  });
}
