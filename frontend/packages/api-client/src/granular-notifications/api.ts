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
import type { EventTriggerPreference, EventTriggersResponse, UpdateTriggerRequest } from './types';

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
