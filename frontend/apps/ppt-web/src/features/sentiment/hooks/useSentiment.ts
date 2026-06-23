/**
 * Sentiment Analysis Hooks (Epic 13, Story 13.2)
 *
 * TanStack Query hooks for the sentiment API.
 */
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { getApiClient } from '../../../lib/api';
import type {
  SentimentAlert,
  SentimentDashboard,
  SentimentThresholds,
  SentimentTrend,
  SentimentTrendQuery,
  UpdateSentimentThresholdsRequest,
} from '../types';

// Path is relative to the api-client baseURL (`/api/v1`). Routing through
// `getApiClient()` ensures the shared axios interceptors apply: Bearer-token
// injection, ErrorResponse → ApiError transformation, 401 → onUnauthorized,
// and transient 5xx/429 retry with backoff.
const API_BASE = '/ai/sentiment';

export const sentimentKeys = {
  all: ['sentiment'] as const,
  dashboard: () => [...sentimentKeys.all, 'dashboard'] as const,
  trends: (query?: SentimentTrendQuery) => [...sentimentKeys.all, 'trends', query] as const,
  alerts: (acknowledged?: boolean) => [...sentimentKeys.all, 'alerts', acknowledged] as const,
  thresholds: () => [...sentimentKeys.all, 'thresholds'] as const,
};

export function useSentimentDashboard() {
  return useQuery({
    queryKey: sentimentKeys.dashboard(),
    queryFn: async () => {
      const res = await getApiClient().get<SentimentDashboard>(`${API_BASE}/dashboard`);
      return res.data;
    },
    staleTime: 60 * 1000,
  });
}

export function useSentimentTrends(query?: SentimentTrendQuery) {
  const params = new URLSearchParams();
  if (query?.buildingId) params.set('building_id', query.buildingId);
  if (query?.fromDate) params.set('from_date', query.fromDate);
  if (query?.toDate) params.set('to_date', query.toDate);
  if (query?.limit) params.set('limit', String(query.limit));
  const qs = params.toString();

  return useQuery({
    queryKey: sentimentKeys.trends(query),
    queryFn: async () => {
      const res = await getApiClient().get<{ trends: SentimentTrend[] }>(
        `${API_BASE}/trends${qs ? `?${qs}` : ''}`
      );
      return res.data.trends;
    },
    staleTime: 60 * 1000,
  });
}

export function useSentimentAlerts(acknowledged?: boolean) {
  const params = new URLSearchParams();
  if (acknowledged !== undefined) params.set('acknowledged', String(acknowledged));

  return useQuery({
    queryKey: sentimentKeys.alerts(acknowledged),
    queryFn: async () => {
      const res = await getApiClient().get<{ alerts: SentimentAlert[] }>(
        `${API_BASE}/alerts${params.toString() ? `?${params}` : ''}`
      );
      return res.data.alerts;
    },
    staleTime: 30 * 1000,
  });
}

export function useSentimentThresholds() {
  return useQuery({
    queryKey: sentimentKeys.thresholds(),
    queryFn: async () => {
      const res = await getApiClient().get<SentimentThresholds>(`${API_BASE}/thresholds`);
      return res.data;
    },
    staleTime: 5 * 60 * 1000,
  });
}

export function useAcknowledgeSentimentAlert() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (alertId: string) => {
      const res = await getApiClient().post<SentimentAlert>(
        `${API_BASE}/alerts/${alertId}/acknowledge`
      );
      return res.data;
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: sentimentKeys.all });
    },
  });
}

export function useUpdateSentimentThresholds() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (req: UpdateSentimentThresholdsRequest) => {
      const res = await getApiClient().put<SentimentThresholds>(`${API_BASE}/thresholds`, req);
      return res.data;
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: sentimentKeys.thresholds() });
    },
  });
}
