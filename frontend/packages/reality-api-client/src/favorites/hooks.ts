/**
 * Reality Portal Favorites & Saved Searches Hooks
 *
 * React Query hooks for favorites and saved searches API (Epic 44).
 */

'use client';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { getApiBase } from '../config';
import type { ListingSummary } from '../listings/types';
import type {
  CreateSavedSearchRequest,
  FavoriteAlertsResponse,
  FavoriteAlertsWireResponse,
  FavoriteListing,
  FavoritesWireResponse,
  MarkAllFavoriteAlertsReadResponse,
  PaginatedFavorites,
  PortalFavoriteWire,
  SavedSearch,
  UpdateFavoriteRequest,
  UpdateSavedSearchRequest,
} from './types';

// Favorites Hooks

/**
 * Map a wire favorite (reality-server `PortalFavoriteWithListing` — snake_case,
 * flattened listing fields, Decimal prices as strings) into the client
 * {@link FavoriteListing} view-model the UI consumes. The list DTO carries only
 * a subset of `ListingSummary`; fields it doesn't provide (`slug`, `area`, …)
 * are defaulted so the card can render from what the favorites endpoint gives.
 */
function mapWireFavorite(w: PortalFavoriteWire): FavoriteListing {
  const listing: ListingSummary = {
    id: w.listing_id,
    title: w.title,
    // The favorites list DTO carries no slug; fall back to the listing id so
    // the card link is at least a stable identifier rather than `undefined`.
    slug: w.listing_id,
    propertyType: w.property_type as ListingSummary['propertyType'],
    transactionType: w.transaction_type as ListingSummary['transactionType'],
    status: w.status as ListingSummary['status'],
    price: toNumeric(w.current_price) ?? 0,
    currency: w.currency,
    // Not carried by the favorites list DTO.
    area: 0,
    address: { city: w.city, country: '' },
    primaryPhoto: w.photo_url
      ? {
          id: w.listing_id,
          url: w.photo_url,
          thumbnailUrl: w.photo_url,
          isPrimary: true,
          order: 0,
        }
      : undefined,
    isFeatured: false,
    createdAt: w.created_at,
    updatedAt: w.created_at,
  };

  return {
    id: w.id,
    listingId: w.listing_id,
    listing,
    addedAt: w.created_at,
    price_alert_enabled: w.price_alert_enabled,
  };
}

export function useFavorites(page = 1, pageSize = 20) {
  return useQuery({
    queryKey: ['favorites', page, pageSize],
    queryFn: async (): Promise<PaginatedFavorites> => {
      const response = await fetch(`${getApiBase()}/api/v1/favorites`, {
        credentials: 'include',
      });
      if (!response.ok) throw new Error('Failed to fetch favorites');

      // reality-server answers with the unpaginated `{ favorites: [...] }`
      // envelope (`FavoritesResponse`), each row snake_case with flattened
      // listing fields — NOT a `{ data: [{ listingId, listing }] }` shape.
      // Normalize to the client view-model, then paginate client-side.
      const wire: FavoritesWireResponse = await response.json();
      const all = (wire.favorites ?? []).map(mapWireFavorite);
      const total = all.length;
      const totalPages = Math.max(1, Math.ceil(total / pageSize));
      const start = (page - 1) * pageSize;
      const data = all.slice(start, start + pageSize);

      return { data, total, page, pageSize, totalPages };
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

/**
 * Update a favorite's settings — notes and/or the price-alert opt-in
 * (`price_alert_enabled`). Completes AC-5 of story 84.3: this is how a user
 * subscribes / unsubscribes a favorited listing from price-change alerts.
 *
 * Consumes `PATCH /api/v1/favorites/{listingId}`. The body is partial-merged
 * server-side, so passing only `price_alert_enabled` leaves notes untouched.
 *
 * Optimistically flips `price_alert_enabled` on the matching row in every
 * cached `['favorites', …]` page so the toggle reflects the user's intent
 * immediately, rolling the snapshot back if the PATCH fails. This removes the
 * dependency on the page's `?? true` fallback masking a transiently-absent
 * field after refetch (see #2365).
 */
export function useUpdateFavorite() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({
      listingId,
      data,
    }: {
      listingId: string;
      data: UpdateFavoriteRequest;
    }): Promise<void> => {
      const response = await fetch(`${getApiBase()}/api/v1/favorites/${listingId}`, {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(data),
        credentials: 'include',
      });
      if (!response.ok) throw new Error('Failed to update favorite');
    },
    onMutate: async ({ listingId, data }) => {
      // Only the price-alert toggle has a visible cached surface to update
      // optimistically; a notes-only edit has nothing to reflect here.
      if (data.price_alert_enabled == null) return { previous: [] };

      await queryClient.cancelQueries({ queryKey: ['favorites'] });

      // Snapshot every matching `['favorites', page, pageSize]` cache entry so
      // we can roll back on error.
      const previous = queryClient.getQueriesData<PaginatedFavorites>({
        queryKey: ['favorites'],
      });

      queryClient.setQueriesData<PaginatedFavorites>({ queryKey: ['favorites'] }, (old) => {
        if (!old) return old;
        return {
          ...old,
          data: old.data.map((favorite) =>
            favorite.listingId === listingId
              ? { ...favorite, price_alert_enabled: data.price_alert_enabled ?? undefined }
              : favorite
          ),
        };
      });

      return { previous };
    },
    onError: (_error, _variables, context) => {
      // Restore the pre-mutation snapshot so a failed toggle doesn't leave the
      // checkbox showing an un-persisted state.
      for (const [queryKey, data] of context?.previous ?? []) {
        queryClient.setQueryData(queryKey, data);
      }
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: ['favorites'] });
      queryClient.invalidateQueries({ queryKey: FAVORITE_ALERTS_KEY });
    },
  });
}

// Favorite Price-Alert Hooks (Story 84.3)

const FAVORITE_ALERTS_KEY = ['favorite-alerts'] as const;

/**
 * Coerce a wire Decimal field (delivered by reality-server as a JSON string via
 * `rust_decimal` `serde-str`) into a `number`, preserving `null` / `undefined`.
 * Without this, `PriceAlerts` would compare prices lexicographically
 * (`'180000' < '95000'` is `true`), mislabelling drops as increases.
 */
function toNumeric(value: string | number | null | undefined): number | null {
  if (value == null) return null;
  const n = Number(value);
  return Number.isNaN(n) ? null : n;
}

/**
 * Fetch the authenticated user's favorite price / status alerts (newest first).
 *
 * Consumes `GET /api/v1/favorites/alerts` — the price-tracking feed populated
 * by reality-server's `FavoriteAlertWorker`. `unread_count` reflects alerts
 * still in `pending` status.
 *
 * The backend serializes `old_price` / `new_price` / `change_percentage`
 * (`rust_decimal::Decimal`) as JSON **strings**; this hook normalizes them to
 * numbers at the boundary so consumers get a numerically-correct shape.
 *
 * NOTE: this fetches a single page of up to `limit` (default 100) alerts; the
 * API supports `offset` paging but no "load more" affordance is wired yet, so
 * alerts beyond the first page are currently unreachable from the UI.
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

      const raw: FavoriteAlertsWireResponse = await response.json();
      return {
        ...raw,
        alerts: raw.alerts.map((a) => ({
          ...a,
          old_price: toNumeric(a.old_price),
          new_price: toNumeric(a.new_price),
          change_percentage: toNumeric(a.change_percentage),
        })),
      };
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
