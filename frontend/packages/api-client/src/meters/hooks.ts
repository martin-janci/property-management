/**
 * TanStack Query hooks for the meters API (Epic 12).
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import * as api from './api';
import type {
  ConsumptionQuery,
  ListMetersQuery,
  ListReadingsQuery,
  PendingReadingsQuery,
  SubmitReadingRequest,
  ValidateReadingRequest,
} from './types';

// ============================================================================
// Query keys
// ============================================================================

export const meterKeys = {
  all: ['meters'] as const,
  buildingLists: () => [...meterKeys.all, 'building-list'] as const,
  buildingList: (buildingId: string, query: ListMetersQuery) =>
    [...meterKeys.buildingLists(), buildingId, query] as const,
  unitList: (unitId: string) => [...meterKeys.all, 'unit-list', unitId] as const,
  details: () => [...meterKeys.all, 'detail'] as const,
  detail: (id: string) => [...meterKeys.details(), id] as const,
  readings: (meterId: string, query: ListReadingsQuery) =>
    [...meterKeys.all, 'readings', meterId, query] as const,
  reading: (id: string) => [...meterKeys.all, 'reading', id] as const,
  pending: (query: PendingReadingsQuery) => [...meterKeys.all, 'pending', query] as const,
  consumption: (meterId: string, query: ConsumptionQuery) =>
    [...meterKeys.all, 'consumption', meterId, query] as const,
};

// ============================================================================
// Queries
// ============================================================================

/** List meters for a building. */
export function useMetersForBuilding(buildingId: string | undefined, query: ListMetersQuery = {}) {
  return useQuery({
    queryKey: meterKeys.buildingList(buildingId ?? '', query),
    queryFn: () => api.listMetersForBuilding(buildingId as string, query),
    enabled: !!buildingId,
  });
}

/** List meters for a unit. */
export function useUnitMeters(unitId: string | undefined) {
  return useQuery({
    queryKey: meterKeys.unitList(unitId ?? ''),
    queryFn: () => api.listMetersForUnit(unitId as string),
    enabled: !!unitId,
  });
}

/** Get a meter with its recent readings. */
export function useMeter(id: string | undefined) {
  return useQuery({
    queryKey: meterKeys.detail(id ?? ''),
    queryFn: () => api.getMeter(id as string),
    enabled: !!id,
  });
}

/** Get a single reading. */
export function useReading(id: string | undefined) {
  return useQuery({
    queryKey: meterKeys.reading(id ?? ''),
    queryFn: () => api.getReading(id as string),
    enabled: !!id,
  });
}

/** List readings for a meter. */
export function useMeterReadings(meterId: string | undefined, query: ListReadingsQuery = {}) {
  return useQuery({
    queryKey: meterKeys.readings(meterId ?? '', query),
    queryFn: () => api.listReadings(meterId as string, query),
    enabled: !!meterId,
  });
}

/** Manager queue of pending readings for an organization. */
export function usePendingReadings(query: PendingReadingsQuery | undefined) {
  return useQuery({
    queryKey: meterKeys.pending(query ?? { organization_id: '' }),
    queryFn: () => api.getPendingReadings(query as PendingReadingsQuery),
    enabled: !!query?.organization_id,
  });
}

/** Consumption history for a meter. */
export function useConsumptionHistory(
  meterId: string | undefined,
  query: ConsumptionQuery | undefined
) {
  return useQuery({
    queryKey: meterKeys.consumption(meterId ?? '', query ?? { from: '', to: '' }),
    queryFn: () => api.getConsumptionHistory(meterId as string, query as ConsumptionQuery),
    enabled: !!meterId && !!query?.from && !!query?.to,
  });
}

// ============================================================================
// Mutations
// ============================================================================

/** Submit a meter reading; invalidates that meter and its readings. */
export function useSubmitReading() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (body: SubmitReadingRequest) => api.submitReading(body),
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({ queryKey: meterKeys.detail(variables.meter_id) });
      queryClient.invalidateQueries({ queryKey: meterKeys.all });
    },
  });
}

/** Validate a reading; invalidates the pending queue and reading caches. */
export function useValidateReading() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, body }: { id: string; body: ValidateReadingRequest }) =>
      api.validateReading(id, body),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: meterKeys.all });
    },
  });
}
