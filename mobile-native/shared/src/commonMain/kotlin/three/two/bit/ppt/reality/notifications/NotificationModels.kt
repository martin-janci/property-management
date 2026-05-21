package three.two.bit.ppt.reality.notifications

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

/**
 * Notification models for Reality Portal mobile app.
 *
 * Epic 48 - Story 48.4: Portal Mobile Alerts
 */

/** Notification type. */
@Serializable
enum class NotificationType {
    @SerialName("new_listing") NEW_LISTING,
    @SerialName("price_drop") PRICE_DROP,
    @SerialName("inquiry_response") INQUIRY_RESPONSE,
    @SerialName("listing_update") LISTING_UPDATE,
    @SerialName("favorite_sold") FAVORITE_SOLD,
    @SerialName("system") SYSTEM,
}

/** Notification entry. */
@Serializable
data class NotificationEntry(
    val id: String,
    val type: NotificationType,
    val title: String,
    val body: String,
    @SerialName("listing_id") val listingId: String? = null,
    @SerialName("inquiry_id") val inquiryId: String? = null,
    val data: Map<String, String> = emptyMap(),
    @SerialName("is_read") val isRead: Boolean = false,
    @SerialName("created_at") val createdAt: String,
)

/** User notifications response. */
@Serializable
data class NotificationsResponse(
    val notifications: List<NotificationEntry>,
    val total: Int,
    @SerialName("unread_count") val unreadCount: Int,
)

/** Push notification token registration request. */
@Serializable
data class RegisterPushTokenRequest(
    val token: String,
    val platform: String, // "android" or "ios"
    @SerialName("device_id") val deviceId: String? = null,
)

/** Push notification token registration response. */
@Serializable
data class RegisterPushTokenResponse(val success: Boolean, val message: String? = null)

/** Notification preferences. */
@Serializable
data class NotificationPreferences(
    @SerialName("new_listings") val newListings: Boolean = true,
    @SerialName("price_drops") val priceDrops: Boolean = true,
    @SerialName("inquiry_responses") val inquiryResponses: Boolean = true,
    @SerialName("listing_updates") val listingUpdates: Boolean = true,
    @SerialName("marketing") val marketing: Boolean = false,
)

/** Update notification preferences request. */
@Serializable
data class UpdateNotificationPreferencesRequest(val preferences: NotificationPreferences)

/** Alert configuration for a saved search. */
@Serializable
data class AlertConfig(
    val id: String,
    @SerialName("search_id") val searchId: String,
    val enabled: Boolean = true,
    val frequency: AlertFrequency = AlertFrequency.INSTANT,
    @SerialName("created_at") val createdAt: String,
)

/** Alert frequency. */
@Serializable
enum class AlertFrequency {
    @SerialName("instant") INSTANT,
    @SerialName("daily") DAILY,
    @SerialName("weekly") WEEKLY,
}

/** Update alert config request. */
@Serializable
data class UpdateAlertConfigRequest(
    val enabled: Boolean? = null,
    val frequency: AlertFrequency? = null,
)
