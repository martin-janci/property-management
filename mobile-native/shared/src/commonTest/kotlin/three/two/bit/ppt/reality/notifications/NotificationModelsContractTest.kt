package three.two.bit.ppt.reality.notifications

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.serialization.decodeFromString
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

/**
 * Contract tests for the notifications + push-token + alerts DTOs.
 *
 * Pins the JSON shape that the reality-server uses for the notifications surface and the
 * saved-search alert configuration.
 */
class NotificationModelsContractTest {

    private val json = Json {
        ignoreUnknownKeys = true
        encodeDefaults = false
    }

    // -------------------------------------------------------------------
    // NotificationType
    // -------------------------------------------------------------------

    @Test
    fun notificationType_uses_snake_case_serial_names() {
        assertEquals("\"new_listing\"", json.encodeToString(NotificationType.NEW_LISTING))
        assertEquals("\"price_drop\"", json.encodeToString(NotificationType.PRICE_DROP))
        assertEquals("\"inquiry_response\"", json.encodeToString(NotificationType.INQUIRY_RESPONSE))
        assertEquals("\"listing_update\"", json.encodeToString(NotificationType.LISTING_UPDATE))
        assertEquals("\"favorite_sold\"", json.encodeToString(NotificationType.FAVORITE_SOLD))
        assertEquals("\"system\"", json.encodeToString(NotificationType.SYSTEM))
    }

    @Test
    fun notificationType_round_trips() {
        for (t in NotificationType.entries) {
            val decoded: NotificationType = json.decodeFromString(json.encodeToString(t))
            assertEquals(t, decoded)
        }
    }

    @Test
    fun alertFrequency_uses_lowercase_serial_names() {
        assertEquals("\"instant\"", json.encodeToString(AlertFrequency.INSTANT))
        assertEquals("\"daily\"", json.encodeToString(AlertFrequency.DAILY))
        assertEquals("\"weekly\"", json.encodeToString(AlertFrequency.WEEKLY))
    }

    // -------------------------------------------------------------------
    // NotificationEntry / NotificationsResponse
    // -------------------------------------------------------------------

    @Test
    fun notificationEntry_decodes_full_payload() {
        val raw =
            """
            {
              "id": "n-1",
              "type": "price_drop",
              "title": "Price drop",
              "body": "Your favorite has a new price",
              "listing_id": "lst-1",
              "data": { "old_price": "300000", "new_price": "275000" },
              "is_read": false,
              "created_at": "2026-04-26T10:00:00Z"
            }
            """
                .trimIndent()
        val entry: NotificationEntry = json.decodeFromString(raw)
        assertEquals("n-1", entry.id)
        assertEquals(NotificationType.PRICE_DROP, entry.type)
        assertEquals("lst-1", entry.listingId)
        assertEquals("300000", entry.data["old_price"])
        assertEquals(false, entry.isRead)
    }

    @Test
    fun notificationEntry_decodes_with_default_data_and_no_inquiry() {
        val raw =
            """
            {
              "id": "n-2",
              "type": "system",
              "title": "Welcome",
              "body": "Hello",
              "is_read": true,
              "created_at": "2026-04-26T10:00:00Z"
            }
            """
                .trimIndent()
        val entry: NotificationEntry = json.decodeFromString(raw)
        assertTrue(entry.data.isEmpty())
        assertNull(entry.listingId)
        assertNull(entry.inquiryId)
        assertEquals(true, entry.isRead)
    }

    @Test
    fun notificationsResponse_maps_unread_count() {
        val raw =
            """
            {
              "notifications": [],
              "total": 0,
              "unread_count": 0
            }
            """
                .trimIndent()
        val response: NotificationsResponse = json.decodeFromString(raw)
        assertEquals(0, response.unreadCount)
        assertEquals(0, response.total)
    }

    // -------------------------------------------------------------------
    // Push token registration
    // -------------------------------------------------------------------

    @Test
    fun registerPushTokenRequest_encodes_device_id_in_snake_case() {
        val request =
            RegisterPushTokenRequest(
                token = "expo-push-token",
                platform = "ios",
                deviceId = "device-uuid-123"
            )
        val encoded = json.encodeToString(request)
        assertTrue(
            encoded.contains("\"device_id\":\"device-uuid-123\""),
            "missing snake_case: $encoded"
        )
        assertTrue(encoded.contains("\"platform\":\"ios\""))
    }

    @Test
    fun registerPushTokenRequest_omits_null_device_id() {
        val request = RegisterPushTokenRequest(token = "tok", platform = "android")
        val encoded = json.encodeToString(request)
        assertTrue(!encoded.contains("device_id"), "device_id should be omitted: $encoded")
    }

    @Test
    fun registerPushTokenResponse_decodes_success_only() {
        val raw = """{ "success": true }"""
        val response: RegisterPushTokenResponse = json.decodeFromString(raw)
        assertEquals(true, response.success)
        assertNull(response.message)
    }

    // -------------------------------------------------------------------
    // NotificationPreferences
    // -------------------------------------------------------------------

    @Test
    fun notificationPreferences_use_snake_case_keys() {
        val prefs =
            NotificationPreferences(
                newListings = true,
                priceDrops = false,
                inquiryResponses = true,
                listingUpdates = true,
                marketing = false
            )
        val encoded = json.encodeToString(prefs)
        assertTrue(encoded.contains("\"new_listings\":true"))
        assertTrue(encoded.contains("\"price_drops\":false"))
        assertTrue(encoded.contains("\"inquiry_responses\":true"))
        assertTrue(encoded.contains("\"listing_updates\":true"))
    }

    @Test
    fun notificationPreferences_round_trip() {
        val prefs =
            NotificationPreferences(
                newListings = false,
                priceDrops = true,
                inquiryResponses = false,
                listingUpdates = true,
                marketing = true
            )
        val decoded: NotificationPreferences = json.decodeFromString(json.encodeToString(prefs))
        assertEquals(prefs, decoded)
    }

    // -------------------------------------------------------------------
    // AlertConfig
    // -------------------------------------------------------------------

    @Test
    fun alertConfig_decodes_search_id_and_frequency() {
        val raw =
            """
            {
              "id": "a-1",
              "search_id": "ss-1",
              "enabled": true,
              "frequency": "daily",
              "created_at": "2026-04-26T10:00:00Z"
            }
            """
                .trimIndent()
        val config: AlertConfig = json.decodeFromString(raw)
        assertEquals("a-1", config.id)
        assertEquals("ss-1", config.searchId)
        assertEquals(AlertFrequency.DAILY, config.frequency)
        assertEquals(true, config.enabled)
    }

    @Test
    fun updateAlertConfigRequest_omits_null_fields() {
        val request = UpdateAlertConfigRequest(enabled = false)
        val encoded = json.encodeToString(request)
        assertTrue(encoded.contains("\"enabled\":false"))
        assertTrue(!encoded.contains("frequency"), "frequency should be omitted: $encoded")
    }
}
