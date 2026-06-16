/**
 * Advanced Notifications Hooks (Epic 40)
 *
 * Auth is handled via the global token provider (set by AuthContext) so callers
 * do not need to pass baseUrl or accessToken — consistent with the rest of the
 * hooks layer (see #486).
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  getAdvancedPreferences,
  getCategoryPreferences,
  getDigestPreferences,
  getGroupingPreferences,
  getQuietHours,
  updateCategoryPreference,
  updateDigestPreferences,
  updateGroupingPreferences,
  updateQuietHours,
} from './api';
import type {
  CategoryPreferencesResponse,
  DigestResponse,
  GroupingResponse,
  NotificationCategory,
  QuietHoursResponse,
  UpdateCategoryPreferenceRequest,
  UpdateDigestRequest,
  UpdateGroupingRequest,
  UpdateQuietHoursRequest,
} from './types';

// ============================================================================
// Query Keys
// ============================================================================

export const CATEGORY_PREFERENCES_KEY = ['notification-preferences', 'categories'] as const;
export const QUIET_HOURS_KEY = ['notification-preferences', 'quiet-hours'] as const;
export const DIGEST_PREFERENCES_KEY = ['notification-preferences', 'digest'] as const;
export const GROUPING_PREFERENCES_KEY = ['notification-preferences', 'grouping'] as const;
export const ADVANCED_PREFERENCES_KEY = ['notification-preferences', 'advanced'] as const;

// ============================================================================
// Story 40.1: Category Preferences Hooks
// ============================================================================

/**
 * Hook to fetch category-based notification preferences.
 */
export function useCategoryPreferences() {
  return useQuery({
    queryKey: CATEGORY_PREFERENCES_KEY,
    queryFn: () => getCategoryPreferences(),
  });
}

/**
 * Hook to update a category preference with optimistic updates and error rollback.
 */
export function useUpdateCategoryPreference() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      category,
      request,
    }: {
      category: NotificationCategory;
      request: UpdateCategoryPreferenceRequest;
    }) => updateCategoryPreference(category, request),

    onMutate: async ({ category, request }) => {
      await queryClient.cancelQueries({ queryKey: CATEGORY_PREFERENCES_KEY });

      const previousData =
        queryClient.getQueryData<CategoryPreferencesResponse>(CATEGORY_PREFERENCES_KEY);

      if (previousData) {
        queryClient.setQueryData<CategoryPreferencesResponse>(CATEGORY_PREFERENCES_KEY, {
          ...previousData,
          categories: previousData.categories.map((cat) =>
            cat.category === category
              ? {
                  ...cat,
                  channels: { ...cat.channels, ...request.channels },
                  updatedAt: new Date().toISOString(),
                }
              : cat
          ),
        });
      }

      return { previousData };
    },

    onError: (_err, _variables, context) => {
      if (context?.previousData) {
        queryClient.setQueryData(CATEGORY_PREFERENCES_KEY, context.previousData);
      }
    },

    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: CATEGORY_PREFERENCES_KEY });
    },
  });
}

// ============================================================================
// Story 40.2: Quiet Hours Hooks
// ============================================================================

/**
 * Hook to fetch quiet hours configuration.
 */
export function useQuietHours() {
  return useQuery({
    queryKey: QUIET_HOURS_KEY,
    queryFn: () => getQuietHours(),
  });
}

/**
 * Hook to update quiet hours with optimistic updates and error rollback.
 */
export function useUpdateQuietHours() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: UpdateQuietHoursRequest) => updateQuietHours(request),

    onMutate: async (request) => {
      await queryClient.cancelQueries({ queryKey: QUIET_HOURS_KEY });

      const previousData = queryClient.getQueryData<QuietHoursResponse>(QUIET_HOURS_KEY);

      if (previousData) {
        queryClient.setQueryData<QuietHoursResponse>(QUIET_HOURS_KEY, {
          quietHours: {
            ...previousData.quietHours,
            ...request,
            updatedAt: new Date().toISOString(),
          },
        });
      }

      return { previousData };
    },

    onError: (_err, _variables, context) => {
      if (context?.previousData) {
        queryClient.setQueryData(QUIET_HOURS_KEY, context.previousData);
      }
    },

    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: QUIET_HOURS_KEY });
    },
  });
}

// ============================================================================
// Story 40.3: Digest Preferences Hooks
// ============================================================================

/**
 * Hook to fetch digest configuration.
 */
export function useDigestPreferences() {
  return useQuery({
    queryKey: DIGEST_PREFERENCES_KEY,
    queryFn: () => getDigestPreferences(),
  });
}

/**
 * Hook to update digest preferences with optimistic updates and error rollback.
 */
export function useUpdateDigestPreferences() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: UpdateDigestRequest) => updateDigestPreferences(request),

    onMutate: async (request) => {
      await queryClient.cancelQueries({ queryKey: DIGEST_PREFERENCES_KEY });

      const previousData = queryClient.getQueryData<DigestResponse>(DIGEST_PREFERENCES_KEY);

      if (previousData) {
        queryClient.setQueryData<DigestResponse>(DIGEST_PREFERENCES_KEY, {
          digest: {
            ...previousData.digest,
            ...request,
            updatedAt: new Date().toISOString(),
          },
        });
      }

      return { previousData };
    },

    onError: (_err, _variables, context) => {
      if (context?.previousData) {
        queryClient.setQueryData(DIGEST_PREFERENCES_KEY, context.previousData);
      }
    },

    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: DIGEST_PREFERENCES_KEY });
    },
  });
}

// ============================================================================
// Story 40.4: Grouping Preferences Hooks
// ============================================================================

/**
 * Hook to fetch notification grouping configuration.
 */
export function useGroupingPreferences() {
  return useQuery({
    queryKey: GROUPING_PREFERENCES_KEY,
    queryFn: () => getGroupingPreferences(),
  });
}

/**
 * Hook to update grouping preferences with optimistic updates and error rollback.
 */
export function useUpdateGroupingPreferences() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: UpdateGroupingRequest) => updateGroupingPreferences(request),

    onMutate: async (request) => {
      await queryClient.cancelQueries({ queryKey: GROUPING_PREFERENCES_KEY });

      const previousData = queryClient.getQueryData<GroupingResponse>(GROUPING_PREFERENCES_KEY);

      if (previousData) {
        queryClient.setQueryData<GroupingResponse>(GROUPING_PREFERENCES_KEY, {
          grouping: {
            ...previousData.grouping,
            ...request,
            updatedAt: new Date().toISOString(),
          },
        });
      }

      return { previousData };
    },

    onError: (_err, _variables, context) => {
      if (context?.previousData) {
        queryClient.setQueryData(GROUPING_PREFERENCES_KEY, context.previousData);
      }
    },

    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: GROUPING_PREFERENCES_KEY });
    },
  });
}

// ============================================================================
// Combined Hook
// ============================================================================

/**
 * Hook to fetch all advanced notification preferences at once.
 */
export function useAdvancedPreferences() {
  return useQuery({
    queryKey: ADVANCED_PREFERENCES_KEY,
    queryFn: () => getAdvancedPreferences(),
  });
}

/**
 * Hook to invalidate all advanced notification preference queries.
 */
export function useInvalidateAdvancedPreferences() {
  const queryClient = useQueryClient();

  return () => {
    queryClient.invalidateQueries({ queryKey: ['notification-preferences'] });
  };
}
