/**
 * Notification analytics hook (Story 2B-C.3, BIT-214).
 *
 * TanStack Query wrapper over `GET /api/v1/admin/notifications/analytics`. The
 * endpoint follows the `admin/audit` precedent: capability-gated (`audit_read`)
 * and absent from the generated `@ppt/api-client`, so we call the REST path
 * directly. The bearer token is read from localStorage (same convention as
 * `features/auth/authApiClient.ts` / `features/privacy/gdprClient.ts`) because
 * the handler requires authentication — a bare `fetch` would 401.
 */
import { useQuery } from '@tanstack/react-query';
import type { NotificationAnalyticsFilters, NotificationAnalyticsResponse } from '../types';

const ANALYTICS_PATH = '/api/v1/admin/notifications/analytics';
const ACCESS_TOKEN_KEY = 'ppt_access_token';

function readAccessToken(): string | undefined {
  try {
    return localStorage.getItem(ACCESS_TOKEN_KEY) ?? undefined;
  } catch {
    return undefined;
  }
}

async function fetchAnalytics(
  filters: NotificationAnalyticsFilters
): Promise<NotificationAnalyticsResponse> {
  const params = new URLSearchParams();
  // Relative lower-bound alias; backend defaults to 24h when omitted.
  params.set('after', filters.window);
  if (filters.channel) {
    params.set('channel', filters.channel);
  }

  const token = readAccessToken();
  const headers = new Headers({ 'Content-Type': 'application/json' });
  if (token) {
    headers.set('Authorization', `Bearer ${token}`);
  }

  const res = await fetch(`${ANALYTICS_PATH}?${params.toString()}`, { headers });
  if (!res.ok) {
    const body = (await res.json().catch(() => null)) as { message?: string } | null;
    const error = new Error(body?.message || `Request failed (HTTP ${res.status})`) as Error & {
      status: number;
    };
    error.status = res.status;
    throw error;
  }
  return res.json() as Promise<NotificationAnalyticsResponse>;
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
