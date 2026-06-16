/**
 * Notification Preferences Hooks (Epic 8A, Story 8A.1)
 *
 * Auth is handled via the global token provider (set by AuthContext) so callers
 * do not need to pass baseUrl or accessToken — consistent with the rest of the
 * hooks layer (see #486).
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  ConfirmationRequiredError,
  getNotificationPreferences,
  updateNotificationPreference,
} from './api';
import type {
  NotificationChannel,
  NotificationPreferencesResponse,
  UpdateNotificationPreferenceRequest,
} from './types';

/** Query key for notification preferences */
export const NOTIFICATION_PREFERENCES_KEY = ['notification-preferences'] as const;

/**
 * Hook to fetch notification preferences for the current user.
 */
export function useNotificationPreferences() {
  return useQuery({
    queryKey: NOTIFICATION_PREFERENCES_KEY,
    queryFn: () => getNotificationPreferences(),
  });
}

/**
 * Hook to update a notification preference with optimistic updates and error rollback.
 */
export function useUpdateNotificationPreference() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      channel,
      request,
    }: {
      channel: NotificationChannel;
      request: UpdateNotificationPreferenceRequest;
    }) => updateNotificationPreference(channel, request),

    // Optimistic update: immediately reflect the toggle in the UI.
    onMutate: async ({ channel, request }) => {
      // Cancel any outgoing refetches so they don't overwrite optimistic state.
      await queryClient.cancelQueries({ queryKey: NOTIFICATION_PREFERENCES_KEY });

      // Snapshot the previous value for rollback on error.
      const previousPreferences = queryClient.getQueryData<NotificationPreferencesResponse>(
        NOTIFICATION_PREFERENCES_KEY
      );

      // Optimistically update the cache.
      if (previousPreferences) {
        queryClient.setQueryData<NotificationPreferencesResponse>(NOTIFICATION_PREFERENCES_KEY, {
          ...previousPreferences,
          preferences: previousPreferences.preferences.map((pref) =>
            pref.channel === channel ? { ...pref, enabled: request.enabled } : pref
          ),
        });
      }

      return { previousPreferences };
    },

    // Error rollback: restore the previous preferences.
    onError: (_err, _variables, context) => {
      if (context?.previousPreferences) {
        queryClient.setQueryData(NOTIFICATION_PREFERENCES_KEY, context.previousPreferences);
      }
    },

    // Always refetch after settling to ensure server state is reflected.
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: NOTIFICATION_PREFERENCES_KEY });
    },
  });
}

export { ConfirmationRequiredError };
