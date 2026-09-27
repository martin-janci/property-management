/**
 * Notification analytics hook (Story 2B-C.3, BIT-214).
 *
 * TanStack Query wrapper over `GET /api/v1/admin/notifications/analytics`. The
 * endpoint follows the `admin/audit` precedent: capability-gated (`audit_read`)
 * and absent from the generated `@ppt/api-client`, so we call the REST path
 * directly through the shared axios client (`getApiClient()`). Routing through
 * that client stamps `Authorization: Bearer <token>` via the request
 * interceptor (a bare `fetch` bypassed it — the manual localStorage token read
 * also missed the single-flight 401 refresh/replay, ErrorResponse → ApiError
 * transformation, and transient-failure retry the interceptor provides, #2982).
 * The path is relative to the client baseURL (`/api/v1`); the rejected ApiError
 * still carries `.status` for consumers that branch on it.
 */
import { useQuery } from '@tanstack/react-query';
import { getApiClient } from '../../../lib/api';
import type { NotificationAnalyticsFilters, NotificationAnalyticsResponse } from '../types';

const ANALYTICS_PATH = '/admin/notifications/analytics';

async function fetchAnalytics(
  filters: NotificationAnalyticsFilters
): Promise<NotificationAnalyticsResponse> {
  // Relative lower-bound alias; backend defaults to 24h when omitted.
  const params: Record<string, string> = { after: filters.window };
  if (filters.channel) {
    params.channel = filters.channel;
  }

  const res = await getApiClient().get<NotificationAnalyticsResponse>(ANALYTICS_PATH, { params });
  return res.data;
}

export const notificationAnalyticsKeys = {
  all: ['notification-analytics'] as const,
  query: (filters: NotificationAnalyticsFilters) =>
    [...notificationAnalyticsKeys.all, filters] as const,
};

/**
 * Fetch per-channel notification delivery analytics for the selected window.
 * Refetches on filter change; kept fresh for 30s so the alert banner reflects
 * recent failures without hammering the endpoint.
 */
export function useNotificationAnalytics(filters: NotificationAnalyticsFilters) {
  return useQuery({
    queryKey: notificationAnalyticsKeys.query(filters),
    queryFn: () => fetchAnalytics(filters),
    staleTime: 30 * 1000,
  });
}
