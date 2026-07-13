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

    /**
     * Pin the flat PortalFavoriteWithListing shape returned by `GET /api/v1/favorites`. The server
     * embeds listing fields directly on the favorite row rather than nesting a full ListingSummary
     * — the KMP client must decode these flat fields so the FavoritesScreen can render cards
     * without extra requests.
     */
    @Test
    fun favoriteEntry_decodes_flat_portal_favorite_with_listing_shape() {
        val raw =
            """
            {
              "id": "fav-1",
              "listing_id": "00000000-0000-0000-0000-000000000001",
              "created_at": "2026-04-26T10:00:00Z",
              "title": "Bright 2br in Old Town",
              "current_price": 185000,
              "currency": "EUR",
              "city": "Bratislava",
              "property_type": "apartment",
              "transaction_type": "sale",
              "photo_url": "https://cdn.example.com/img/a.jpg",
              "status": "active",
              "price_changed": true,
              "price_change_percentage": -5.2,
              "price_alert_enabled": false
            }
            """
                .trimIndent()
        val entry: FavoriteEntry = json.decodeFromString(raw)
        assertEquals("Bright 2br in Old Town", entry.title)
        assertEquals(185_000L, entry.currentPrice)
        assertEquals("EUR", entry.currency)
        assertEquals("Bratislava", entry.city)
        assertEquals("sale", entry.transactionType)
        assertEquals("https://cdn.example.com/img/a.jpg", entry.photoUrl)
        assertTrue(entry.priceChanged)
        assertNull(entry.userId) // not in PortalFavoriteWithListing
        assertNull(entry.listing) // no nested listing on this endpoint
    }

    @Test
    fun checkFavoriteResponse_decodes_is_favorited_field() {
        val trueRaw = """{"is_favorited": true}"""
        val falseRaw = """{"is_favorited": false}"""
        val trueResp: CheckFavoriteResponse = json.decodeFromString(trueRaw)
        val falseResp: CheckFavoriteResponse = json.decodeFromString(falseRaw)
        assertTrue(trueResp.isFavorited)
        assertEquals(false, falseResp.isFavorited)
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
        assertEquals(2L, response.total)
        assertEquals(2, response.favorites.size)
        assertEquals("fav-2", response.favorites[1].id)
    }

    /**
     * `AddFavoriteRequest` was removed when the add-favorite endpoint was corrected to `POST
     * /api/v1/favorites/{listing_id}` (listing ID as path param). This test now verifies that
     * [AddFavoriteResponse] decodes the server's `PortalFavorite` 201 response, which is the only
     * wire-format concern remaining for the add flow.
     */
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
                alertEnabled = true,
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
                            createdAt = "2026-04-26T10:00:00Z",
                        )
                    ),
                total = 1,
            )
        val decoded: SavedSearchesResponse = json.decodeFromString(json.encodeToString(resp))
        assertEquals(resp, decoded)
    }

    /**
     * Pin the `FavoriteAlertsResponse` / `FavoriteAlert` snake_case wire shape returned by
     * `GET /api/v1/favorites/alerts` (gap-84-3). Monetary fields are whole-currency numbers and
     * `change_percentage` is a signed fraction — matching the rest of the portal price models.
     */
    @Test
    fun favoriteAlertsResponse_decodes_snake_case_price_change_shape() {
        val raw =
            """
            {
              "alerts": [
                {
                  "id": "al-1",
                  "favorite_id": "fav-1",
                  "listing_id": "lst-1",
                  "title": "Bright 2br in Old Town",
                  "alert_type": "price_change",
                  "old_price": 200000,
                  "new_price": 185000,
                  "currency": "EUR",
                  "change_percentage": -7.5,
                  "status": "pending",
                  "created_at": "2026-07-10T10:00:00Z"
                }
              ],
              "unread_count": 1,
              "limit": 100,
              "offset": 0
            }
            """
                .trimIndent()
        val resp: FavoriteAlertsResponse = json.decodeFromString(raw)
        assertEquals(1L, resp.unreadCount)
        assertEquals(1, resp.alerts.size)
        val alert = resp.alerts.first()
        assertEquals("fav-1", alert.favoriteId)
        assertEquals("lst-1", alert.listingId)
        assertEquals(200000L, alert.oldPrice)
        assertEquals(185000L, alert.newPrice)
        assertEquals(-7.5, alert.changePercentage)
        assertTrue(alert.isPriceChange)
        assertTrue(alert.isUnread)
        assertTrue(alert.isPriceDrop)
    }

    /** Status-only alerts omit the price fields entirely; they must still decode. */
    @Test
    fun favoriteAlert_decodes_status_change_without_price_fields() {
        val raw =
            """
            {
              "id": "al-2",
              "favorite_id": "fav-2",
              "listing_id": "lst-2",
              "title": "Studio near the river",
              "alert_type": "status_change",
              "previous_status": "active",
              "new_status": "sold",
              "status": "sent",
              "created_at": "2026-07-09T09:00:00Z"
            }
            """
                .trimIndent()
        val alert: FavoriteAlert = json.decodeFromString(raw)
        assertNull(alert.oldPrice)
        assertNull(alert.newPrice)
        assertNull(alert.changePercentage)
        assertEquals("sold", alert.newStatus)
        assertTrue(!alert.isPriceChange)
        assertTrue(!alert.isUnread)
    }
}
