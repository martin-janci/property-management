package three.two.bit.ppt.reality.realtor

import kotlinx.serialization.Serializable
import three.two.bit.ppt.reality.api.DecimalAsLongSerializer

/**
 * Realtor "my listings" + per-listing analytics wire models.
 *
 * These mirror reality-server's portal-listings surface
 * (`backend/servers/reality-server/src/routes/portal_listings.rs`), which serializes **camelCase**
 * (`#[serde(rename_all = "camelCase")]`). Monetary fields are `rust_decimal` values emitted as JSON
 * strings (serde-str) so they decode through [DecimalAsLongSerializer] into whole-currency units —
 * the same convention already used by [ListingSummary]/[ListingDetail].
 *
 * Story 33.4 — closes the mobile MyListings / ListingAnalytics stubs now that the backend LIST +
 * analytics endpoints exist.
 */

/** One of the signed-in portal user's own listings (`GET /api/v1/my/listings`). */
@Serializable
data class PortalListing(
    val id: String,
    val title: String,
    val description: String? = null,
    val propertyType: String = "",
    val transactionType: String = "",
    @Serializable(with = DecimalAsLongSerializer::class) val price: Long = 0,
    val currency: String = "EUR",
    val street: String = "",
    val city: String = "",
    val postalCode: String = "",
    val country: String = "",
    @Serializable(with = DecimalAsLongSerializer::class) val sizeSqm: Long? = null,
    val rooms: Int? = null,
    val floor: Int? = null,
    val totalFloors: Int? = null,
    val status: String = "draft",
    val isNegotiable: Boolean = false,
    val isPublished: Boolean = false,
    val slug: String? = null,
    val createdAt: String? = null,
    val updatedAt: String? = null,
)

/** Paginated response for `GET /api/v1/my/listings`. */
@Serializable
data class MyListingsResponse(val listings: List<PortalListing> = emptyList(), val total: Long = 0)

/** One day's analytics counters (`GET /api/v1/my/listings/{id}/analytics`). */
@Serializable
data class DailyListingAnalytics(
    val date: String,
    val views: Int = 0,
    val uniqueViews: Int = 0,
    val favoritesAdded: Int = 0,
    val favoritesRemoved: Int = 0,
    val inquiries: Int = 0,
    val phoneClicks: Int = 0,
    val shareClicks: Int = 0,
    val sourceWebsite: Int = 0,
    val sourceMobile: Int = 0,
    val sourceSearch: Int = 0,
    val sourceDirect: Int = 0,
)

/** Analytics summary + daily series for one listing (`GET /api/v1/my/listings/{id}/analytics`). */
@Serializable
data class ListingAnalytics(
    val listingId: String,
    val totalViews: Long = 0,
    val totalInquiries: Long = 0,
    val totalFavorites: Long = 0,
    val daysOnMarket: Int = 0,
    val dailyAnalytics: List<DailyListingAnalytics> = emptyList(),
)

/**
 * Realtor-wide analytics rolled up across all of the caller's listings.
 *
 * Derived client-side (the backend only exposes per-listing analytics); the realtor dashboard has
 * no listing id, so [PortalListingsRepository.getPortfolioAnalytics] fans out one analytics call
 * per listing and folds the daily series together into portfolio totals + trends.
 */
data class PortfolioAnalytics(
    val totalListings: Int = 0,
    val activeListings: Int = 0,
    val totalViews: Long = 0,
    val totalInquiries: Long = 0,
    val totalFavorites: Long = 0,
    val viewsTrend: List<Int> = emptyList(),
    val inquiriesTrend: List<Int> = emptyList(),
)
