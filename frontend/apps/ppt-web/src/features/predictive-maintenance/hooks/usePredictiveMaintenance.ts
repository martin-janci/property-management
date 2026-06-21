/**
 * Predictive Maintenance Hooks (Epic 13, Story 13.3)
 */
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import type { Equipment, EquipmentQuery, MaintenancePrediction, PredictionsQuery } from '../types';

const EQUIPMENT_API = '/api/v1/ai/equipment';

async function apiFetch<T>(url: string, options?: RequestInit): Promise<T> {
  const res = await fetch(url, {
    ...options,
    headers: { 'Content-Type': 'application/json', ...options?.headers },
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({ message: 'Request failed' }));
    throw new Error(err.message || `HTTP ${res.status}`);
  }
  return res.json() as Promise<T>;
}

export const predictiveKeys = {
  all: ['predictive-maintenance'] as const,
  equipment: (q?: EquipmentQuery) => [...predictiveKeys.all, 'equipment', q] as const,
  predictions: (q?: PredictionsQuery) => [...predictiveKeys.all, 'predictions', q] as const,
  needingMaintenance: () => [...predictiveKeys.all, 'needing-maintenance'] as const,
};

export function useEquipmentList(query?: EquipmentQuery) {
  const params = new URLSearchParams();
  if (query?.buildingId) params.set('building_id', query.buildingId);
  if (query?.category) params.set('category', query.category);
  if (query?.status) params.set('status', query.status);
  if (query?.limit) params.set('limit', String(query.limit));
  if (query?.offset) params.set('offset', String(query.offset));
  const qs = params.toString();

  return useQuery({
    queryKey: predictiveKeys.equipment(query),
    queryFn: () => apiFetch<{ equipment: Equipment[] }>(`${EQUIPMENT_API}${qs ? `?${qs}` : ''}`),
    staleTime: 60 * 1000,
  });
}

export function useMaintenancePredictions(query?: PredictionsQuery) {
  const params = new URLSearchParams();
  if (query?.limit) params.set('limit', String(query.limit));
  const qs = params.toString();

  return useQuery({
    queryKey: predictiveKeys.predictions(query),
    queryFn: () =>
      apiFetch<{ predictions: MaintenancePrediction[] }>(
        `${EQUIPMENT_API}/predictions${qs ? `?${qs}` : ''}`
      ),
    staleTime: 60 * 1000,
  });
}

export function useNeedingMaintenance(daysAhead = 30) {
  return useQuery({
    queryKey: predictiveKeys.needingMaintenance(),
    queryFn: () =>
      apiFetch<{ equipment: Equipment[] }>(
        `${EQUIPMENT_API}/needing-maintenance?days_ahead=${daysAhead}`
      ),
    staleTime: 5 * 60 * 1000,
  });
}

export function useAcknowledgeMaintenancePrediction() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (predictionId: string) =>
      apiFetch<MaintenancePrediction>(`${EQUIPMENT_API}/predictions/${predictionId}/acknowledge`, {
        method: 'POST',
        body: JSON.stringify({}),
      }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: predictiveKeys.all });
    },
  });
}
