/**
 * Notification Preferences API (Epic 8A, Story 8A.1)
 *
 * Uses the centralised authenticated fetch helper so auth token handling is
 * consistent with the rest of the api-client (see lib/fetch.ts, #486).
 */

import { getToken } from '../auth';
import { authenticatedFetchJson } from '../lib/fetch';
import type {
  NotificationChannel,
  NotificationPreferencesResponse,
  UpdateNotificationPreferenceRequest,
  UpdatePreferenceResponse,
} from './types';

const API_BASE = '/api/v1/users/me/notification-preferences';

/**
 * Fetch all notification preferences for the current user.
 */
export async function getNotificationPreferences(): Promise<NotificationPreferencesResponse> {
  return authenticatedFetchJson<NotificationPreferencesResponse>(API_BASE);
}

/**
 * Update a specific notification channel preference.
 *
 * Throws `ConfirmationRequiredError` when the server responds 409 (would
 * disable all channels — caller must confirm before retrying with
 * `confirmDisableAll: true`).
 *
 * Uses a raw fetch (rather than authenticatedFetchJson) so that we can inspect
 * the HTTP status code and surface the typed 409 → ConfirmationRequiredError.
 */
export async function updateNotificationPreference(
  channel: NotificationChannel,
  request: UpdateNotificationPreferenceRequest
): Promise<UpdatePreferenceResponse> {
  const token = getToken();
  const response = await fetch(`${API_BASE}/${channel}`, {
    method: 'PATCH',
    headers: {
      'Content-Type': 'application/json',
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    },
    body: JSON.stringify(request),
  });

  if (!response.ok) {
    let errorMessage = 'Failed to update notification preference';
    let errorData: { error?: { message?: string } } = {};
    try {
      errorData = await response.json();
      errorMessage = errorData.error?.message || errorMessage;
    } catch {
      // Response is not JSON, use default message
    }

    if (response.status === 409) {
      throw new ConfirmationRequiredError(
        errorData.error?.message || 'Confirmation required to disable all channels',
        channel
      );
    }

    throw new Error(errorMessage);
  }

  return response.json();
}

/**
 * Custom error for when confirmation is required to disable all channels.
 */
export class ConfirmationRequiredError extends Error {
  channel: NotificationChannel;

  constructor(message: string, channel: NotificationChannel) {
    super(message);
    this.name = 'ConfirmationRequiredError';
    this.channel = channel;
  }
}
