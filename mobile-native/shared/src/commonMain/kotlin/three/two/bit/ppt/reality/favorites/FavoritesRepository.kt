package three.two.bit.ppt.reality.favorites

import io.ktor.client.*
import io.ktor.client.call.*
import io.ktor.client.request.*
import io.ktor.http.*
import three.two.bit.ppt.reality.api.HttpClientProvider
import three.two.bit.ppt.reality.api.asPathSegment

/**
 * Repository for favorites and saved searches.
 *
 * Epic 48 - Story 48.3: Portal Mobile Favorites
 */
class FavoritesRepository(
    private val baseUrl: String,
    private val sessionToken: String? = null,
    private val client: HttpClient = HttpClientProvider.client,
) {

    private fun HttpRequestBuilder.configureRequest() {
        sessionToken?.let { header(HttpHeaders.Authorization, "Bearer $it") }
    }

    // --- Favorites ---

    /** Get user's favorite listings. */
    suspend fun getFavorites(): Result<FavoritesResponse> {
        return try {
            val response = client.get("$baseUrl/api/v1/favorites") { configureRequest() }

            if (response.status.isSuccess()) {
                Result.success(response.body())
            } else if (response.status == HttpStatusCode.Unauthorized) {
                Result.failure(FavoritesException("Please sign in to view favorites"))
            } else {
                Result.failure(FavoritesException("Failed to load favorites: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Add a listing to favorites.
     *
     * Endpoint: `POST /api/v1/favorites/{listing_id}` (listing ID is a path segment, not a body
     * field — the server's `AddFavorite` body only carries an optional `notes` field).
     */
    suspend fun addFavorite(listingId: String): Result<AddFavoriteResponse> {
        return try {
            val response =
                client.post("$baseUrl/api/v1/favorites/${listingId.asPathSegment()}") {
                    configureRequest()
                    // Empty typed body — server's `AddFavorite` is `{notes?: String}`; we omit it.
                    // Using the @Serializable `AddFavoriteBody` keeps the call on the
                    // ContentNegotiation pipeline instead of bypassing it with a raw "{}" string.
                    contentType(ContentType.Application.Json)
                    setBody(AddFavoriteBody)
                }

            if (response.status.isSuccess()) {
                Result.success(response.body())
            } else if (response.status == HttpStatusCode.Unauthorized) {
                Result.failure(FavoritesException("Please sign in to save favorites"))
            } else if (response.status == HttpStatusCode.Conflict) {
                Result.failure(FavoritesException("Already in favorites"))
            } else {
                Result.failure(FavoritesException("Failed to add favorite: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /** Remove a listing from favorites. */
    suspend fun removeFavorite(listingId: String): Result<Unit> {
        return try {
            val response =
                client.delete("$baseUrl/api/v1/favorites/${listingId.asPathSegment()}") {
                    configureRequest()
                }

            if (response.status.isSuccess()) {
                Result.success(Unit)
            } else if (response.status == HttpStatusCode.Unauthorized) {
                Result.failure(FavoritesException("Please sign in to manage favorites"))
            } else if (response.status == HttpStatusCode.NotFound) {
                Result.failure(FavoritesException("Favorite not found"))
            } else {
                Result.failure(FavoritesException("Failed to remove favorite: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Check if a listing is favorited.
     *
     * Endpoint: `GET /api/v1/favorites/{listing_id}/check`
     *
     * The server **always** returns HTTP 200 with body `{"is_favorited": bool}` — it does NOT
     * return 404 for an un-favorited listing. We must parse the response body instead of treating
     * any 2xx as `true`.
     */
    suspend fun isFavorite(listingId: String): Result<Boolean> {
        return try {
            val response =
                client.get("$baseUrl/api/v1/favorites/${listingId.asPathSegment()}/check") {
                    configureRequest()
                }

            when {
                response.status.isSuccess() -> {
                    val body: CheckFavoriteResponse = response.body()
                    Result.success(body.isFavorited)
                }
                response.status == HttpStatusCode.Unauthorized ->
                    Result.success(false) // unauthenticated → treat as not-favorited
                else ->
                    Result.failure(
                        FavoritesException("Failed to check favorite: ${response.status}")
                    )
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    // --- Price-tracking alerts (gap-84-3) ---

    /**
     * Fetch the authenticated user's favorite price-tracking alerts (newest first).
     *
     * Endpoint: `GET /api/v1/favorites/alerts?limit&offset`. The server clamps `limit` to `[1,
     * 200]` and floors `offset` at 0, and returns the page plus the total `unread_count`
     * (independent of the page window). Requires auth — unauthenticated callers get a 401 which is
     * surfaced as a [FavoritesException] so the UI can prompt sign-in.
     */
    suspend fun getFavoriteAlerts(
        limit: Int = 100,
        offset: Int = 0,
    ): Result<FavoriteAlertsResponse> {
        return try {
            val response =
                client.get("$baseUrl/api/v1/favorites/alerts") {
                    configureRequest()
                    parameter("limit", limit)
                    parameter("offset", offset)
                }

            if (response.status.isSuccess()) {
                Result.success(response.body())
            } else if (response.status == HttpStatusCode.Unauthorized) {
                Result.failure(FavoritesException("Please sign in to view price alerts"))
            } else {
                Result.failure(
                    FavoritesException("Failed to load price alerts: ${response.status}")
                )
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Mark a single favorite alert as read.
     *
     * Endpoint: `POST /api/v1/favorites/alerts/{alert_id}/read`. Idempotent on the server — an
     * already-read alert still returns 204 — so a 2xx is the only success signal. A 404 means the
     * alert doesn't exist or isn't owned by the caller.
     */
    suspend fun markFavoriteAlertRead(alertId: String): Result<Unit> {
        return try {
            val response =
                client.post("$baseUrl/api/v1/favorites/alerts/${alertId.asPathSegment()}/read") {
                    configureRequest()
                }

            if (response.status.isSuccess()) {
                Result.success(Unit)
            } else if (response.status == HttpStatusCode.Unauthorized) {
                Result.failure(FavoritesException("Please sign in to manage price alerts"))
            } else if (response.status == HttpStatusCode.NotFound) {
                Result.failure(FavoritesException("Alert not found"))
            } else {
                Result.failure(FavoritesException("Failed to mark alert read: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Mark all of the authenticated user's pending favorite alerts as read.
     *
     * Endpoint: `POST /api/v1/favorites/alerts/read-all`. Returns the count that were flipped.
     */
    suspend fun markAllFavoriteAlertsRead(): Result<MarkAllFavoriteAlertsReadResponse> {
        return try {
            val response =
                client.post("$baseUrl/api/v1/favorites/alerts/read-all") { configureRequest() }

            if (response.status.isSuccess()) {
                Result.success(response.body())
            } else if (response.status == HttpStatusCode.Unauthorized) {
                Result.failure(FavoritesException("Please sign in to manage price alerts"))
            } else {
                Result.failure(
                    FavoritesException("Failed to mark all alerts read: ${response.status}")
                )
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    // --- Saved Searches ---

    /** Get user's saved searches. */
    suspend fun getSavedSearches(): Result<SavedSearchesResponse> {
        return try {
            val response = client.get("$baseUrl/api/v1/saved-searches") { configureRequest() }

            if (response.status.isSuccess()) {
                Result.success(response.body())
            } else if (response.status == HttpStatusCode.Unauthorized) {
                Result.failure(FavoritesException("Please sign in to view saved searches"))
            } else {
                Result.failure(
                    FavoritesException("Failed to load saved searches: ${response.status}")
                )
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /** Create a new saved search. */
    suspend fun createSavedSearch(request: CreateSavedSearchRequest): Result<SavedSearch> {
        return try {
            val response =
                client.post("$baseUrl/api/v1/saved-searches") {
                    configureRequest()
                    setBody(request)
                }

            if (response.status.isSuccess()) {
                Result.success(response.body())
            } else if (response.status == HttpStatusCode.Unauthorized) {
                Result.failure(FavoritesException("Please sign in to save searches"))
            } else {
                Result.failure(FavoritesException("Failed to save search: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /** Update a saved search. */
    suspend fun updateSavedSearch(
        searchId: String,
        request: UpdateSavedSearchRequest,
    ): Result<SavedSearch> {
        return try {
            val response =
                client.patch("$baseUrl/api/v1/saved-searches/${searchId.asPathSegment()}") {
                    configureRequest()
                    setBody(request)
                }

            if (response.status.isSuccess()) {
                Result.success(response.body())
            } else if (response.status == HttpStatusCode.Unauthorized) {
                Result.failure(FavoritesException("Please sign in to update saved searches"))
            } else if (response.status == HttpStatusCode.NotFound) {
                Result.failure(FavoritesException("Saved search not found"))
            } else {
                Result.failure(
                    FavoritesException("Failed to update saved search: ${response.status}")
                )
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /** Delete a saved search. */
    suspend fun deleteSavedSearch(searchId: String): Result<Unit> {
        return try {
            val response =
                client.delete("$baseUrl/api/v1/saved-searches/${searchId.asPathSegment()}") {
                    configureRequest()
                }

            if (response.status.isSuccess()) {
                Result.success(Unit)
            } else if (response.status == HttpStatusCode.Unauthorized) {
                Result.failure(FavoritesException("Please sign in to manage saved searches"))
            } else if (response.status == HttpStatusCode.NotFound) {
                Result.failure(FavoritesException("Saved search not found"))
            } else {
                Result.failure(
                    FavoritesException("Failed to delete saved search: ${response.status}")
                )
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /** Toggle alert for a saved search. */
    suspend fun toggleSearchAlert(searchId: String, enabled: Boolean): Result<SavedSearch> {
        return updateSavedSearch(searchId, UpdateSavedSearchRequest(alertEnabled = enabled))
    }
}

/** Favorites-specific exception. */
class FavoritesException(message: String) : Exception(message)
