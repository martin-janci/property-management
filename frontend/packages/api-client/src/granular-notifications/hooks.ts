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
import {
  deleteInboxGroup,
  getEventTriggers,
  listInboxGroups,
  markAllInboxGroupsRead,
  markInboxGroupRead,
  resetEventTriggers,
  updateEventTrigger,
} from './api';
import type {
  EventTriggerPreference,
  EventTriggersResponse,
  InboxQuery,
  UpdateTriggerRequest,
} from './types';

export const notificationTriggerKeys = {
  all: ['notification-triggers'] as const,
  events: () => [...notificationTriggerKeys.all, 'events'] as const,
};

/**
 * A trigger counts as "enabled" when at least one channel is on — mirroring the
 * backend rollup (`push_enabled || email_enabled || in_app_enabled`, see
 * `api-server/src/routes/granular_notifications.rs::list_event_preferences`).
 */
function isTriggerEnabled(pref: EventTriggerPreference): boolean {
  return pref.pushEnabled || pref.emailEnabled || pref.inAppEnabled;
}

/**
 * Apply a single-trigger channel patch to a cached response, keeping the
 * per-category `enabledEvents` rollups in sync with the patched preferences.
 *
 * Without recomputing `categories[].enabledEvents`, an optimistic patch that
 * flips a trigger's *last channel off* (or its *first channel on*) leaves the
 * "X of Y enabled" badge stale until the settle-invalidate refetch resolves —
 * a visible flicker on slow networks (#2369). The predicate matches the backend
 * exactly, so this stays consistent with the server rollup that replaces it on
 * settle.
 */
export function applyTriggerOptimisticPatch(
  previous: EventTriggersResponse,
  eventType: string,
  patch: UpdateTriggerRequest
): EventTriggersResponse {
  const preferences = previous.preferences.map((pref) =>
    pref.eventType === eventType ? { ...pref, ...patch } : pref
  );
  const categories = previous.categories.map((category) => ({
    ...category,
    enabledEvents: preferences.filter(
      (pref) => pref.category === category.category && isTriggerEnabled(pref)
    ).length,
  }));
  return { ...previous, preferences, categories };
}

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
 * pattern). The patch also recomputes the category `enabledEvents` rollups so the
 * "X of Y enabled" badge tracks the toggle instantly (#2369); the settle-invalidate
 * then reconciles both against the server.
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
        queryClient.setQueryData<EventTriggersResponse>(
          notificationTriggerKeys.events(),
          applyTriggerOptimisticPatch(previous, eventType, patch)
        );
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

// ---------------------------------------------------------------------------
// In-app notification inbox
// ---------------------------------------------------------------------------

export const inboxKeys = {
  all: ['notification-inbox'] as const,
  list: (query: InboxQuery) => [...inboxKeys.all, 'list', query] as const,
  unread: () => [...inboxKeys.all, 'unread'] as const,
};

/** Poll interval for the unread badge. The WebSocket channel is best-effort. */
const UNREAD_POLL_MS = 60_000;

/** The caller's notification groups. */
export function useNotificationInbox(query: InboxQuery = {}) {
  return useQuery({
    queryKey: inboxKeys.list(query),
    queryFn: () => listInboxGroups(query),
  });
}

/** Unread-group count for the top-bar bell (polled). */
export function useUnreadNotificationCount(enabled = true) {
  return useQuery({
    queryKey: inboxKeys.unread(),
    queryFn: async () => (await listInboxGroups({ limit: 1 })).totalUnread,
    enabled,
    refetchInterval: UNREAD_POLL_MS,
    staleTime: 30_000,
  });
}

function useInboxMutation<TVars>(fn: (vars: TVars) => Promise<void>) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: fn,
    onSettled: () => queryClient.invalidateQueries({ queryKey: inboxKeys.all }),
  });
}

/** Mark one group read. */
export function useMarkInboxGroupRead() {
  return useInboxMutation((groupId: string) => markInboxGroupRead(groupId));
}

/** Mark all groups read. */
export function useMarkAllInboxRead() {
  return useInboxMutation<void>(() => markAllInboxGroupsRead());
}

/** Remove one group from the inbox. */
export function useDeleteInboxGroup() {
  return useInboxMutation((groupId: string) => deleteInboxGroup(groupId));
}
