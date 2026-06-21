/**
 * Sentiment Analysis Hooks (Epic 13, Story 13.2)
 *
 * TanStack Query hooks for the sentiment API.
 */
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import type {
  SentimentAlert,
  SentimentDashboard,
  SentimentThresholds,
  SentimentTrend,
  SentimentTrendQuery,
  UpdateSentimentThresholdsRequest,
} from '../types';

const API_BASE = '/api/v1/ai/sentiment';

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
    queryFn: () => apiFetch<SentimentDashboard>(`${API_BASE}/dashboard`),
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
    queryFn: () =>
      apiFetch<{ trends: SentimentTrend[] }>(`${API_BASE}/trends${qs ? `?${qs}` : ''}`).then(
        (r) => r.trends
      ),
    staleTime: 60 * 1000,
  });
}

export function useSentimentAlerts(acknowledged?: boolean) {
  const params = new URLSearchParams();
  if (acknowledged !== undefined) params.set('acknowledged', String(acknowledged));

  return useQuery({
    queryKey: sentimentKeys.alerts(acknowledged),
    queryFn: () =>
      apiFetch<{ alerts: SentimentAlert[] }>(
        `${API_BASE}/alerts${params.toString() ? `?${params}` : ''}`
      ).then((r) => r.alerts),
    staleTime: 30 * 1000,
  });
}

export function useSentimentThresholds() {
  return useQuery({
    queryKey: sentimentKeys.thresholds(),
    queryFn: () => apiFetch<SentimentThresholds>(`${API_BASE}/thresholds`),
    staleTime: 5 * 60 * 1000,
  });
}

export function useAcknowledgeSentimentAlert() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (alertId: string) =>
      apiFetch<SentimentAlert>(`${API_BASE}/alerts/${alertId}/acknowledge`, { method: 'POST' }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: sentimentKeys.all });
    },
  });
}

export function useUpdateSentimentThresholds() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (req: UpdateSentimentThresholdsRequest) =>
      apiFetch<SentimentThresholds>(`${API_BASE}/thresholds`, {
        method: 'PUT',
        body: JSON.stringify(req),
      }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: sentimentKeys.thresholds() });
    },
  });
}
