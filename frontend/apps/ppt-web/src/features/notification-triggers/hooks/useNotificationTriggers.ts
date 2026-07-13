/**
 * Notification trigger hooks (Story 84.4 — Notification Trigger System, PM gap 84-4).
 *
 * TanStack Query wrappers over the granular notification-event API:
 *   GET  /api/v1/users/me/notification-preferences/granular/events
 *   PUT  /api/v1/users/me/notification-preferences/granular/events/{eventType}
 *   POST /api/v1/users/me/notification-preferences/granular/events/reset
 *
 * These endpoints require authentication and are absent from the generated
 * `@ppt/api-client`, so we call the REST paths directly and read the bearer token
 * from localStorage — the same convention as `features/notification-analytics`,
 * `features/auth/authApiClient.ts`, and `features/privacy/gdprClient.ts`.
 */
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import type { EventTriggersResponse, UpdateTriggerRequest } from '../types';

const BASE_PATH = '/api/v1/users/me/notification-preferences/granular';
const ACCESS_TOKEN_KEY = 'ppt_access_token';

function readAccessToken(): string | undefined {
  try {
    return localStorage.getItem(ACCESS_TOKEN_KEY) ?? undefined;
  } catch {
    return undefined;
  }
}

function authHeaders(): Headers {
  const headers = new Headers({ 'Content-Type': 'application/json' });
  const token = readAccessToken();
  if (token) {
    headers.set('Authorization', `Bearer ${token}`);
  }
  return headers;
}

async function parseError(res: Response): Promise<Error & { status: number }> {
  const body = (await res.json().catch(() => null)) as { message?: string } | null;
  const error = new Error(body?.message || `Request failed (HTTP ${res.status})`) as Error & {
    status: number;
  };
  error.status = res.status;
  return error;
}

async function fetchTriggers(): Promise<EventTriggersResponse> {
  const res = await fetch(`${BASE_PATH}/events`, { headers: authHeaders() });
  if (!res.ok) {
    throw await parseError(res);
  }
  return res.json() as Promise<EventTriggersResponse>;
}

async function putTrigger(eventType: string, patch: UpdateTriggerRequest): Promise<void> {
  const res = await fetch(`${BASE_PATH}/events/${encodeURIComponent(eventType)}`, {
    method: 'PUT',
    headers: authHeaders(),
    body: JSON.stringify(patch),
  });
  if (!res.ok) {
    throw await parseError(res);
  }
}

async function postReset(): Promise<EventTriggersResponse> {
  const res = await fetch(`${BASE_PATH}/events/reset`, {
    method: 'POST',
    headers: authHeaders(),
  });
  if (!res.ok) {
    throw await parseError(res);
  }
  return res.json() as Promise<EventTriggersResponse>;
}

export const notificationTriggerKeys = {
  all: ['notification-triggers'] as const,
  events: () => [...notificationTriggerKeys.all, 'events'] as const,
};

/** Fetch the full set of notification triggers (event types) and category rollups. */
export function useNotificationTriggers() {
  return useQuery({
    queryKey: notificationTriggerKeys.events(),
    queryFn: fetchTriggers,
    staleTime: 30 * 1000,
  });
}

/**
 * Toggle a single channel on one trigger. Invalidates the trigger list on success
 * so the category rollups (`enabledEvents`) stay in sync.
 */
export function useUpdateNotificationTrigger() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ eventType, patch }: { eventType: string; patch: UpdateTriggerRequest }) =>
      putTrigger(eventType, patch),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: notificationTriggerKeys.events() });
    },
  });
}

/** Reset every trigger back to its role/system default and refresh the cache. */
export function useResetNotificationTriggers() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: postReset,
    onSuccess: (data) => {
      queryClient.setQueryData(notificationTriggerKeys.events(), data);
    },
  });
}
