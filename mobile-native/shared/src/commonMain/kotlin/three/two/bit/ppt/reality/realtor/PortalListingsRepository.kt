package three.two.bit.ppt.reality.realtor

import io.ktor.client.*
import io.ktor.client.call.*
import io.ktor.client.request.*
import io.ktor.http.*
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.coroutineScope
import three.two.bit.ppt.reality.api.HttpClientProvider
import three.two.bit.ppt.reality.api.asPathSegment

/**
 * Repository for a realtor's own listings + analytics (Story 33.4).
 *
 * Wraps reality-server's portal-listings surface
 * (`backend/servers/reality-server/src/routes/portal_listings.rs`). Every endpoint is gated by
 * `PortalPrincipal` and scoped to the caller's own rows, so all reads carry the session bearer
 * token. This closes the mobile MyListings / ListingAnalytics screens, which previously rendered
 * permanent empty stubs because no LIST / analytics HTTP surface existed.
 */
class PortalListingsRepository(
    private val baseUrl: String,
    private val sessionToken: String? = null,
    private val client: HttpClient = HttpClientProvider.client,
) {

    private fun HttpRequestBuilder.configureRequest() {
        sessionToken?.let { header(HttpHeaders.Authorization, "Bearer $it") }
    }

    /**
     * List the signed-in portal user's own listings (`GET /api/v1/my/listings`).
     *
     * @param status optional lifecycle filter (e.g. `draft`, `active`, `paused`, `sold`, `rented`,
     *   `archived`); omit to return all of the caller's listings.
     * @param limit page size (server clamps to 1..=100).
     * @param offset row offset.
     */
    suspend fun listMyListings(
        status: String? = null,
        limit: Int = 20,
        offset: Int = 0,
    ): Result<MyListingsResponse> {
        return try {
            val response =
                client.get("$baseUrl/api/v1/my/listings") {
                    configureRequest()
                    status?.let { parameter("status", it) }
                    parameter("limit", limit)
                    parameter("offset", offset)
                }
            when {
                response.status.isSuccess() -> Result.success(response.body())
                response.status == HttpStatusCode.Unauthorized ->
                    Result.failure(PortalListingException("Please sign in to view your listings"))
                else ->
                    Result.failure(
                        PortalListingException("Failed to load listings: ${response.status}")
                    )
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Analytics summary + daily series for one of the caller's listings (`GET
     * /api/v1/my/listings/{id}/analytics`). A listing the caller does not own yields 404.
     *
     * @param fromDate inclusive lower bound (ISO-8601 date, e.g. `2026-07-01`); omit for none.
     * @param toDate inclusive upper bound (ISO-8601 date); omit for none.
     */
    suspend fun getListingAnalytics(
        listingId: String,
        fromDate: String? = null,
        toDate: String? = null,
    ): Result<ListingAnalytics> {
        return try {
            val response =
                client.get("$baseUrl/api/v1/my/listings/${listingId.asPathSegment()}/analytics") {
                    configureRequest()
                    fromDate?.let { parameter("fromDate", it) }
                    toDate?.let { parameter("toDate", it) }
                }
            when {
                response.status.isSuccess() -> Result.success(response.body())
                response.status == HttpStatusCode.Unauthorized ->
                    Result.failure(PortalListingException("Please sign in to view analytics"))
                response.status == HttpStatusCode.NotFound ->
                    Result.failure(PortalListingException("Listing not found or not owned"))
                else ->
                    Result.failure(
                        PortalListingException("Failed to load analytics: ${response.status}")
                    )
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Realtor-wide analytics aggregated across all of the caller's listings.
     *
     * The backend exposes analytics per listing but no portfolio-wide roll-up, and the realtor
     * dashboard has no single listing id, so this fans out one analytics call per listing
     * (concurrently) and folds the per-day series together by date into a single portfolio trend.
     * Individual per-listing analytics failures are skipped so one bad row can't sink the whole
     * dashboard; a failure listing the caller's listings, however, propagates.
     */
    suspend fun getPortfolioAnalytics(): Result<PortfolioAnalytics> {
        val listings =
            listMyListings(limit = 100)
                .getOrElse {
                    return Result.failure(it)
                }
                .listings

        val analytics: List<ListingAnalytics> = coroutineScope {
            listings
                .map { listing -> async { getListingAnalytics(listing.id).getOrNull() } }
                .awaitAll()
                .filterNotNull()
        }

        var totalViews = 0L
        var totalInquiries = 0L
        var totalFavorites = 0L
        val viewsByDate = HashMap<String, Int>()
        val inquiriesByDate = HashMap<String, Int>()
        analytics.forEach { a ->
            totalViews += a.totalViews
            totalInquiries += a.totalInquiries
            totalFavorites += a.totalFavorites
            a.dailyAnalytics.forEach { d ->
                viewsByDate[d.date] = (viewsByDate[d.date] ?: 0) + d.views
                inquiriesByDate[d.date] = (inquiriesByDate[d.date] ?: 0) + d.inquiries
            }
        }
        // ISO-8601 dates sort lexicographically, so a plain sort gives chronological order.
        val dates = viewsByDate.keys.sorted()

        return Result.success(
            PortfolioAnalytics(
                totalListings = listings.size,
                activeListings = listings.count { it.status.equals("active", ignoreCase = true) },
                totalViews = totalViews,
                totalInquiries = totalInquiries,
                totalFavorites = totalFavorites,
                viewsTrend = dates.map { viewsByDate[it] ?: 0 },
                inquiriesTrend = dates.map { inquiriesByDate[it] ?: 0 },
            )
        )
    }
}

/** Portal-listings-specific exception. */
class PortalListingException(message: String) : Exception(message)
