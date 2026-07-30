package three.two.bit.ppt.reality.realtor

import io.ktor.client.HttpClient
import io.ktor.client.engine.mock.MockEngine
import io.ktor.client.engine.mock.respond
import io.ktor.client.plugins.contentnegotiation.ContentNegotiation
import io.ktor.http.HttpHeaders
import io.ktor.http.HttpStatusCode
import io.ktor.http.fullPath
import io.ktor.http.headersOf
import io.ktor.serialization.kotlinx.json.json
import io.ktor.utils.io.ByteReadChannel
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.Json

/**
 * Repository tests for [PortalListingsRepository] (Story 33.4).
 *
 * Pins the wire contract of reality-server's portal-listings surface: camelCase fields, a
 * `rust_decimal` price serialized as a JSON string (`"200000.00"`), the 401 mapping, and the
 * client-side portfolio-analytics aggregation that folds per-listing daily series together.
 */
class PortalListingsRepositoryTest {

    private val json = Json {
        ignoreUnknownKeys = true
        isLenient = true
        encodeDefaults = true
    }

    private fun repoWith(status: HttpStatusCode, body: String): PortalListingsRepository {
        val engine = MockEngine { _ ->
            respond(
                content = ByteReadChannel(body),
                status = status,
                headers = headersOf(HttpHeaders.ContentType, "application/json"),
            )
        }
        val client = HttpClient(engine) { install(ContentNegotiation) { json(json) } }
        return PortalListingsRepository(
            baseUrl = "https://example.test",
            sessionToken = "test-token",
            client = client,
        )
    }

    /** A MockEngine that routes by path so aggregation across list + analytics can be exercised. */
    private fun repoWithRouter(
        route: (path: String) -> Pair<HttpStatusCode, String>
    ): PortalListingsRepository {
        val engine = MockEngine { request ->
            val (status, body) = route(request.url.fullPath)
            respond(
                content = ByteReadChannel(body),
                status = status,
                headers = headersOf(HttpHeaders.ContentType, "application/json"),
            )
        }
        val client = HttpClient(engine) { install(ContentNegotiation) { json(json) } }
        return PortalListingsRepository(
            baseUrl = "https://example.test",
            sessionToken = "test-token",
            client = client,
        )
    }

    @Test
    fun listMyListings_decodes_camelCase_and_decimal_string_price() = runTest {
        val body =
            """
            {
              "listings": [
                {
                  "id": "lst-1",
                  "title": "Sunny 2BR",
                  "propertyType": "apartment",
                  "transactionType": "sale",
                  "price": "200000.00",
                  "currency": "EUR",
                  "city": "Bratislava",
                  "rooms": 2,
                  "sizeSqm": "64.50",
                  "status": "active",
                  "isPublished": true
                }
              ],
              "total": 1
            }
            """
                .trimIndent()
        val repo = repoWith(HttpStatusCode.OK, body)

        val result = repo.listMyListings()

        assertTrue(result.isSuccess, "expected success, got $result")
        val resp = result.getOrThrow()
        assertEquals(1, resp.listings.size)
        assertEquals(1L, resp.total)
        val listing = resp.listings.first()
        assertEquals("lst-1", listing.id)
        // Decimal string truncated to whole-currency units by DecimalAsLongSerializer.
        assertEquals(200000L, listing.price)
        assertEquals(64L, listing.sizeSqm)
        assertEquals("active", listing.status)
    }

    @Test
    fun listMyListings_maps_unauthorized_to_failure() = runTest {
        val repo = repoWith(HttpStatusCode.Unauthorized, """{"error":"unauthenticated"}""")
        val result = repo.listMyListings()
        assertTrue(result.isFailure, "401 should surface as failure, got $result")
        assertTrue(result.exceptionOrNull() is PortalListingException)
    }

    @Test
    fun getListingAnalytics_decodes_summary_and_daily_series() = runTest {
        val body =
            """
            {
              "listingId": "lst-1",
              "totalViews": 42,
              "totalInquiries": 3,
              "totalFavorites": 5,
              "daysOnMarket": 12,
              "dailyAnalytics": [
                {"date": "2026-07-01", "views": 20, "inquiries": 1},
                {"date": "2026-07-02", "views": 22, "inquiries": 2}
              ]
            }
            """
                .trimIndent()
        val repo = repoWith(HttpStatusCode.OK, body)

        val result = repo.getListingAnalytics("lst-1")

        assertTrue(result.isSuccess, "expected success, got $result")
        val analytics = result.getOrThrow()
        assertEquals(42L, analytics.totalViews)
        assertEquals(2, analytics.dailyAnalytics.size)
        assertEquals(20, analytics.dailyAnalytics.first().views)
    }

    @Test
    fun getListingAnalytics_maps_not_found_to_failure() = runTest {
        val repo = repoWith(HttpStatusCode.NotFound, """{"error":"not found"}""")
        val result = repo.getListingAnalytics("lst-x")
        assertTrue(result.isFailure, "404 should surface as failure, got $result")
        assertTrue(result.exceptionOrNull() is PortalListingException)
    }

    @Test
    fun getPortfolioAnalytics_folds_per_listing_series_by_date() = runTest {
        val listBody =
            """
            {
              "listings": [
                {"id": "lst-1", "title": "A", "price": "100000.00", "currency": "EUR", "status": "active"},
                {"id": "lst-2", "title": "B", "price": "150000.00", "currency": "EUR", "status": "paused"}
              ],
              "total": 2
            }
            """
                .trimIndent()
        val analytics1 =
            """
            {"listingId":"lst-1","totalViews":30,"totalInquiries":2,"totalFavorites":4,
             "dailyAnalytics":[{"date":"2026-07-01","views":10,"inquiries":1},
                               {"date":"2026-07-02","views":20,"inquiries":1}]}
            """
                .trimIndent()
        val analytics2 =
            """
            {"listingId":"lst-2","totalViews":15,"totalInquiries":1,"totalFavorites":3,
             "dailyAnalytics":[{"date":"2026-07-02","views":5,"inquiries":1}]}
            """
                .trimIndent()

        val repo = repoWithRouter { path ->
            when {
                path.contains("lst-1/analytics") -> HttpStatusCode.OK to analytics1
                path.contains("lst-2/analytics") -> HttpStatusCode.OK to analytics2
                path.contains("/my/listings") -> HttpStatusCode.OK to listBody
                else -> HttpStatusCode.NotFound to "{}"
            }
        }

        val result = repo.getPortfolioAnalytics()

        assertTrue(result.isSuccess, "expected success, got $result")
        val p = result.getOrThrow()
        assertEquals(2, p.totalListings)
        assertEquals(1, p.activeListings)
        assertEquals(45L, p.totalViews) // 30 + 15
        assertEquals(3L, p.totalInquiries) // 2 + 1
        assertEquals(7L, p.totalFavorites) // 4 + 3
        // Dates 2026-07-01 (10) and 2026-07-02 (20 + 5 = 25), sorted chronologically.
        assertEquals(listOf(10, 25), p.viewsTrend)
        assertEquals(listOf(1, 2), p.inquiriesTrend)
    }

    @Test
    fun getPortfolioAnalytics_keeps_days_with_inquiries_but_zero_views() = runTest {
        val listBody =
            """
            {
              "listings": [
                {"id": "lst-1", "title": "A", "price": "100000.00", "currency": "EUR", "status": "active"}
              ],
              "total": 1
            }
            """
                .trimIndent()
        // 2026-07-02 has an inquiry but zero views. Each DailyListingAnalytics row carries both
        // metrics, so the fold keys viewsByDate AND inquiriesByDate off the same d.date on every
        // iteration — a zero-view day still lands in viewsByDate (value 0). The date axis therefore
        // already retains it and its inquiry is never dropped. This test pins that behaviour so a
        // future refactor (e.g. only inserting into viewsByDate when views > 0) can't regress it.
        val analytics1 =
            """
            {"listingId":"lst-1","totalViews":10,"totalInquiries":2,"totalFavorites":0,
             "dailyAnalytics":[{"date":"2026-07-01","views":10,"inquiries":1},
                               {"date":"2026-07-02","views":0,"inquiries":1}]}
            """
                .trimIndent()

        val repo = repoWithRouter { path ->
            when {
                path.contains("lst-1/analytics") -> HttpStatusCode.OK to analytics1
                path.contains("/my/listings") -> HttpStatusCode.OK to listBody
                else -> HttpStatusCode.NotFound to "{}"
            }
        }

        val result = repo.getPortfolioAnalytics()

        assertTrue(result.isSuccess, "expected success, got $result")
        val p = result.getOrThrow()
        // Both days retained; 2026-07-02 contributes 0 views but 1 inquiry.
        assertEquals(listOf(10, 0), p.viewsTrend)
        assertEquals(listOf(1, 1), p.inquiriesTrend)
    }
}
