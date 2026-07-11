package three.two.bit.ppt.reality.api

import io.ktor.client.call.*
import io.ktor.client.request.*
import io.ktor.http.*
import three.two.bit.ppt.reality.favorites.AddFavoriteBody
import three.two.bit.ppt.reality.favorites.AddFavoriteResponse
import three.two.bit.ppt.reality.favorites.FavoritesResponse
import three.two.bit.ppt.reality.inquiry.CreateInquiryRequest
import three.two.bit.ppt.reality.inquiry.CreateInquiryResponse
import three.two.bit.ppt.reality.listing.ListingDetail
import three.two.bit.ppt.reality.listing.ListingSearchRequest
import three.two.bit.ppt.reality.listing.ListingSearchResponse

/**
 * Reality Portal API Client.
 *
 * This client provides common API operations for the Reality Portal mobile app. For full OpenAPI
 * spec generation, run: `openapi-generator generate -i
 * docs/api/generated/by-service/reality-server.yaml -g kotlin -o
 * mobile-native/shared/src/commonMain/kotlin/api`
 *
 * Note: Uses shared HttpClientProvider to avoid resource leaks (Epic 48 - Code Review Fix).
 *
 * Epic 48 - Common API operations for listings, favorites, and inquiries.
 */
class ApiClient(
    private val baseUrl: String = ApiConfig.requireBaseUrl(),
    private val accessToken: String? = null,
    private val tenantId: String? = null,
) {
    private val client = HttpClientProvider.client

    private fun HttpRequestBuilder.configureRequest() {
        accessToken?.let { header(HttpHeaders.Authorization, "Bearer $it") }
        tenantId?.let { header("X-Tenant-ID", it) }
    }

    suspend fun healthCheck(): String {
        return client.get("$baseUrl/health") { configureRequest() }.toString()
    }

    // --- Listings API ---

    /**
     * Search listings with filters and pagination.
     *
     * @param request Search request with query, filters, and pagination
     * @return Result containing paginated listing summaries or error
     */
    suspend fun getListings(request: ListingSearchRequest): Result<ListingSearchResponse> {
        return try {
            val response =
                client.post("$baseUrl/api/v1/listings/search") {
                    configureRequest()
                    setBody(request)
                }
            if (response.status.isSuccess()) {
                Result.success(response.body())
            } else {
                Result.failure(ApiException("Failed to fetch listings: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Get listing details by ID.
     *
     * @param id Listing ID
     * @return Result containing listing details or error
     */
    suspend fun getListing(id: String): Result<ListingDetail> {
        return try {
            val response =
                client.get("$baseUrl/api/v1/listings/${id.asPathSegment()}") { configureRequest() }
            if (response.status.isSuccess()) {
                Result.success(response.body())
            } else if (response.status == HttpStatusCode.NotFound) {
                Result.failure(ApiException("Listing not found"))
            } else {
                Result.failure(ApiException("Failed to fetch listing: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    // --- Favorites API ---

    /**
     * Get user's favorite listings. Requires authentication.
     *
     * @return Result containing favorites list or error
     */
    suspend fun getFavorites(): Result<FavoritesResponse> {
        return try {
            val response = client.get("$baseUrl/api/v1/favorites") { configureRequest() }
            if (response.status.isSuccess()) {
                Result.success(response.body())
            } else if (response.status == HttpStatusCode.Unauthorized) {
                Result.failure(ApiException("Please sign in to view favorites"))
            } else {
                Result.failure(ApiException("Failed to fetch favorites: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Add a listing to favorites. Requires authentication.
     *
     * @param listingId ID of the listing to add to favorites
     * @return Result containing add favorite response or error
     */
    suspend fun addFavorite(listingId: String): Result<AddFavoriteResponse> {
        return try {
            val response =
                client.post("$baseUrl/api/v1/favorites/${listingId.asPathSegment()}") {
                    configureRequest()
                    // Empty typed body — `AddFavoriteBody` serializes to `{}` via
                    // ContentNegotiation, keeping this call on the same pipeline as every other.
                    contentType(ContentType.Application.Json)
                    setBody(AddFavoriteBody)
                }
            if (response.status.isSuccess()) {
                Result.success(response.body())
            } else if (response.status == HttpStatusCode.Unauthorized) {
                Result.failure(ApiException("Please sign in to save favorites"))
            } else if (response.status == HttpStatusCode.Conflict) {
                Result.failure(ApiException("Already in favorites"))
            } else {
                Result.failure(ApiException("Failed to add favorite: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Remove a listing from favorites. Requires authentication.
     *
     * @param listingId ID of the listing to remove from favorites
     * @return Result containing Unit on success or error
     */
    suspend fun removeFavorite(listingId: String): Result<Unit> {
        return try {
            val response =
                client.delete("$baseUrl/api/v1/favorites/${listingId.asPathSegment()}") {
                    configureRequest()
                }
            if (response.status.isSuccess()) {
                Result.success(Unit)
            } else if (response.status == HttpStatusCode.Unauthorized) {
                Result.failure(ApiException("Please sign in to manage favorites"))
            } else if (response.status == HttpStatusCode.NotFound) {
                Result.failure(ApiException("Favorite not found"))
            } else {
                Result.failure(ApiException("Failed to remove favorite: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    // --- Inquiries API ---

    /**
     * Submit an inquiry for a listing.
     *
     * @param request Inquiry request with listing ID and message
     * @return Result containing inquiry response or error
     */
    suspend fun submitInquiry(request: CreateInquiryRequest): Result<CreateInquiryResponse> {
        return try {
            val response =
                client.post("$baseUrl/api/v1/inquiries") {
                    configureRequest()
                    setBody(request)
                }
            if (response.status.isSuccess()) {
                Result.success(response.body())
            } else {
                Result.failure(ApiException("Failed to submit inquiry: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
}

/** API-specific exception for client operations. */
class ApiException(message: String) : Exception(message)
