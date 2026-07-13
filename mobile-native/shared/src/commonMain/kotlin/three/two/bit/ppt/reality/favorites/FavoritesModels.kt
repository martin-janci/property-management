package three.two.bit.ppt.reality.favorites

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import three.two.bit.ppt.reality.listing.ListingSummary

/**
 * Favorites models for Reality Portal mobile app.
 *
 * Epic 48 - Story 48.3: Portal Mobile Favorites
 *
 * The reality-server `/api/v1/favorites` endpoint returns `PortalFavoriteWithListing` rows which
 * embed a flat summary of the listing (title, current_price, city, photo_url, etc.) rather than a
 * nested `ListingSummary` object. The fields below mirror that shape exactly so the Compose UI can
 * display favorites cards without a second listing-detail request.
 *
 * Gap-82-7: wired flat server fields + fixed isFavorite/addFavorite endpoint contracts.
 */

/**
 * Favorite entry as returned by `GET /api/v1/favorites`.
 *
 * Maps `PortalFavoriteWithListing` from the reality-server:
 * - `id`, `listing_id`, `created_at` — standard favorite metadata.
 * - `title`, `current_price`, `currency`, `city`, `property_type`, `transaction_type`, `photo_url`,
 *   `status`, `price_changed`, `price_change_percentage`, `price_alert_enabled` — flat listing
 *   summary embedded by the server.
 * - `listing` — always `null` for this endpoint; kept for backward compat with any code that still
 *   checks `entry.listing`.
 */
@Serializable
data class FavoriteEntry(
    val id: String,
    @SerialName("listing_id") val listingId: String,
    // `user_id` is not in PortalFavoriteWithListing; treat as optional so
    // the decoder doesn't fail on the real server payload.
    @SerialName("user_id") val userId: String? = null,
    @SerialName("created_at") val createdAt: String,
    // Flat listing summary (reality-server embeds these directly).
    val title: String? = null,
    @SerialName("current_price") val currentPrice: Long? = null,
    val currency: String? = null,
    val city: String? = null,
    @SerialName("property_type") val propertyType: String? = null,
    @SerialName("transaction_type") val transactionType: String? = null,
    @SerialName("photo_url") val photoUrl: String? = null,
    val status: String? = null,
    @SerialName("price_changed") val priceChanged: Boolean = false,
    @SerialName("price_change_percentage") val priceChangePercentage: Double? = null,
    @SerialName("price_alert_enabled") val priceAlertEnabled: Boolean = false,
    // Retained for any callers that still check the nested shape; always null
    // from the real server but may be set in unit tests using mock data.
    val listing: ListingSummary? = null,
)

/** User favorites response. */
@Serializable data class FavoritesResponse(val favorites: List<FavoriteEntry>, val total: Long = 0)

/**
 * Response body from `GET /api/v1/favorites/{listing_id}/check`.
 *
 * The server always returns HTTP 200 with this payload — it never returns 404 for an un-favorited
 * listing. The KMP client must inspect `isFavorited` rather than treating any 2xx as `true`.
 */
@Serializable data class CheckFavoriteResponse(@SerialName("is_favorited") val isFavorited: Boolean)

/**
 * Empty body for `POST /api/v1/favorites/{listing_id}`.
 *
 * The server's `AddFavorite` schema is `{notes?: String}`; we omit the optional `notes`. Encoding
 * via a typed `@Serializable data object` routes through Ktor's `ContentNegotiation` pipeline (same
 * as every other call), instead of bypassing it with a raw `setBody("{}")` string literal.
 */
@Serializable data object AddFavoriteBody

/**
 * Add favorite response from `POST /api/v1/favorites/{listing_id}`.
 *
 * Matches `PortalFavorite` returned by the server on 201.
 */
@Serializable
data class AddFavoriteResponse(
    val id: String,
    @SerialName("listing_id") val listingId: String,
    @SerialName("created_at") val createdAt: String,
)

// ============================================
// Price-tracking alerts (favorites) — gap-84-3
// ============================================

/**
 * A single favorite alert as returned by `GET /api/v1/favorites/alerts`.
 *
 * Mirrors the reality-server `FavoriteAlert` DTO (see
 * `backend/crates/db/src/models/reality_portal.rs`). These are the in-app
 * delivery view of the price-tracking pipeline: the `FavoriteAlertWorker`
 * enqueues a row whenever a favorited listing's price (or status) changes, and
 * this feed is how the mobile Price-Tracking surface retrieves them.
 *
 * Wire notes:
 * - `alert_type` is `"price_change"` or `"status_change"`. Use [isPriceChange].
 * - Monetary fields (`old_price` / `new_price`) are whole-currency numbers on
 *   the wire — mapped to `Long?` to match the rest of the portal models
 *   ([FavoriteEntry.currentPrice], `ListingSummary.price`). They are `null` for
 *   status-only alerts.
 * - `change_percentage` is a signed fractional number (negative = price drop),
 *   mapped to `Double?` to match [FavoriteEntry.priceChangePercentage].
 * - `status` is the delivery state: `"pending"` (unread) | `"sent"` (read) |
 *   `"failed"`. Use [isUnread].
 */
@Serializable
data class FavoriteAlert(
    val id: String,
    @SerialName("favorite_id") val favoriteId: String,
    @SerialName("listing_id") val listingId: String,
    val title: String,
    @SerialName("alert_type") val alertType: String,
    @SerialName("old_price") val oldPrice: Long? = null,
    @SerialName("new_price") val newPrice: Long? = null,
    val currency: String? = null,
    @SerialName("change_percentage") val changePercentage: Double? = null,
    @SerialName("previous_status") val previousStatus: String? = null,
    @SerialName("new_status") val newStatus: String? = null,
    val status: String,
    @SerialName("created_at") val createdAt: String,
    @SerialName("processed_at") val processedAt: String? = null,
) {
    /** True when this alert reports a price movement (vs a listing-status change). */
    val isPriceChange: Boolean
        get() = alertType == "price_change"

    /** True while the alert is still pending in-app delivery (i.e. unread). */
    val isUnread: Boolean
        get() = status == "pending"

    /** True when the price moved down (a drop) — only meaningful for price alerts. */
    val isPriceDrop: Boolean
        get() = isPriceChange && (changePercentage ?: 0.0) < 0.0
}

/**
 * Response body for `GET /api/v1/favorites/alerts`.
 *
 * Mirrors the reality-server `FavoriteAlertsResponse`: the newest-first page of
 * alerts plus the total unread count (independent of the page window) and the
 * echoed `limit` / `offset`.
 */
@Serializable
data class FavoriteAlertsResponse(
    val alerts: List<FavoriteAlert> = emptyList(),
    @SerialName("unread_count") val unreadCount: Long = 0,
    val limit: Long = 0,
    val offset: Long = 0,
)

/**
 * Response body for `POST /api/v1/favorites/alerts/read-all`.
 *
 * Mirrors the reality-server `MarkAllFavoriteAlertsReadResponse`.
 */
@Serializable
data class MarkAllFavoriteAlertsReadResponse(@SerialName("marked_read") val markedRead: Long = 0)

/** Saved search. */
@Serializable
data class SavedSearch(
    val id: String,
    val name: String,
    val query: String? = null,
    val filters: SavedSearchFilters? = null,
    @SerialName("alert_enabled") val alertEnabled: Boolean = false,
    @SerialName("created_at") val createdAt: String,
    @SerialName("last_notified_at") val lastNotifiedAt: String? = null,
    @SerialName("new_count") val newCount: Int = 0,
)

/** Saved search filters. */
@Serializable
data class SavedSearchFilters(
    val type: String? = null,
    val category: String? = null,
    val city: String? = null,
    @SerialName("min_price") val minPrice: Long? = null,
    @SerialName("max_price") val maxPrice: Long? = null,
    @SerialName("min_rooms") val minRooms: Int? = null,
)

/** Saved searches response. */
@Serializable data class SavedSearchesResponse(val searches: List<SavedSearch>, val total: Int)

/** Create saved search request. */
@Serializable
data class CreateSavedSearchRequest(
    val name: String,
    val query: String? = null,
    val filters: SavedSearchFilters? = null,
    @SerialName("alert_enabled") val alertEnabled: Boolean = false,
)

/** Update saved search request. */
@Serializable
data class UpdateSavedSearchRequest(
    val name: String? = null,
    @SerialName("alert_enabled") val alertEnabled: Boolean? = null,
)
