package three.two.bit.ppt.reality.listing

import io.ktor.client.HttpClient
import io.ktor.client.engine.mock.MockEngine
import io.ktor.client.engine.mock.respond
import io.ktor.client.plugins.contentnegotiation.ContentNegotiation
import io.ktor.http.HttpHeaders
import io.ktor.http.HttpMethod
import io.ktor.http.HttpStatusCode
import io.ktor.http.headersOf
import io.ktor.utils.io.ByteReadChannel
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.Json

/**
 * Repository-level coroutine tests for [ListingRepository.getListingDetail] — the fetch that
 * feeds the Reality Portal listing-detail screen.
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

    private val fullDetailBody =
        """
        {
          "id": "lst-99",
          "title": "Bright 3br with terrace",
          "description": "A lovely flat in Old Town.",
          "type": "sale",
          "category": "apartment",
          "status": "active",
          "price": 285000,
          "currency": "EUR",
          "price_per_sqm": 3800,
          "area_sqm": 75.0,
          "rooms": 3,
          "bedrooms": 2,
          "bathrooms": 1,
          "year_built": 2008,
          "address": {
            "street": "Hlavna 1",
            "city": "Bratislava",
            "postal_code": "81101",
            "country": "SK",
            "coordinates": { "latitude": 48.15, "longitude": 17.10 }
          },
          "images": [
            { "id": "img-1", "url": "https://cdn.example.com/a.jpg", "is_primary": true },
            { "id": "img-2", "url": "https://cdn.example.com/b.jpg",
              "thumbnail_url": "https://cdn.example.com/b-thumb.jpg" }
          ],
          "features": ["balcony", "elevator", "parking"],
          "energy_rating": "B",
          "realtor": {
            "id": "rlt-1",
            "name": "Jana Realitka",
            "phone": "+421900000000",
            "agency": { "id": "ag-1", "name": "TopReality" }
          },
          "view_count": 120,
          "inquiry_count": 4,
          "created_at": "2026-04-26T10:00:00Z",
          "updated_at": "2026-04-27T09:00:00Z"
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

        // Render-contract: the fields the detail screen binds must all decode.
        assertEquals("Bright 3br with terrace", detail.title)
        assertEquals(ListingType.SALE, detail.type)
        assertEquals(PropertyCategory.APARTMENT, detail.category)
        assertEquals(ListingStatus.ACTIVE, detail.status)
        assertEquals(285_000L, detail.price)
        assertEquals(3_800L, detail.pricePerSqm)
        assertEquals(75.0, detail.areaSqm)
        assertEquals(3, detail.rooms)
        assertEquals(listOf("balcony", "elevator", "parking"), detail.features)
        assertEquals(2, detail.images.size)
        assertTrue(detail.images.first().isPrimary)
        assertEquals("Bratislava", detail.address.city)
        assertNotNull(detail.address.coordinates)
        val realtor = assertNotNull(detail.realtor)
        assertEquals("Jana Realitka", realtor.name)
        assertEquals("TopReality", realtor.agency?.name)
        assertEquals(120, detail.viewCount)
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
}
