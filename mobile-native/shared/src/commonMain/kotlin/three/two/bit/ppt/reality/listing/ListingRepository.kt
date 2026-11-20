package three.two.bit.ppt.reality.listing

import io.ktor.client.*
import io.ktor.client.call.*
import io.ktor.client.request.*
import io.ktor.http.*
import three.two.bit.ppt.reality.api.HttpClientProvider
import three.two.bit.ppt.reality.api.asPathSegment

/**
 * Repository for listing operations.
 *
 * Epic 48 - Story 48.1: Portal Mobile Search
 */
class ListingRepository(
    private val baseUrl: String,
    private val sessionToken: String? = null,
    private val client: HttpClient = HttpClientProvider.client,
) {

    private fun HttpRequestBuilder.configureRequest() {
        sessionToken?.let { header(HttpHeaders.Authorization, "Bearer $it") }
    }

    /**
     * Search listings with filters and pagination.
     *
     * reality-server exposes search as `GET /api/v1/listings` with flat query parameters (there is
     * no `POST /listings/search`). Filters the server does not support (district, region, bedrooms,
     * bathrooms, features, year built, geo radius) are not sent.
     */
    suspend fun searchListings(request: ListingSearchRequest): Result<ListingSearchResponse> {
        return try {
            val response =
                client.get("$baseUrl/api/v1/listings") {
                    configureRequest()
                    applySearch(request)
                }

            if (response.status.isSuccess()) {
                Result.success(response.body<WireListingSearchResponse>().toDomain())
            } else {
                Result.failure(ListingException("Search failed: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /** Get listing details by ID (bare UUID or SEO slug). */
    suspend fun getListingDetail(listingId: String): Result<ListingDetail> {
        return try {
            val response =
                client.get("$baseUrl/api/v1/listings/${listingId.asPathSegment()}") {
                    configureRequest()
                }

            if (response.status.isSuccess()) {
                Result.success(response.body<WireListingDetail>().toDomain())
            } else if (response.status == HttpStatusCode.NotFound) {
                Result.failure(ListingException("Listing not found"))
            } else {
                Result.failure(ListingException("Failed to load listing: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Get featured listings for homepage.
     *
     * The server returns three buckets (`sale`, `rent`, `new`); the home rail shows the sale and
     * rent buckets interleaved, de-duplicated by id.
     */
    suspend fun getFeaturedListings(): Result<FeaturedListingsResponse> {
        return try {
            val response = client.get("$baseUrl/api/v1/listings/featured") { configureRequest() }

            if (response.status.isSuccess()) {
                val wire = response.body<WireFeaturedListingsResponse>()
                val merged = interleave(wire.sale, wire.rent).distinctBy { it.id }
                Result.success(FeaturedListingsResponse(listings = merged.map { it.toDomain() }))
            } else {
                Result.failure(ListingException("Failed to load featured: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /** Get recent listings (newest published first, via the search endpoint). */
    suspend fun getRecentListings(limit: Int = 10): Result<RecentListingsResponse> {
        return try {
            val response =
                client.get("$baseUrl/api/v1/listings") {
                    configureRequest()
                    parameter("sort", "date_desc")
                    parameter("page", 1)
                    parameter("limit", limit)
                }

            if (response.status.isSuccess()) {
                val wire = response.body<WireListingSearchResponse>()
                Result.success(
                    RecentListingsResponse(listings = wire.listings.map { it.toDomain() }),
                )
            } else {
                Result.failure(ListingException("Failed to load recent: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Get search suggestions for a partial query.
     *
     * The server returns nearby city names and popular search phrases without counts; cities are
     * filtered client-side by [query], popular phrases are surfaced as
     * [SearchSuggestionsResponse.recentSearches].
     */
    suspend fun getSearchSuggestions(query: String): Result<SearchSuggestionsResponse> {
        return try {
            val response = client.get("$baseUrl/api/v1/listings/suggestions") { configureRequest() }

            if (response.status.isSuccess()) {
                val wire = response.body<WireSuggestionsResponse>()
                val needle = query.trim()
                val cities =
                    wire.cities
                        .filter { needle.isEmpty() || it.contains(needle, ignoreCase = true) }
                        .map { CitySuggestion(city = it, country = "", listingCount = 0) }
                Result.success(
                    SearchSuggestionsResponse(
                        cities = cities,
                        regions = emptyList(),
                        recentSearches = wire.popularSearches,
                    ),
                )
            } else {
                Result.failure(ListingException("Failed to load suggestions: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    private fun HttpRequestBuilder.applySearch(request: ListingSearchRequest) {
        request.query?.takeIf { it.isNotBlank() }?.let { parameter("q", it) }
        request.filters?.let { f ->
            f.type?.let { parameter("transaction_type", it.toServer()) }
            f.category?.let { parameter("property_type", ServerPropertyType.fromDomain(it)) }
            f.city?.takeIf { it.isNotBlank() }?.let { parameter("city", it) }
            f.minPrice?.let { parameter("price_min", it) }
            f.maxPrice?.let { parameter("price_max", it) }
            f.minArea?.let { parameter("area_min", it.toInt()) }
            f.maxArea?.let { parameter("area_max", it.toInt()) }
            f.minRooms?.let { parameter("rooms_min", it) }
            f.maxRooms?.let { parameter("rooms_max", it) }
        }
        request.sort.toServer()?.let { parameter("sort", it) }
        parameter("page", request.page)
        parameter("limit", request.pageSize)
    }

    private fun <T> interleave(a: List<T>, b: List<T>): List<T> {
        val out = ArrayList<T>(a.size + b.size)
        for (i in 0 until maxOf(a.size, b.size)) {
            a.getOrNull(i)?.let(out::add)
            b.getOrNull(i)?.let(out::add)
        }
        return out
    }
}

/** Listing-specific exception. */
class ListingException(message: String) : Exception(message)
