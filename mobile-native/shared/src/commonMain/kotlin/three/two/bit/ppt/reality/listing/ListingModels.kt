package three.two.bit.ppt.reality.listing

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

/**
 * Listing models for Reality Portal mobile app.
 *
 * Epic 48 - Story 48.1: Portal Mobile Search
 */

/** Property listing type. */
@Serializable
enum class ListingType {
    @SerialName("sale") SALE,
    @SerialName("rent") RENT,
}

/** Property category. */
@Serializable
enum class PropertyCategory {
    @SerialName("apartment") APARTMENT,
    @SerialName("house") HOUSE,
    @SerialName("land") LAND,
    @SerialName("commercial") COMMERCIAL,
    @SerialName("garage") GARAGE,
    @SerialName("office") OFFICE,
    @SerialName("warehouse") WAREHOUSE,
}

/** Listing status. */
@Serializable
enum class ListingStatus {
    @SerialName("active") ACTIVE,
    @SerialName("pending") PENDING,
    @SerialName("sold") SOLD,
    @SerialName("rented") RENTED,
    @SerialName("withdrawn") WITHDRAWN,
}

/** Geographic coordinates. */
@Serializable data class Coordinates(val latitude: Double, val longitude: Double)

/** Address information. */
@Serializable
data class Address(
    val street: String? = null,
    val city: String,
    val district: String? = null,
    val region: String? = null,
    @SerialName("postal_code") val postalCode: String? = null,
    val country: String,
    val coordinates: Coordinates? = null,
)

/** Listing image. */
@Serializable
data class ListingImage(
    val id: String,
    val url: String,
    @SerialName("thumbnail_url") val thumbnailUrl: String? = null,
    @SerialName("is_primary") val isPrimary: Boolean = false,
    val caption: String? = null,
    val order: Int = 0,
)

/** Agency information. */
@Serializable
data class AgencyInfo(
    val id: String,
    val name: String,
    @SerialName("logo_url") val logoUrl: String? = null,
    val phone: String? = null,
    val email: String? = null,
)

/** Realtor information. */
@Serializable
data class RealtorInfo(
    val id: String,
    val name: String,
    @SerialName("avatar_url") val avatarUrl: String? = null,
    val phone: String? = null,
    val email: String? = null,
    val agency: AgencyInfo? = null,
)

/** Property listing summary for search results. */
@Serializable
data class ListingSummary(
    val id: String,
    val title: String,
    val type: ListingType,
    val category: PropertyCategory,
    val status: ListingStatus,
    val price: Long,
    val currency: String = "EUR",
    @SerialName("price_per_sqm") val pricePerSqm: Long? = null,
    @SerialName("area_sqm") val areaSqm: Double? = null,
    val rooms: Int? = null,
    val bedrooms: Int? = null,
    val bathrooms: Int? = null,
    val address: Address,
    @SerialName("primary_image") val primaryImage: ListingImage? = null,
    @SerialName("image_count") val imageCount: Int = 0,
    val realtor: RealtorInfo? = null,
    @SerialName("is_featured") val isFeatured: Boolean = false,
    @SerialName("is_new") val isNew: Boolean = false,
    @SerialName("is_price_reduced") val isPriceReduced: Boolean = false,
    @SerialName("created_at") val createdAt: String,
    @SerialName("updated_at") val updatedAt: String,
)

/** Full property listing details. */
@Serializable
data class ListingDetail(
    val id: String,
    val title: String,
    val description: String,
    val type: ListingType,
    val category: PropertyCategory,
    val status: ListingStatus,
    val price: Long,
    val currency: String = "EUR",
    @SerialName("price_per_sqm") val pricePerSqm: Long? = null,
    @SerialName("area_sqm") val areaSqm: Double,
    @SerialName("usable_area_sqm") val usableAreaSqm: Double? = null,
    @SerialName("land_area_sqm") val landAreaSqm: Double? = null,
    val rooms: Int? = null,
    val bedrooms: Int? = null,
    val bathrooms: Int? = null,
    val floor: Int? = null,
    @SerialName("total_floors") val totalFloors: Int? = null,
    @SerialName("year_built") val yearBuilt: Int? = null,
    @SerialName("year_renovated") val yearRenovated: Int? = null,
    val address: Address,
    val images: List<ListingImage> = emptyList(),
    val features: List<String> = emptyList(),
    @SerialName("energy_rating") val energyRating: String? = null,
    @SerialName("heating_type") val heatingType: String? = null,
    val parking: String? = null,
    val realtor: RealtorInfo? = null,
    @SerialName("is_featured") val isFeatured: Boolean = false,
    @SerialName("is_new") val isNew: Boolean = false,
    @SerialName("is_price_reduced") val isPriceReduced: Boolean = false,
    @SerialName("previous_price") val previousPrice: Long? = null,
    @SerialName("view_count") val viewCount: Int = 0,
    @SerialName("inquiry_count") val inquiryCount: Int = 0,
    @SerialName("created_at") val createdAt: String,
    @SerialName("updated_at") val updatedAt: String,
)

/** Search filters for listings. */
@Serializable
data class ListingSearchFilters(
    val type: ListingType? = null,
    val category: PropertyCategory? = null,
    val city: String? = null,
    val district: String? = null,
    val region: String? = null,
    @SerialName("min_price") val minPrice: Long? = null,
    @SerialName("max_price") val maxPrice: Long? = null,
    @SerialName("min_area") val minArea: Double? = null,
    @SerialName("max_area") val maxArea: Double? = null,
    @SerialName("min_rooms") val minRooms: Int? = null,
    @SerialName("max_rooms") val maxRooms: Int? = null,
    val bedrooms: Int? = null,
    val bathrooms: Int? = null,
    val features: List<String>? = null,
    @SerialName("year_built_min") val yearBuiltMin: Int? = null,
    @SerialName("year_built_max") val yearBuiltMax: Int? = null,
    @SerialName("near_lat") val nearLat: Double? = null,
    @SerialName("near_lng") val nearLng: Double? = null,
    @SerialName("radius_km") val radiusKm: Double? = null,
)

/** Sort options for listings. */
@Serializable
enum class ListingSortOption {
    @SerialName("newest") NEWEST,
    @SerialName("oldest") OLDEST,
    @SerialName("price_asc") PRICE_ASC,
    @SerialName("price_desc") PRICE_DESC,
    @SerialName("area_asc") AREA_ASC,
    @SerialName("area_desc") AREA_DESC,
    @SerialName("relevance") RELEVANCE,
}

/** Paginated search response. */
@Serializable
data class ListingSearchResponse(
    val listings: List<ListingSummary>,
    val total: Int,
    val page: Int,
    @SerialName("page_size") val pageSize: Int,
    @SerialName("total_pages") val totalPages: Int,
)

/** Search request. */
@Serializable
data class ListingSearchRequest(
    val query: String? = null,
    val filters: ListingSearchFilters? = null,
    val sort: ListingSortOption = ListingSortOption.NEWEST,
    val page: Int = 1,
    @SerialName("page_size") val pageSize: Int = 20,
)

/** Featured listings response. */
@Serializable data class FeaturedListingsResponse(val listings: List<ListingSummary>)

/** Recent listings response. */
@Serializable data class RecentListingsResponse(val listings: List<ListingSummary>)

/** City with listing count for suggestions. */
@Serializable
data class CitySuggestion(
    val city: String,
    val region: String? = null,
    val country: String,
    @SerialName("listing_count") val listingCount: Int,
)

/** Search suggestions response. */
@Serializable
data class SearchSuggestionsResponse(
    val cities: List<CitySuggestion>,
    val regions: List<String>,
    @SerialName("recent_searches") val recentSearches: List<String>,
)
