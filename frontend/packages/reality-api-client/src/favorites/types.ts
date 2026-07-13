/**
 * Reality Portal Favorites & Saved Searches Types
 *
 * TypeScript types for favorites and saved searches API (Epic 44).
 */

import type { ListingFilters, ListingSummary } from '../listings/types';

// Favorite Listing
export interface FavoriteListing {
  id: string;
  listingId: string;
  listing: ListingSummary;
  addedAt: string;
  notes?: string;
}

// Saved Search
export interface SavedSearch {
  id: string;
  name: string;
  filters: ListingFilters;
  alertsEnabled: boolean;
  alertFrequency?: 'daily' | 'weekly' | 'instant';
  newListingsCount?: number;
  lastAlertAt?: string;
  createdAt: string;
  updatedAt: string;
}

// Create Saved Search Request
export interface CreateSavedSearchRequest {
  name: string;
  filters: ListingFilters;
  alertsEnabled?: boolean;
  alertFrequency?: 'daily' | 'weekly' | 'instant';
}

// Update Saved Search Request
export interface UpdateSavedSearchRequest {
  name?: string;
  filters?: ListingFilters;
  alertsEnabled?: boolean;
  alertFrequency?: 'daily' | 'weekly' | 'instant';
}

// Paginated Response
export interface PaginatedFavorites {
  data: FavoriteListing[];
  total: number;
  page: number;
  pageSize: number;
  totalPages: number;
}

// ============================================
// Favorite price / status alerts (Story 84.3)
// ============================================

/**
 * A single queued favorite alert, as delivered by
 * `GET /api/v1/favorites/alerts`.
 *
 * Field names mirror the reality-server `FavoriteAlert` DTO verbatim
 * (snake_case on the wire — that DTO has no `rename_all = "camelCase"`),
 * so this interface is the runtime-accurate shape of the response.
 */
export interface FavoriteAlert {
  id: string;
  favorite_id: string;
  listing_id: string;
  title: string;
  /** `price_change` (price drop/increase) or `back_on_market`. */
  alert_type: string;
  old_price?: number | null;
  new_price?: number | null;
  currency?: string | null;
  change_percentage?: number | null;
  previous_status?: string | null;
  new_status?: string | null;
  /** `pending` (unread) or `sent` (read). */
  status: string;
  created_at: string;
  processed_at?: string | null;
}

/** Response of `GET /api/v1/favorites/alerts`. */
export interface FavoriteAlertsResponse {
  alerts: FavoriteAlert[];
  unread_count: number;
  limit: number;
  offset: number;
}

/** Response of `POST /api/v1/favorites/alerts/read-all`. */
export interface MarkAllFavoriteAlertsReadResponse {
  marked_read: number;
}
