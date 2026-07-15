/**
 * Granular Notification Trigger Hooks (Story 84.4 — Notification Trigger System,
 * PM gap 84-4).
 *
 * TanStack Query wrappers over the granular notification-event API. Auth is
 * handled by the global token provider via `authenticatedFetchJson` (see #486),
 * so callers pass neither baseUrl nor accessToken — consistent with the rest of
 * the hooks layer and the sibling `advanced-notifications` module (Epic 40).
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { getEventTriggers, resetEventTriggers, updateEventTrigger } from './api';
import type { EventTriggersResponse, UpdateTriggerRequest } from './types';

export const notificationTriggerKeys = {
  all: ['notification-triggers'] as const,
  events: () => [...notificationTriggerKeys.all, 'events'] as const,
};

/** Fetch the full set of notification triggers (event types) and category rollups. */
export function useNotificationTriggers() {
  return useQuery({
    queryKey: notificationTriggerKeys.events(),
    queryFn: getEventTriggers,
    staleTime: 30 * 1000,
  });
}

/**
 * Toggle a single channel on one trigger, with an optimistic cache patch so the
 * checkbox flips immediately and rolls back on error (the `advanced-notifications`
 * pattern). The settle-invalidate keeps the category `enabledEvents` rollups in
 * sync with the server.
 */
export function useUpdateNotificationTrigger() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ eventType, patch }: { eventType: string; patch: UpdateTriggerRequest }) =>
      updateEventTrigger(eventType, patch),

    onMutate: async ({ eventType, patch }) => {
      await queryClient.cancelQueries({ queryKey: notificationTriggerKeys.events() });

      const previous = queryClient.getQueryData<EventTriggersResponse>(
        notificationTriggerKeys.events()
      );

      if (previous) {
        queryClient.setQueryData<EventTriggersResponse>(notificationTriggerKeys.events(), {
          ...previous,
          preferences: previous.preferences.map((pref) =>
            pref.eventType === eventType ? { ...pref, ...patch } : pref
          ),
        });
      }

      return { previous };
    },

    onError: (_err, _variables, context) => {
      if (context?.previous) {
        queryClient.setQueryData(notificationTriggerKeys.events(), context.previous);
      }
    },

    onSettled: () => {
      void queryClient.invalidateQueries({ queryKey: notificationTriggerKeys.events() });
    },
  });
}

/** Reset every trigger back to its role/system default and refresh the cache. */
export function useResetNotificationTriggers() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: resetEventTriggers,
    onSuccess: (data) => {
      queryClient.setQueryData(notificationTriggerKeys.events(), data);
    },
  });
}
