/**
 * Reality Portal Favorites & Saved Searches Hooks
 *
 * React Query hooks for favorites and saved searches API (Epic 44).
 */

'use client';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { getApiBase } from '../config';
import type {
  CreateSavedSearchRequest,
  FavoriteAlertsResponse,
  MarkAllFavoriteAlertsReadResponse,
  PaginatedFavorites,
  SavedSearch,
  UpdateSavedSearchRequest,
} from './types';

// Favorites Hooks

export function useFavorites(page = 1, pageSize = 20) {
  return useQuery({
    queryKey: ['favorites', page, pageSize],
    queryFn: async (): Promise<PaginatedFavorites> => {
      const params = new URLSearchParams();
      params.set('page', String(page));
      params.set('pageSize', String(pageSize));

      const response = await fetch(`${getApiBase()}/api/v1/favorites?${params}`, {
        credentials: 'include',
      });
      if (!response.ok) throw new Error('Failed to fetch favorites');
      return response.json();
    },
  });
}

export function useFavoriteIds() {
  return useQuery({
    queryKey: ['favorite-ids'],
    queryFn: async (): Promise<string[]> => {
      const response = await fetch(`${getApiBase()}/api/v1/favorites/ids`, {
        credentials: 'include',
      });
      if (!response.ok) throw new Error('Failed to fetch favorite IDs');
      return response.json();
    },
    staleTime: 60 * 1000, // 1 minute
  });
}

export function useAddFavorite() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({
      listingId,
      notes,
    }: {
      listingId: string;
      notes?: string;
    }): Promise<void> => {
      const response = await fetch(`${getApiBase()}/api/v1/favorites`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ listingId, notes }),
        credentials: 'include',
      });
      if (!response.ok) throw new Error('Failed to add favorite');
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['favorites'] });
      queryClient.invalidateQueries({ queryKey: ['favorite-ids'] });
      queryClient.invalidateQueries({ queryKey: ['listings'] });
    },
  });
}

export function useRemoveFavorite() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (listingId: string): Promise<void> => {
      const response = await fetch(`${getApiBase()}/api/v1/favorites/${listingId}`, {
        method: 'DELETE',
        credentials: 'include',
      });
      if (!response.ok) throw new Error('Failed to remove favorite');
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['favorites'] });
      queryClient.invalidateQueries({ queryKey: ['favorite-ids'] });
      queryClient.invalidateQueries({ queryKey: ['listings'] });
    },
  });
}

// Favorite Price-Alert Hooks (Story 84.3)

const FAVORITE_ALERTS_KEY = ['favorite-alerts'] as const;

/**
 * Fetch the authenticated user's favorite price / status alerts (newest first).
 *
 * Consumes `GET /api/v1/favorites/alerts` — the price-tracking feed populated
 * by reality-server's `FavoriteAlertWorker`. `unread_count` reflects alerts
 * still in `pending` status.
 */
export function useFavoriteAlerts(limit = 100, offset = 0) {
  return useQuery({
    queryKey: [...FAVORITE_ALERTS_KEY, limit, offset],
    queryFn: async (): Promise<FavoriteAlertsResponse> => {
      const params = new URLSearchParams();
      params.set('limit', String(limit));
      params.set('offset', String(offset));

      const response = await fetch(`${getApiBase()}/api/v1/favorites/alerts?${params}`, {
        credentials: 'include',
      });
      if (!response.ok) throw new Error('Failed to fetch favorite alerts');
      return response.json();
    },
    staleTime: 30 * 1000,
  });
}

/** Mark a single favorite alert as read (idempotent server-side). */
export function useMarkFavoriteAlertRead() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (alertId: string): Promise<void> => {
      const response = await fetch(`${getApiBase()}/api/v1/favorites/alerts/${alertId}/read`, {
        method: 'POST',
        credentials: 'include',
      });
      if (!response.ok) throw new Error('Failed to mark favorite alert read');
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: FAVORITE_ALERTS_KEY });
    },
  });
}

/** Mark every pending favorite alert as read. */
export function useMarkAllFavoriteAlertsRead() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (): Promise<MarkAllFavoriteAlertsReadResponse> => {
      const response = await fetch(`${getApiBase()}/api/v1/favorites/alerts/read-all`, {
        method: 'POST',
        credentials: 'include',
      });
      if (!response.ok) throw new Error('Failed to mark all favorite alerts read');
      return response.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: FAVORITE_ALERTS_KEY });
    },
  });
}

// Saved Searches Hooks

export function useSavedSearches() {
  return useQuery({
    queryKey: ['saved-searches'],
    queryFn: async (): Promise<SavedSearch[]> => {
      const response = await fetch(`${getApiBase()}/api/v1/saved-searches`, {
        credentials: 'include',
      });
      if (!response.ok) throw new Error('Failed to fetch saved searches');
      return response.json();
    },
  });
}

export function useSavedSearch(id: string) {
  return useQuery({
    queryKey: ['saved-search', id],
    queryFn: async (): Promise<SavedSearch> => {
      const response = await fetch(`${getApiBase()}/api/v1/saved-searches/${id}`, {
        credentials: 'include',
      });
      if (!response.ok) throw new Error('Failed to fetch saved search');
      return response.json();
    },
    enabled: !!id,
  });
}

export function useCreateSavedSearch() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (data: CreateSavedSearchRequest): Promise<SavedSearch> => {
      const response = await fetch(`${getApiBase()}/api/v1/saved-searches`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(data),
        credentials: 'include',
      });
      if (!response.ok) throw new Error('Failed to create saved search');
      return response.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['saved-searches'] });
    },
  });
}

export function useUpdateSavedSearch() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({
      id,
      data,
    }: {
      id: string;
      data: UpdateSavedSearchRequest;
    }): Promise<SavedSearch> => {
      const response = await fetch(`${getApiBase()}/api/v1/saved-searches/${id}`, {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(data),
        credentials: 'include',
      });
      if (!response.ok) throw new Error('Failed to update saved search');
      return response.json();
    },
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({ queryKey: ['saved-searches'] });
      queryClient.invalidateQueries({ queryKey: ['saved-search', variables.id] });
    },
  });
}

export function useDeleteSavedSearch() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (id: string): Promise<void> => {
      const response = await fetch(`${getApiBase()}/api/v1/saved-searches/${id}`, {
        method: 'DELETE',
        credentials: 'include',
      });
      if (!response.ok) throw new Error('Failed to delete saved search');
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['saved-searches'] });
    },
  });
}

export function useToggleSearchAlert() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({ id, enabled }: { id: string; enabled: boolean }): Promise<void> => {
      const response = await fetch(`${getApiBase()}/api/v1/saved-searches/${id}/alerts`, {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ enabled }),
        credentials: 'include',
      });
      if (!response.ok) throw new Error('Failed to toggle search alert');
    },
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({ queryKey: ['saved-searches'] });
      queryClient.invalidateQueries({ queryKey: ['saved-search', variables.id] });
    },
  });
}
