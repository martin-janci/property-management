package three.two.bit.ppt.reality.favorites

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.serialization.decodeFromString
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

/**
 * Contract tests for the favorites + saved-search DTOs.
 *
 * Pins the JSON shape that the reality-server returns for the /api/v1/favorites and
 * /api/v1/saved-searches endpoints so the client-side @SerialName annotations can't drift
 * unnoticed.
 */
class FavoritesModelsContractTest {

    private val json = Json {
        ignoreUnknownKeys = true
        encodeDefaults = false
    }

    @Test
    fun favoriteEntry_decodes_snake_case_listing_id_and_user_id() {
        val raw =
            """
            {
              "id": "fav-1",
              "listing_id": "lst-1",
              "user_id": "u-1",
              "created_at": "2026-04-26T10:00:00Z"
            }
            """
                .trimIndent()
        val entry: FavoriteEntry = json.decodeFromString(raw)
        assertEquals("lst-1", entry.listingId)
        assertEquals("u-1", entry.userId)
        assertNull(entry.listing)
    }

    @Test
    fun favoritesResponse_maps_total_alongside_entries() {
        val raw =
            """
            {
              "favorites": [
                {
                  "id": "fav-1",
                  "listing_id": "lst-1",
                  "user_id": "u-1",
                  "created_at": "2026-04-26T10:00:00Z"
                },
                {
                  "id": "fav-2",
                  "listing_id": "lst-2",
                  "user_id": "u-1",
                  "created_at": "2026-04-26T11:00:00Z"
                }
              ],
              "total": 2
            }
            """
                .trimIndent()
        val response: FavoritesResponse = json.decodeFromString(raw)
        assertEquals(2, response.total)
        assertEquals(2, response.favorites.size)
        assertEquals("fav-2", response.favorites[1].id)
    }

    @Test
    fun addFavoriteRequest_encodes_listing_id_in_snake_case() {
        val request = AddFavoriteRequest(listingId = "lst-42")
        val encoded = json.encodeToString(request)
        assertTrue(
            encoded.contains("\"listing_id\":\"lst-42\""),
            "missing snake_case key: $encoded"
        )
        assertTrue(!encoded.contains("listingId"), "leaked camelCase key: $encoded")
    }

    @Test
    fun addFavoriteResponse_decodes_minimal_payload() {
        val raw =
            """
            {
              "id": "fav-1",
              "listing_id": "lst-1",
              "created_at": "2026-04-26T10:00:00Z"
            }
            """
                .trimIndent()
        val resp: AddFavoriteResponse = json.decodeFromString(raw)
        assertEquals("fav-1", resp.id)
        assertEquals("lst-1", resp.listingId)
    }

    @Test
    fun savedSearch_maps_alert_enabled_and_new_count() {
        val raw =
            """
            {
              "id": "ss-1",
              "name": "Bratislava 2br",
              "query": "bratislava",
              "filters": {
                "type": "rent",
                "city": "Bratislava",
                "min_price": 500,
                "max_price": 1500,
                "min_rooms": 2
              },
              "alert_enabled": true,
              "created_at": "2026-04-26T10:00:00Z",
              "last_notified_at": "2026-04-26T11:00:00Z",
              "new_count": 7
            }
            """
                .trimIndent()
        val ss: SavedSearch = json.decodeFromString(raw)
        assertEquals("ss-1", ss.id)
        assertTrue(ss.alertEnabled)
        assertEquals(7, ss.newCount)
        assertEquals("2026-04-26T11:00:00Z", ss.lastNotifiedAt)
        val filters = assertNotNull(ss.filters)
        assertEquals(500L, filters.minPrice)
        assertEquals(1500L, filters.maxPrice)
        assertEquals(2, filters.minRooms)
    }

    @Test
    fun savedSearch_defaults_when_optional_fields_missing() {
        val raw =
            """
            {
              "id": "ss-2",
              "name": "Default search",
              "created_at": "2026-04-26T10:00:00Z"
            }
            """
                .trimIndent()
        val ss: SavedSearch = json.decodeFromString(raw)
        assertEquals(false, ss.alertEnabled) // default
        assertEquals(0, ss.newCount) // default
        assertNull(ss.query)
        assertNull(ss.filters)
        assertNull(ss.lastNotifiedAt)
    }

    @Test
    fun createSavedSearchRequest_round_trips_with_filters() {
        val req =
            CreateSavedSearchRequest(
                name = "Penthouses",
                query = "penthouse",
                filters = SavedSearchFilters(type = "sale", city = "Bratislava", minRooms = 3),
                alertEnabled = true
            )
        val encoded = json.encodeToString(req)
        assertTrue(encoded.contains("\"alert_enabled\":true"))
        assertTrue(encoded.contains("\"min_rooms\":3"))

        val decoded: CreateSavedSearchRequest = json.decodeFromString(encoded)
        assertEquals(req, decoded)
    }

    @Test
    fun updateSavedSearchRequest_omits_null_fields_with_encodeDefaults_false() {
        val req = UpdateSavedSearchRequest(alertEnabled = false)
        val encoded = json.encodeToString(req)

        // `name` is null and should not appear in the payload.
        assertTrue(!encoded.contains("\"name\""), "name should be omitted: $encoded")
        assertTrue(encoded.contains("\"alert_enabled\":false"))
    }

    @Test
    fun savedSearchesResponse_round_trips() {
        val resp =
            SavedSearchesResponse(
                searches =
                    listOf(
                        SavedSearch(
                            id = "ss-1",
                            name = "A",
                            alertEnabled = true,
                            createdAt = "2026-04-26T10:00:00Z"
                        )
                    ),
                total = 1
            )
        val decoded: SavedSearchesResponse = json.decodeFromString(json.encodeToString(resp))
        assertEquals(resp, decoded)
    }
}
