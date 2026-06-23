/**
 * Predictive Maintenance Hooks (Epic 13, Story 13.3)
 */
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { getApiClient } from '../../../lib/api';
import type { Equipment, EquipmentQuery, MaintenancePrediction, PredictionsQuery } from '../types';

// Path is relative to the api-client baseURL (`/api/v1`). Routing through
// `getApiClient()` ensures the shared axios interceptors apply: Bearer-token
// injection, ErrorResponse → ApiError transformation, 401 → onUnauthorized,
// and transient 5xx/429 retry with backoff.
const EQUIPMENT_API = '/ai/equipment';

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
    queryFn: async () => {
      const res = await getApiClient().get<{ equipment: Equipment[] }>(
        `${EQUIPMENT_API}${qs ? `?${qs}` : ''}`
      );
      return res.data;
    },
    staleTime: 60 * 1000,
  });
}

export function useMaintenancePredictions(query?: PredictionsQuery) {
  const params = new URLSearchParams();
  if (query?.limit) params.set('limit', String(query.limit));
  const qs = params.toString();

  return useQuery({
    queryKey: predictiveKeys.predictions(query),
    queryFn: async () => {
      const res = await getApiClient().get<{ predictions: MaintenancePrediction[] }>(
        `${EQUIPMENT_API}/predictions${qs ? `?${qs}` : ''}`
      );
      return res.data;
    },
    staleTime: 60 * 1000,
  });
}

export function useNeedingMaintenance(daysAhead = 30) {
  return useQuery({
    queryKey: predictiveKeys.needingMaintenance(),
    queryFn: async () => {
      const res = await getApiClient().get<{ equipment: Equipment[] }>(
        `${EQUIPMENT_API}/needing-maintenance?days_ahead=${daysAhead}`
      );
      return res.data;
    },
    staleTime: 5 * 60 * 1000,
  });
}

export function useAcknowledgeMaintenancePrediction() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (predictionId: string) => {
      const res = await getApiClient().post<MaintenancePrediction>(
        `${EQUIPMENT_API}/predictions/${predictionId}/acknowledge`,
        {}
      );
      return res.data;
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: predictiveKeys.all });
    },
  });
}
