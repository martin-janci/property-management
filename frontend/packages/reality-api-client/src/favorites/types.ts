/**
 * Reality Portal Favorites & Saved Searches Types
 *
 * TypeScript types for favorites and saved searches API (Epic 44).
 */

import type { ListingFilters, ListingSummary } from '../listings/types';

/**
 * A favorite row exactly as it arrives **on the wire** from
 * `GET /api/v1/favorites`.
 *
 * Field names mirror the reality-server `PortalFavoriteWithListing` DTO
 * verbatim (`backend/crates/db/src/models/reality_portal.rs`): snake_case (that
 * struct has no `#[serde(rename_all = "camelCase")]`), with the listing fields
 * **flattened** onto the favorite — there is no nested `listing` object. The
 * price fields are `rust_decimal::Decimal`, serialized as JSON **strings**
 * (e.g. `"180000.00"`), so they arrive as strings, not numbers.
 * `useFavorites` normalizes this raw shape into {@link FavoriteListing} at the
 * client boundary — do not consume this interface directly in components.
 */
export interface PortalFavoriteWire {
  id: string;
  listing_id: string;
  title: string;
  current_price: string | number;
  original_price?: string | number | null;
  currency: string;
  city: string;
  property_type: string;
  transaction_type: string;
  photo_url?: string | null;
  status: string;
  price_changed: boolean;
  price_change_percentage?: string | number | null;
  price_alert_enabled: boolean;
  created_at: string;
}

/**
 * Raw response of `GET /api/v1/favorites` — the reality-server
 * `FavoritesResponse { favorites: Vec<PortalFavoriteWithListing> }` envelope.
 * Unpaginated: the route returns every favorite in one `favorites` array.
 */
export interface FavoritesWireResponse {
  favorites: PortalFavoriteWire[];
}

/**
 * A favorite normalized into the client view-model that the favorites UI
 * consumes: the flattened wire listing fields (see {@link PortalFavoriteWire})
 * are lifted into a nested {@link ListingSummary} and the Decimal price strings
 * are coerced to numbers. This is the shape components read.
 */
export interface FavoriteListing {
  id: string;
  listingId: string;
  listing: ListingSummary;
  addedAt: string;
  notes?: string;
  /**
   * Whether this favorite is subscribed to price-change alerts (AC-5, story
   * 84.3). Backend default is `true`; toggled via `useUpdateFavorite`.
   */
  price_alert_enabled?: boolean;
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

// Paginated Response.
// NOTE: `GET /api/v1/favorites` is unpaginated on the wire (it returns the full
// `{ favorites: [...] }` array); `useFavorites` derives this page envelope
// client-side by slicing that array.
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
 * A single queued favorite alert exactly as it arrives **on the wire** from
 * `GET /api/v1/favorites/alerts`.
 *
 * Field names mirror the reality-server `FavoriteAlert` DTO verbatim
 * (snake_case — that DTO has no `rename_all = "camelCase"`). Crucially, the
 * price / percentage fields are backed by `rust_decimal::Decimal`, which the
 * backend serializes **as a JSON string** (`rust_decimal` `serde-str`
 * feature). They therefore arrive as strings like `"180000.00"` / `"-47.2"`,
 * NOT numbers. `useFavoriteAlerts` normalizes this raw shape into
 * {@link FavoriteAlert} (numbers) once at the client boundary — do not consume
 * this interface directly in components.
 */
export interface FavoriteAlertWire {
  id: string;
  favorite_id: string;
  listing_id: string;
  title: string;
  /** `price_change` (price drop/increase) or `back_on_market`. */
  alert_type: string;
  old_price?: string | null;
  new_price?: string | null;
  currency?: string | null;
  change_percentage?: string | null;
  previous_status?: string | null;
  new_status?: string | null;
  /** `pending` (unread) or `sent` (read). */
  status: string;
  created_at: string;
  processed_at?: string | null;
}

/** Raw response of `GET /api/v1/favorites/alerts` (Decimal fields as strings). */
export interface FavoriteAlertsWireResponse {
  alerts: FavoriteAlertWire[];
  unread_count: number;
  limit: number;
  offset: number;
}

/**
 * A favorite alert with its Decimal price / percentage fields normalized to
 * `number` (see {@link FavoriteAlertWire} for why the wire delivers strings).
 * This is the shape components consume, so `<` / `>` price comparisons are
 * numeric — never lexicographic string comparisons.
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

/** Response of `GET /api/v1/favorites/alerts` (normalized numeric prices). */
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

/**
 * Body of `PATCH /api/v1/favorites/{listing_id}` — mirrors the reality-server
 * `UpdatePortalFavorite` DTO. Both fields are optional and `COALESCE`-merged
 * server-side, so sending only `price_alert_enabled` leaves notes untouched.
 * This is how AC-5 (story 84.3) subscribes / unsubscribes a favorite from
 * price-change alerts.
 */
export interface UpdateFavoriteRequest {
  notes?: string | null;
  price_alert_enabled?: boolean | null;
}
