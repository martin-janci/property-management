package three.two.bit.ppt.reality.listing

import io.ktor.client.HttpClient
import io.ktor.client.engine.mock.MockEngine
import io.ktor.client.engine.mock.respond
import io.ktor.client.plugins.contentnegotiation.ContentNegotiation
import io.ktor.http.HttpHeaders
import io.ktor.http.HttpMethod
import io.ktor.http.HttpStatusCode
import io.ktor.http.Parameters
import io.ktor.http.headersOf
import io.ktor.serialization.kotlinx.json.json
import io.ktor.utils.io.ByteReadChannel
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.Json

/**
 * Pins [ListingRepository] to reality-server's real listing contract (`routes/listings.rs`): search
 * is `GET /api/v1/listings` with flat query params, `/featured` returns three camelCase buckets,
 * and "recent" is a `date_desc` search. Before this, the client POSTed to `/listings/search` and
 * GETed `/listings/recent`, neither of which exists.
 */
class ListingRepositoryServerContractTest {

    private val json = Json {
        ignoreUnknownKeys = true
        isLenient = true
        encodeDefaults = true
    }

    private class Captured {
        var method: HttpMethod? = null
        var path: String? = null
        var params: Parameters? = null
    }

    private fun repo(cap: Captured, body: String): ListingRepository {
        val engine = MockEngine { request ->
            cap.method = request.method
            cap.path = request.url.encodedPath
            cap.params = request.url.parameters
            respond(
                content = ByteReadChannel(body),
                status = HttpStatusCode.OK,
                headers = headersOf(HttpHeaders.ContentType, "application/json"),
            )
        }
        val client = HttpClient(engine) { install(ContentNegotiation) { json(json) } }
        return ListingRepository(baseUrl = "https://example.test", client = client)
    }

    private val searchBody =
        """
        {
          "listings": [
            {
              "id": "lst-1",
              "title": "Garage downtown",
              "slug": "garage-downtown-lst-1",
              "price": 20000,
              "currency": "EUR",
              "area": 18,
              "city": "Bratislava",
              "photo_url": "https://cdn.example.com/g.jpg",
              "property_type": "parking",
              "transaction_type": "sale",
              "published_at": "2026-04-26T10:00:00Z"
            }
          ],
          "total": 41,
          "page": 2,
          "limit": 20
        }
        """
            .trimIndent()

    @Test
    fun searchListings_uses_get_with_server_query_params() = runTest {
        val cap = Captured()
        val request =
            ListingSearchRequest(
                query = "garage",
                filters =
                    ListingSearchFilters(
                        type = ListingType.SALE,
                        category = PropertyCategory.GARAGE,
                        city = "Bratislava",
                        minPrice = 10_000,
                        maxPrice = 30_000,
                        minArea = 10.0,
                        minRooms = 1,
                        nearLat = 48.1,
                    ),
                sort = ListingSortOption.PRICE_ASC,
                page = 2,
                pageSize = 20,
            )

        val response = assertNotNull(repo(cap, searchBody).searchListings(request).getOrNull())

        assertEquals(HttpMethod.Get, cap.method)
        assertEquals("/api/v1/listings", cap.path)
        val p = assertNotNull(cap.params)
        assertEquals("garage", p["q"])
        assertEquals("sale", p["transaction_type"])
        assertEquals("parking", p["property_type"])
        assertEquals("Bratislava", p["city"])
        assertEquals("10000", p["price_min"])
        assertEquals("30000", p["price_max"])
        assertEquals("10", p["area_min"])
        assertEquals("1", p["rooms_min"])
        assertEquals("price_asc", p["sort"])
        assertEquals("2", p["page"])
        assertEquals("20", p["limit"])
        // Filters the server does not support are not sent.
        assertNull(p["near_lat"])

        assertEquals(41, response.total)
        assertEquals(3, response.totalPages)
        val item = response.listings.single()
        assertEquals(PropertyCategory.GARAGE, item.category)
        assertEquals(ListingType.SALE, item.type)
        assertEquals("Bratislava", item.address.city)
        assertEquals(18.0, item.areaSqm)
        assertEquals("https://cdn.example.com/g.jpg", item.primaryImage?.url)
    }

    @Test
    fun getRecentListings_is_a_date_desc_search() = runTest {
        val cap = Captured()

        val recent = assertNotNull(repo(cap, searchBody).getRecentListings(limit = 5).getOrNull())

        assertEquals("/api/v1/listings", cap.path)
        assertEquals("date_desc", cap.params?.get("sort"))
        assertEquals("5", cap.params?.get("limit"))
        assertEquals(1, recent.listings.size)
    }

    @Test
    fun getFeaturedListings_merges_sale_and_rent_buckets() = runTest {
        val row = { id: String, tx: String ->
            """
            {"id":"$id","title":"T $id","propertyType":"apartment","transactionType":"$tx",
             "status":"active","price":100000,"currency":"EUR","area":50,
             "address":{"city":"Košice","country":"SK"},
             "primaryPhoto":{"id":"p-$id","url":"https://cdn.example.com/$id.jpg",
               "thumbnailUrl":"https://cdn.example.com/$id-t.jpg","isPrimary":true,"order":0},
             "isFeatured":true,"isFavorite":false,
             "createdAt":"2026-04-26T10:00:00Z","updatedAt":"2026-04-26T10:00:00Z"}
            """
        }
        val body =
            """{"sale":[${row("a", "sale")},${row("b", "sale")}],"rent":[${row("c", "rent")}],"new":[${row("a", "sale")}]}"""
        val cap = Captured()

        val featured = assertNotNull(repo(cap, body).getFeaturedListings().getOrNull())

        assertEquals("/api/v1/listings/featured", cap.path)
        assertEquals(listOf("a", "c", "b"), featured.listings.map { it.id })
        val first = featured.listings.first()
        assertEquals("Košice", first.address.city)
        assertEquals(2_000L, first.pricePerSqm)
        assertTrue(first.isFeatured)
        assertEquals(ListingType.RENT, featured.listings[1].type)
    }

    @Test
    fun getSearchSuggestions_filters_server_cities_by_query() = runTest {
        val cap = Captured()
        val body =
            """{"cities":["Bratislava","Banská Bystrica","Košice"],"popular_searches":["Byty Bratislava"]}"""

        val result = assertNotNull(repo(cap, body).getSearchSuggestions("ban").getOrNull())

        assertEquals("/api/v1/listings/suggestions", cap.path)
        assertEquals(listOf("Banská Bystrica"), result.cities.map { it.city })
        assertEquals(listOf("Byty Bratislava"), result.recentSearches)
    }
}
