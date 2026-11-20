package three.two.bit.ppt.reality.listing

import io.ktor.client.HttpClient
import io.ktor.client.engine.mock.MockEngine
import io.ktor.client.engine.mock.respond
import io.ktor.client.plugins.contentnegotiation.ContentNegotiation
import io.ktor.http.HttpHeaders
import io.ktor.http.HttpMethod
import io.ktor.http.HttpStatusCode
import io.ktor.http.headersOf
import io.ktor.serialization.kotlinx.json.json
import io.ktor.utils.io.ByteReadChannel
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.Json

/**
 * Repository-level coroutine tests for [ListingRepository.getListingDetail] — the fetch that feeds
 * the Reality Portal listing-detail screen.
 *
 * The [ListingModelsContractTest] pins the raw DTO encode/decode; this pins the *render contract*
 * as seen through the repository: the GET hits `/api/v1/listings/{id}`, a 200 decodes a full
 * [ListingDetail] (so the detail screen can render title/price/images/features/realtor), a 404 maps
 * to the user-facing "Listing not found", and other non-2xx surface a [ListingException].
 */
class ListingDetailRepositoryTest {

    private val json = Json {
        ignoreUnknownKeys = true
        isLenient = true
        encodeDefaults = true
    }

    private class Captured {
        var method: HttpMethod? = null
        var path: String? = null
    }

    private fun repoCapturing(
        captured: Captured,
        status: HttpStatusCode,
        body: String,
    ): ListingRepository {
        val engine = MockEngine { request ->
            captured.method = request.method
            captured.path = request.url.encodedPath
            respond(
                content = ByteReadChannel(body),
                status = status,
                headers = headersOf(HttpHeaders.ContentType, "application/json"),
            )
        }
        val client = HttpClient(engine) { install(ContentNegotiation) { json(json) } }
        return ListingRepository(
            baseUrl = "https://example.test",
            sessionToken = "test-token",
            client = client,
        )
    }

    /** Server wire shape: `ListingDetail` in reality-server `routes/listings.rs`. */
    private val fullDetailBody =
        """
        {
          "id": "lst-99",
          "title": "Bright 3br with terrace",
          "description": "A lovely flat in Old Town.",
          "price": 285000,
          "currency": "EUR",
          "area": 75,
          "rooms": 3,
          "bathrooms": 1,
          "floor": 2,
          "total_floors": 5,
          "address": "Hlavna 1, 81101 Bratislava",
          "city": "Bratislava",
          "country": "SK",
          "latitude": 48.15,
          "longitude": 17.10,
          "property_type": "apartment",
          "transaction_type": "sale",
          "photos": ["https://cdn.example.com/a.jpg", "https://cdn.example.com/b.jpg"],
          "features": ["balcony", "elevator", "parking"],
          "published_at": "2026-04-26T10:00:00Z",
          "view_count": 120
        }
        """
            .trimIndent()

    @Test
    fun getListingDetail_gets_by_id_path_and_decodes_render_fields() = runTest {
        val cap = Captured()
        val repo = repoCapturing(cap, HttpStatusCode.OK, fullDetailBody)

        val result = repo.getListingDetail("lst-99")

        assertTrue(result.isSuccess, "expected success, got $result")
        val detail = assertNotNull(result.getOrNull())

        // Endpoint contract.
        assertEquals(HttpMethod.Get, cap.method)
        assertEquals("/api/v1/listings/lst-99", cap.path)

        // Render-contract: the server's flat payload maps onto the fields the screen binds.
        assertEquals("Bright 3br with terrace", detail.title)
        assertEquals(ListingType.SALE, detail.type)
        assertEquals(PropertyCategory.APARTMENT, detail.category)
        assertEquals(ListingStatus.ACTIVE, detail.status)
        assertEquals(285_000L, detail.price)
        assertEquals(3_800L, detail.pricePerSqm)
        assertEquals(75.0, detail.areaSqm)
        assertEquals(3, detail.rooms)
        assertEquals(2, detail.floor)
        assertEquals(listOf("balcony", "elevator", "parking"), detail.features)
        assertEquals(2, detail.images.size)
        assertTrue(detail.images.first().isPrimary)
        assertEquals("https://cdn.example.com/a.jpg", detail.images.first().url)
        assertEquals("Bratislava", detail.address.city)
        assertEquals("Hlavna 1, 81101 Bratislava", detail.address.street)
        assertNotNull(detail.address.coordinates)
        assertEquals(120, detail.viewCount)
        assertEquals("2026-04-26T10:00:00Z", detail.createdAt)
    }

    @Test
    fun getListingDetail_maps_server_property_types() = runTest {
        val body =
            fullDetailBody
                .replace("\"apartment\"", "\"parking\"")
                .replace("\"transaction_type\": \"sale\"", "\"transaction_type\": \"rent\"")
        val repo = repoCapturing(Captured(), HttpStatusCode.OK, body)

        val detail = assertNotNull(repo.getListingDetail("lst-99").getOrNull())

        assertEquals(PropertyCategory.GARAGE, detail.category)
        assertEquals(ListingType.RENT, detail.type)
    }

    @Test
    fun getListingDetail_maps_404_to_listing_not_found() = runTest {
        val cap = Captured()
        val repo = repoCapturing(cap, HttpStatusCode.NotFound, """{"error":"missing"}""")

        val result = repo.getListingDetail("nope")

        assertTrue(result.isFailure, "404 should surface as failure, got $result")
        val ex = result.exceptionOrNull()
        assertTrue(
            ex is ListingException,
            "expected ListingException, got ${ex?.let { it::class }}",
        )
        assertEquals("Listing not found", ex?.message)
    }

    @Test
    fun getListingDetail_maps_server_error_to_failure() = runTest {
        val cap = Captured()
        val repo = repoCapturing(cap, HttpStatusCode.InternalServerError, """{"error":"boom"}""")

        val result = repo.getListingDetail("lst-99")

        assertTrue(result.isFailure, "5xx should surface as failure, got $result")
        assertTrue(result.exceptionOrNull() is ListingException)
    }

    /**
     * Security (#87ed762c): the listingId is deep-link / nav-sourced and is spliced into the
     * request path, so it must be percent-encoded — mirroring `ApiClient.getListing` /
     * `pathSegment()` — so a hostile id cannot smuggle extra path segments or query parameters
     * (path-injection). A `/`, `?`, `#`, or `..`-bearing id must NOT escape its segment.
     */
    @Test
    fun getListingDetail_url_encodes_listing_id_path_segment() = runTest {
        val cap = Captured()
        val repo = repoCapturing(cap, HttpStatusCode.OK, fullDetailBody)

        repo.getListingDetail("../admin/secret")

        // The `/` characters must be percent-encoded; the path stays anchored under /listings/.
        assertEquals(
            "/api/v1/listings/..%2Fadmin%2Fsecret",
            cap.path,
            "listingId must be percent-encoded into a single path segment",
        )
    }
}
