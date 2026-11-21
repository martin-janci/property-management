package three.two.bit.ppt.reality.listing

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

/**
 * Wire-level DTOs for the reality-server listing endpoints.
 *
 * The app's domain models ([ListingSummary], [ListingDetail], ...) predate the server and carry a
 * richer shape (nested address, realtor, typed enums). reality-server returns flatter payloads, so
 * [ListingRepository] decodes these DTOs and maps them onto the domain models. Field names and
 * nullability mirror `backend/servers/reality-server/src/routes/listings.rs`.
 */

/** `ListingSummary` row of `GET /api/v1/listings`. */
@Serializable
internal data class WireListingSummary(
    val id: String,
    val title: String,
    /**
     * SEO slug `{slugified-title}-{uuid}` (#3616). The server always emits this; it is nullable
     * here only so decoding tolerates older/empty payloads. The mapper falls back to [id] so the
     * domain [ListingSummary.slug] is always populated. The trailing UUID is the detail-lookup
     * identifier.
     */
    val slug: String? = null,
    val description: String? = null,
    val price: Long,
    val currency: String = "EUR",
    val area: Int? = null,
    val rooms: Int? = null,
    val city: String = "",
    @SerialName("photo_url") val photoUrl: String? = null,
    @SerialName("property_type") val propertyType: String = "",
    @SerialName("transaction_type") val transactionType: String = "",
    @SerialName("published_at") val publishedAt: String = "",
)

/** `ListingSearchResponse` of `GET /api/v1/listings`. */
@Serializable
internal data class WireListingSearchResponse(
    val listings: List<WireListingSummary>,
    val total: Long,
    val page: Int,
    val limit: Int,
)

/** `ListingDetail` of `GET /api/v1/listings/{id}`. */
@Serializable
internal data class WireListingDetail(
    val id: String,
    val title: String,
    val description: String? = null,
    val price: Long,
    val currency: String = "EUR",
    val area: Int? = null,
    val rooms: Int? = null,
    val bathrooms: Int? = null,
    val floor: Int? = null,
    @SerialName("total_floors") val totalFloors: Int? = null,
    val address: String = "",
    val city: String = "",
    val country: String = "",
    val latitude: Double? = null,
    val longitude: Double? = null,
    @SerialName("property_type") val propertyType: String = "",
    @SerialName("transaction_type") val transactionType: String = "",
    val photos: List<String> = emptyList(),
    val features: List<String> = emptyList(),
    @SerialName("published_at") val publishedAt: String = "",
    @SerialName("view_count") val viewCount: Long = 0,
)

/** camelCase address of a `/featured` row. */
@Serializable
internal data class WireRichAddress(
    val city: String = "",
    val district: String? = null,
    val street: String? = null,
    val postalCode: String? = null,
    val country: String = "",
    val latitude: Double? = null,
    val longitude: Double? = null,
)

/** camelCase photo of a `/featured` row. */
@Serializable
internal data class WireRichPhoto(
    val id: String,
    val url: String,
    val thumbnailUrl: String? = null,
    val isPrimary: Boolean = false,
    val order: Int = 0,
)

/** camelCase `RichListingSummary` of `GET /api/v1/listings/featured`. */
@Serializable
internal data class WireRichListingSummary(
    val id: String,
    val title: String,
    /** SEO slug `{slugified-title}-{uuid}` (#3616); see [WireListingSummary.slug]. */
    val slug: String? = null,
    val propertyType: String = "",
    val transactionType: String = "",
    val status: String = "active",
    val price: Long,
    val currency: String = "EUR",
    val pricePerSqm: Long? = null,
    val area: Int = 0,
    val rooms: Int? = null,
    val bedrooms: Int? = null,
    val bathrooms: Int? = null,
    val address: WireRichAddress = WireRichAddress(),
    val primaryPhoto: WireRichPhoto? = null,
    val isFeatured: Boolean = false,
    val createdAt: String = "",
    val updatedAt: String = "",
)

/** `FeaturedListingsResponse` (three buckets) of `GET /api/v1/listings/featured`. */
@Serializable
internal data class WireFeaturedListingsResponse(
    val sale: List<WireRichListingSummary> = emptyList(),
    val rent: List<WireRichListingSummary> = emptyList(),
    val new: List<WireRichListingSummary> = emptyList(),
)

/** `SuggestionsResponse` of `GET /api/v1/listings/suggestions`. */
@Serializable
internal data class WireSuggestionsResponse(
    val cities: List<String> = emptyList(),
    @SerialName("popular_searches") val popularSearches: List<String> = emptyList(),
)

/** Server `property_type` values (DB check `listings_property_type_check`). */
internal object ServerPropertyType {
    fun toDomain(raw: String): PropertyCategory =
        when (raw.lowercase()) {
            "apartment" -> PropertyCategory.APARTMENT
            "house" -> PropertyCategory.HOUSE
            "land" -> PropertyCategory.LAND
            "commercial" -> PropertyCategory.COMMERCIAL
            "parking" -> PropertyCategory.GARAGE
            "storage" -> PropertyCategory.WAREHOUSE
            else -> PropertyCategory.OTHER
        }

    fun fromDomain(category: PropertyCategory): String =
        when (category) {
            PropertyCategory.APARTMENT -> "apartment"
            PropertyCategory.HOUSE -> "house"
            PropertyCategory.LAND -> "land"
            PropertyCategory.COMMERCIAL,
            PropertyCategory.OFFICE -> "commercial"
            PropertyCategory.GARAGE -> "parking"
            PropertyCategory.WAREHOUSE -> "storage"
            PropertyCategory.OTHER -> "other"
        }
}

internal fun transactionTypeToDomain(raw: String): ListingType =
    if (raw.equals("rent", ignoreCase = true)) ListingType.RENT else ListingType.SALE

internal fun ListingType.toServer(): String =
    when (this) {
        ListingType.SALE -> "sale"
        ListingType.RENT -> "rent"
    }

/**
 * Server sort keys (`PortalRepository::search_listings`): `price_asc`, `price_desc`, `area_asc`,
 * `date_desc`. Anything else falls back to newest-first on the server, so options it cannot honour
 * map to `null` (server default) rather than to a misleading order.
 */
internal fun ListingSortOption.toServer(): String? =
    when (this) {
        ListingSortOption.NEWEST -> "date_desc"
        ListingSortOption.PRICE_ASC -> "price_asc"
        ListingSortOption.PRICE_DESC -> "price_desc"
        ListingSortOption.AREA_ASC -> "area_asc"
        ListingSortOption.OLDEST,
        ListingSortOption.AREA_DESC,
        ListingSortOption.RELEVANCE -> null
    }

private fun pricePerSqm(price: Long, area: Int?): Long? =
    area?.takeIf { it > 0 }?.let { price / it }

internal fun WireListingSummary.toDomain(): ListingSummary =
    ListingSummary(
        id = id,
        title = title,
        // Always-populated detail-lookup identifier; fall back to the bare UUID.
        slug = slug ?: id,
        type = transactionTypeToDomain(transactionType),
        category = ServerPropertyType.toDomain(propertyType),
        status = ListingStatus.ACTIVE,
        price = price,
        currency = currency,
        pricePerSqm = pricePerSqm(price, area),
        areaSqm = area?.toDouble(),
        rooms = rooms,
        address = Address(city = city, country = ""),
        primaryImage =
            photoUrl?.let { ListingImage(id = "$id-0", url = it, isPrimary = true, order = 0) },
        imageCount = if (photoUrl != null) 1 else 0,
        createdAt = publishedAt,
        updatedAt = publishedAt,
    )

internal fun WireRichListingSummary.toDomain(): ListingSummary =
    ListingSummary(
        id = id,
        title = title,
        // Always-populated detail-lookup identifier; fall back to the bare UUID.
        slug = slug ?: id,
        type = transactionTypeToDomain(transactionType),
        category = ServerPropertyType.toDomain(propertyType),
        status = ListingStatus.ACTIVE,
        price = price,
        currency = currency,
        pricePerSqm = pricePerSqm ?: pricePerSqm(price, area),
        areaSqm = area.takeIf { it > 0 }?.toDouble(),
        rooms = rooms,
        bedrooms = bedrooms,
        bathrooms = bathrooms,
        address =
            Address(
                street = address.street,
                city = address.city,
                district = address.district,
                postalCode = address.postalCode,
                country = address.country,
                coordinates =
                    if (address.latitude != null && address.longitude != null) {
                        Coordinates(address.latitude, address.longitude)
                    } else {
                        null
                    },
            ),
        primaryImage =
            primaryPhoto?.let {
                ListingImage(
                    id = it.id,
                    url = it.url,
                    thumbnailUrl = it.thumbnailUrl,
                    isPrimary = true,
                    order = it.order,
                )
            },
        imageCount = if (primaryPhoto != null) 1 else 0,
        isFeatured = isFeatured,
        createdAt = createdAt,
        updatedAt = updatedAt,
    )

internal fun WireListingDetail.toDomain(): ListingDetail =
    ListingDetail(
        id = id,
        title = title,
        description = description.orEmpty(),
        type = transactionTypeToDomain(transactionType),
        category = ServerPropertyType.toDomain(propertyType),
        status = ListingStatus.ACTIVE,
        price = price,
        currency = currency,
        pricePerSqm = pricePerSqm(price, area),
        areaSqm = area?.toDouble() ?: 0.0,
        rooms = rooms,
        bathrooms = bathrooms,
        floor = floor,
        totalFloors = totalFloors,
        address =
            Address(
                street = address.takeIf { it.isNotBlank() },
                city = city,
                country = country,
                coordinates =
                    if (latitude != null && longitude != null) {
                        Coordinates(latitude, longitude)
                    } else {
                        null
                    },
            ),
        images =
            photos.mapIndexed { index, url ->
                ListingImage(id = "$id-$index", url = url, isPrimary = index == 0, order = index)
            },
        features = features,
        viewCount = viewCount.coerceAtMost(Int.MAX_VALUE.toLong()).toInt(),
        createdAt = publishedAt,
        updatedAt = publishedAt,
    )

internal fun WireListingSearchResponse.toDomain(): ListingSearchResponse {
    val pageSize = limit.coerceAtLeast(1)
    val totalInt = total.coerceAtMost(Int.MAX_VALUE.toLong()).toInt()
    return ListingSearchResponse(
        listings = listings.map { it.toDomain() },
        total = totalInt,
        page = page,
        pageSize = pageSize,
        totalPages = ((totalInt + pageSize - 1) / pageSize),
    )
}
