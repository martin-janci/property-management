package three.two.bit.ppt.reality.listing

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.serialization.decodeFromString
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

/**
 * Contract tests for the listing-domain DTOs.
 *
 * The reality-server controls the JSON shape; these tests pin the client-side mapping so any drift
 * in the OpenAPI generator output, the @SerialName annotations, or the manual model edits is caught
 * at build time. They cover:
 * - Snake_case <-> camelCase mapping for fields like primary_image, price_per_sqm.
 * - Round-trip stability for ListingDetail and ListingSummary.
 * - Decoding of API responses with optional / missing fields.
 * - SerialName mapping for ListingType, PropertyCategory, ListingStatus, ListingSortOption.
 */
class ListingModelsContractTest {

    private val json = Json {
        ignoreUnknownKeys = true
        encodeDefaults = false
    }

    // -------------------------------------------------------------------
    // Enum SerialName contracts
    // -------------------------------------------------------------------

    @Test
    fun listingType_serialises_to_lowercase_string() {
        assertEquals("\"sale\"", json.encodeToString(ListingType.SALE))
        assertEquals("\"rent\"", json.encodeToString(ListingType.RENT))
    }

    @Test
    fun propertyCategory_serialises_to_lowercase_string() {
        assertEquals("\"apartment\"", json.encodeToString(PropertyCategory.APARTMENT))
        assertEquals("\"house\"", json.encodeToString(PropertyCategory.HOUSE))
        assertEquals("\"land\"", json.encodeToString(PropertyCategory.LAND))
    }

    @Test
    fun listingStatus_round_trips_through_serial_names() {
        for (status in ListingStatus.entries) {
            val encoded = json.encodeToString(status)
            val decoded: ListingStatus = json.decodeFromString(encoded)
            assertEquals(status, decoded)
        }
    }

    @Test
    fun listingSortOption_uses_underscore_serial_names() {
        assertEquals("\"price_asc\"", json.encodeToString(ListingSortOption.PRICE_ASC))
        assertEquals("\"price_desc\"", json.encodeToString(ListingSortOption.PRICE_DESC))
        assertEquals("\"area_asc\"", json.encodeToString(ListingSortOption.AREA_ASC))
        assertEquals("\"newest\"", json.encodeToString(ListingSortOption.NEWEST))
    }

    // -------------------------------------------------------------------
    // Address / nested decoding
    // -------------------------------------------------------------------

    @Test
    fun address_uses_postal_code_snake_case() {
        val raw =
            """
            {
              "street": "Hlavna 1",
              "city": "Bratislava",
              "postal_code": "81101",
              "country": "SK"
            }
            """
                .trimIndent()
        val addr: Address = json.decodeFromString(raw)
        assertEquals("Hlavna 1", addr.street)
        assertEquals("Bratislava", addr.city)
        assertEquals("81101", addr.postalCode)
        assertEquals("SK", addr.country)
        assertNull(addr.coordinates)
    }

    @Test
    fun address_decodes_optional_coordinates() {
        val raw =
            """
            {
              "city": "Bratislava",
              "country": "SK",
              "coordinates": { "latitude": 48.15, "longitude": 17.10 }
            }
            """
                .trimIndent()
        val addr: Address = json.decodeFromString(raw)
        val coords = assertNotNull(addr.coordinates)
        assertEquals(48.15, coords.latitude)
        assertEquals(17.10, coords.longitude)
    }

    // -------------------------------------------------------------------
    // ListingSummary
    // -------------------------------------------------------------------

    @Test
    fun listingSummary_decodes_minimal_api_payload() {
        val raw =
            """
            {
              "id": "lst-1",
              "title": "Cosy 2br",
              "type": "sale",
              "category": "apartment",
              "status": "active",
              "price": 250000,
              "address": { "city": "Bratislava", "country": "SK" },
              "created_at": "2026-04-26T10:00:00Z",
              "updated_at": "2026-04-26T10:00:00Z"
            }
            """
                .trimIndent()
        val summary: ListingSummary = json.decodeFromString(raw)

        assertEquals("lst-1", summary.id)
        assertEquals(ListingType.SALE, summary.type)
        assertEquals(PropertyCategory.APARTMENT, summary.category)
        assertEquals(ListingStatus.ACTIVE, summary.status)
        assertEquals(250_000L, summary.price)
        assertEquals("EUR", summary.currency) // default
        assertEquals(0, summary.imageCount) // default
        assertNull(summary.primaryImage)
        assertNull(summary.realtor)
    }

    @Test
    fun listingSummary_maps_snake_case_flags() {
        val raw =
            """
            {
              "id": "lst-2",
              "title": "Penthouse",
              "type": "rent",
              "category": "apartment",
              "status": "active",
              "price": 1500,
              "currency": "USD",
              "price_per_sqm": 25,
              "area_sqm": 60.5,
              "address": { "city": "Bratislava", "country": "SK" },
              "primary_image": {
                "id": "img-1",
                "url": "https://example.com/a.jpg",
                "thumbnail_url": "https://example.com/a-thumb.jpg",
                "is_primary": true
              },
              "image_count": 5,
              "is_featured": true,
              "is_new": false,
              "is_price_reduced": true,
              "created_at": "2026-04-26T10:00:00Z",
              "updated_at": "2026-04-26T10:00:00Z"
            }
            """
                .trimIndent()
        val summary: ListingSummary = json.decodeFromString(raw)

        assertEquals(25L, summary.pricePerSqm)
        assertEquals(60.5, summary.areaSqm)
        assertEquals(5, summary.imageCount)
        assertTrue(summary.isFeatured)
        assertTrue(summary.isPriceReduced)

        val primary = assertNotNull(summary.primaryImage)
        assertTrue(primary.isPrimary)
        assertEquals("https://example.com/a-thumb.jpg", primary.thumbnailUrl)
    }

    // -------------------------------------------------------------------
    // ListingSearchResponse / pagination
    // -------------------------------------------------------------------

    @Test
    fun listingSearchResponse_maps_pagination_fields() {
        val raw =
            """
            {
              "listings": [],
              "total": 0,
              "page": 1,
              "page_size": 20,
              "total_pages": 0
            }
            """
                .trimIndent()
        val response: ListingSearchResponse = json.decodeFromString(raw)
        assertEquals(20, response.pageSize)
        assertEquals(0, response.totalPages)
        assertTrue(response.listings.isEmpty())
    }

    // -------------------------------------------------------------------
    // ListingSearchRequest defaults round-trip
    // -------------------------------------------------------------------

    @Test
    fun listingSearchRequest_defaults_are_omitted_with_encodeDefaults_false() {
        val request = ListingSearchRequest(query = "bratislava")
        val encoded = json.encodeToString(request)

        // With encodeDefaults = false, page/page_size/sort defaults should not be emitted.
        assertTrue(encoded.contains("\"query\":\"bratislava\""))
        assertTrue(
            !encoded.contains("\"page_size\""),
            "page_size default should be omitted: $encoded",
        )
        assertTrue(!encoded.contains("\"sort\""), "sort default should be omitted: $encoded")
    }

    @Test
    fun listingSearchRequest_renders_filters_with_snake_case_keys() {
        val request =
            ListingSearchRequest(
                query = null,
                filters =
                    ListingSearchFilters(
                        type = ListingType.SALE,
                        category = PropertyCategory.APARTMENT,
                        minPrice = 100_000,
                        maxPrice = 500_000,
                        minRooms = 2,
                        radiusKm = 5.0,
                        nearLat = 48.15,
                        nearLng = 17.10,
                    ),
            )
        val encoded = json.encodeToString(request)

        // SerialName keys present:
        assertTrue(encoded.contains("\"min_price\":100000"))
        assertTrue(encoded.contains("\"max_price\":500000"))
        assertTrue(encoded.contains("\"min_rooms\":2"))
        assertTrue(encoded.contains("\"radius_km\":5.0"))
        assertTrue(encoded.contains("\"near_lat\":48.15"))
        assertTrue(encoded.contains("\"near_lng\":17.1"))
    }

    @Test
    fun listingSearchRequest_round_trips() {
        val request =
            ListingSearchRequest(
                query = "cosy",
                filters = ListingSearchFilters(category = PropertyCategory.HOUSE, minRooms = 3),
                sort = ListingSortOption.PRICE_ASC,
                page = 2,
                pageSize = 50,
            )
        val decoded: ListingSearchRequest = json.decodeFromString(json.encodeToString(request))
        assertEquals(request, decoded)
    }

    // -------------------------------------------------------------------
    // SearchSuggestions
    // -------------------------------------------------------------------

    @Test
    fun searchSuggestionsResponse_maps_recent_searches_field() {
        val raw =
            """
            {
              "cities": [
                { "city": "Bratislava", "country": "SK", "listing_count": 42 }
              ],
              "regions": ["BA", "KE"],
              "recent_searches": ["centrum", "ruzinov"]
            }
            """
                .trimIndent()
        val resp: SearchSuggestionsResponse = json.decodeFromString(raw)
        assertEquals(1, resp.cities.size)
        assertEquals(42, resp.cities[0].listingCount)
        assertEquals(listOf("centrum", "ruzinov"), resp.recentSearches)
    }

    // -------------------------------------------------------------------
    // ListingDetail full round-trip
    // -------------------------------------------------------------------

    @Test
    fun listingDetail_round_trips_through_json() {
        val detail =
            ListingDetail(
                id = "lst-99",
                title = "Title",
                description = "Desc",
                type = ListingType.SALE,
                category = PropertyCategory.HOUSE,
                status = ListingStatus.ACTIVE,
                price = 1_000_000,
                areaSqm = 120.0,
                rooms = 4,
                bedrooms = 3,
                bathrooms = 2,
                yearBuilt = 1990,
                address =
                    Address(
                        street = "Hlavna 1",
                        city = "Bratislava",
                        postalCode = "81101",
                        country = "SK",
                    ),
                images =
                    listOf(ListingImage(id = "img-1", url = "https://x/a.jpg", isPrimary = true)),
                features = listOf("balcony", "elevator"),
                createdAt = "2026-04-26T10:00:00Z",
                updatedAt = "2026-04-26T10:00:00Z",
            )

        val decoded: ListingDetail = json.decodeFromString(json.encodeToString(detail))
        assertEquals(detail, decoded)
    }
}
