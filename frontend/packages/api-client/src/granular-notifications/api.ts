/**
 * Granular Notification Triggers API (Story 84.4 — Notification Trigger System,
 * PM gap 84-4).
 *
 * Talks to the granular notification-event endpoints:
 *   GET  /api/v1/users/me/notification-preferences/granular/events
 *   PUT  /api/v1/users/me/notification-preferences/granular/events/{eventType}
 *   POST /api/v1/users/me/notification-preferences/granular/events/reset
 *
 * Uses the centralised authenticated fetch helper so auth-token handling and the
 * `401 mfa_required` challenge/retry are consistent with the rest of the
 * api-client (see `../lib/fetch`, #486) — the sibling `advanced-notifications`
 * module (Epic 40) over the same preference family already does this.
 */

import { authenticatedFetchJson } from '../lib/fetch';
import type {
  EventTriggerPreference,
  EventTriggersResponse,
  InboxQuery,
  InboxResponse,
  UpdateTriggerRequest,
} from './types';

const BASE_PATH = '/api/v1/users/me/notification-preferences/granular';

/** Fetch the full set of notification triggers (event types) and category rollups. */
export async function getEventTriggers(): Promise<EventTriggersResponse> {
  return authenticatedFetchJson<EventTriggersResponse>(`${BASE_PATH}/events`);
}

/**
 * Update the channel flags on a single trigger. The backend echoes the updated
 * `EventPreferenceWithDetails`, which callers may use for an optimistic cache patch.
 */
export async function updateEventTrigger(
  eventType: string,
  patch: UpdateTriggerRequest
): Promise<EventTriggerPreference> {
  return authenticatedFetchJson<EventTriggerPreference>(
    `${BASE_PATH}/events/${encodeURIComponent(eventType)}`,
    {
      method: 'PUT',
      body: JSON.stringify(patch),
    }
  );
}

/** Reset every trigger back to its role/system default and return the fresh set. */
export async function resetEventTriggers(): Promise<EventTriggersResponse> {
  return authenticatedFetchJson<EventTriggersResponse>(`${BASE_PATH}/events/reset`, {
    method: 'POST',
  });
}

// ---------------------------------------------------------------------------
// In-app notification inbox (Epic 29 notification groups)
//   GET    {BASE}/groups?includeRead=&limit=&offset=
//   POST   {BASE}/groups/{groupId}/read
//   POST   {BASE}/groups/read-all
//   DELETE {BASE}/groups/{groupId}
// ---------------------------------------------------------------------------

/** List the caller's notification groups, newest first. */
export async function listInboxGroups(query: InboxQuery = {}): Promise<InboxResponse> {
  const params = new URLSearchParams();
  if (query.includeRead) params.set('includeRead', 'true');
  if (query.limit !== undefined) params.set('limit', String(query.limit));
  if (query.offset !== undefined) params.set('offset', String(query.offset));
  const qs = params.toString();
  return authenticatedFetchJson<InboxResponse>(`${BASE_PATH}/groups${qs ? `?${qs}` : ''}`);
}

/** Mark one group (and its notifications) as read. */
export async function markInboxGroupRead(groupId: string): Promise<void> {
  await authenticatedFetchJson<unknown>(`${BASE_PATH}/groups/${encodeURIComponent(groupId)}/read`, {
    method: 'POST',
  });
}

/** Mark every group as read. */
export async function markAllInboxGroupsRead(): Promise<void> {
  await authenticatedFetchJson<unknown>(`${BASE_PATH}/groups/read-all`, { method: 'POST' });
}

/** Remove a group from the inbox. */
export async function deleteInboxGroup(groupId: string): Promise<void> {
  await authenticatedFetchJson<unknown>(`${BASE_PATH}/groups/${encodeURIComponent(groupId)}`, {
    method: 'DELETE',
  });
}
