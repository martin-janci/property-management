package three.two.bit.ppt.reality.favorites

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import three.two.bit.ppt.reality.listing.ListingSummary

/**
 * Favorites models for Reality Portal mobile app.
 *
 * Epic 48 - Story 48.3: Portal Mobile Favorites
 */

/** Favorite listing entry. */
@Serializable
data class FavoriteEntry(
    val id: String,
    @SerialName("listing_id") val listingId: String,
    @SerialName("user_id") val userId: String,
    @SerialName("created_at") val createdAt: String,
    val listing: ListingSummary? = null,
)

/** User favorites response. */
@Serializable data class FavoritesResponse(val favorites: List<FavoriteEntry>, val total: Int)

/** Add favorite request. */
@Serializable data class AddFavoriteRequest(@SerialName("listing_id") val listingId: String)

/** Add favorite response. */
@Serializable
data class AddFavoriteResponse(
    val id: String,
    @SerialName("listing_id") val listingId: String,
    @SerialName("created_at") val createdAt: String,
)

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
