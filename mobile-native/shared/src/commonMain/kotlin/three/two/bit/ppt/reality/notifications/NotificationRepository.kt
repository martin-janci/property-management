package three.two.bit.ppt.reality.notifications

import io.ktor.client.*
import io.ktor.client.call.*
import io.ktor.client.request.*
import io.ktor.http.*
import three.two.bit.ppt.reality.api.HttpClientProvider
import three.two.bit.ppt.reality.api.asPathSegment
import three.two.bit.ppt.reality.favorites.FavoriteAlert
import three.two.bit.ppt.reality.favorites.FavoriteAlertsResponse

/**
 * Repository for notifications and alerts.
 *
 * Epic 48 - Story 48.4: Portal Mobile Alerts
 */
class NotificationRepository(
    private val baseUrl: String,
    private val sessionToken: String? = null,
    private val client: HttpClient = HttpClientProvider.client,
) {

    private fun HttpRequestBuilder.configureRequest() {
        sessionToken?.let { header(HttpHeaders.Authorization, "Bearer $it") }
    }

    // --- Notifications ---
    //
    // reality-server has no generic `/notifications` inbox. The in-app notification feed it does
    // have is the favorite-alerts feed (`/api/v1/favorites/alerts`: price changes and status
    // changes on saved listings), so the inbox calls are served from there. Push tokens and
    // preferences live under `/api/v1/users/me`.

    /** Get the user's notifications (favorite alerts, newest first). */
    suspend fun getNotifications(
        page: Int = 1,
        pageSize: Int = 20,
        unreadOnly: Boolean = false,
    ): Result<NotificationsResponse> {
        return try {
            val response =
                client.get("$baseUrl/api/v1/favorites/alerts") {
                    configureRequest()
                    parameter("limit", pageSize)
                    parameter("offset", (page.coerceAtLeast(1) - 1) * pageSize)
                }

            if (response.status.isSuccess()) {
                val wire = response.body<FavoriteAlertsResponse>()
                // The server has no unread filter; apply it to the fetched page.
                val entries =
                    wire.alerts.map { it.toNotification() }.filter { !unreadOnly || !it.isRead }
                Result.success(
                    NotificationsResponse(
                        notifications = entries,
                        total = entries.size,
                        unreadCount = wire.unreadCount.coerceAtMost(Int.MAX_VALUE.toLong()).toInt(),
                    ),
                )
            } else if (response.status == HttpStatusCode.Unauthorized) {
                Result.failure(NotificationException("Please sign in to view notifications"))
            } else {
                Result.failure(
                    NotificationException("Failed to load notifications: ${response.status}"),
                )
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /**
     * Get unread notification count (`unread_count` of the favorite-alerts feed). A signed-out
     * caller (401) gets 0 so a badge never errors.
     */
    suspend fun getUnreadCount(): Result<Int> {
        return try {
            val response =
                client.get("$baseUrl/api/v1/favorites/alerts") {
                    configureRequest()
                    parameter("limit", 1)
                }

            if (response.status.isSuccess()) {
                val unread = response.body<FavoriteAlertsResponse>().unreadCount
                Result.success(unread.coerceAtMost(Int.MAX_VALUE.toLong()).toInt())
            } else if (response.status == HttpStatusCode.Unauthorized) {
                Result.success(0)
            } else {
                Result.failure(
                    NotificationException("Failed to load unread count: ${response.status}"),
                )
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /** Mark notification as read. */
    suspend fun markAsRead(notificationId: String): Result<Unit> {
        return try {
            val response =
                client.post(
                    "$baseUrl/api/v1/favorites/alerts/${notificationId.asPathSegment()}/read",
                ) {
                    configureRequest()
                }

            if (response.status.isSuccess()) {
                Result.success(Unit)
            } else {
                Result.failure(NotificationException("Failed to mark as read: ${response.status}"))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /** Mark all notifications as read. */
    suspend fun markAllAsRead(): Result<Unit> {
        return try {
            val response =
                client.post("$baseUrl/api/v1/favorites/alerts/read-all") { configureRequest() }

            if (response.status.isSuccess()) {
                Result.success(Unit)
            } else {
                Result.failure(
                    NotificationException("Failed to mark all as read: ${response.status}"),
                )
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /** Delete (dismiss) a notification (`DELETE /api/v1/favorites/alerts/{id}`). */
    suspend fun deleteNotification(notificationId: String): Result<Unit> {
        return try {
            val response =
                client.delete(
                    "$baseUrl/api/v1/favorites/alerts/${notificationId.asPathSegment()}",
                ) {
                    configureRequest()
                }

            if (response.status.isSuccess()) {
                Result.success(Unit)
            } else {
                Result.failure(
                    NotificationException("Failed to delete notification: ${response.status}"),
                )
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    // --- Push Notifications ---

    /**
     * Register (or refresh) a device push token (`POST /api/v1/users/me/push-tokens`). `platform`
     * is `android` or `ios`; the server stores them as `fcm` / `apns`.
     */
    suspend fun registerPushToken(
        token: String,
        platform: String,
        deviceId: String? = null,
    ): Result<RegisterPushTokenResponse> {
        return try {
            val response =
                client.post("$baseUrl/api/v1/users/me/push-tokens") {
                    configureRequest()
                    contentType(ContentType.Application.Json)
                    setBody(RegisterPushTokenRequest(token, platform, deviceId))
                }

            if (response.status.isSuccess()) {
                Result.success(RegisterPushTokenResponse(success = true))
            } else {
                Result.failure(
                    NotificationException("Failed to register push token: ${response.status}"),
                )
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /** Unregister a device push token (`DELETE /api/v1/users/me/push-tokens/{token}`). */
    suspend fun unregisterPushToken(token: String): Result<Unit> {
        return try {
            val response =
                client.delete("$baseUrl/api/v1/users/me/push-tokens/${token.asPathSegment()}") {
                    configureRequest()
                }

            if (response.status.isSuccess()) {
                Result.success(Unit)
            } else {
                Result.failure(
                    NotificationException("Failed to unregister push token: ${response.status}"),
                )
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    // --- Preferences ---

    /** Notification preferences (`GET /api/v1/users/me/notification-preferences`). */
    suspend fun getPreferences(): Result<NotificationPreferences> {
        return try {
            val response =
                client.get("$baseUrl/api/v1/users/me/notification-preferences") {
                    configureRequest()
                }

            if (response.status.isSuccess()) {
                Result.success(response.body<NotificationPreferences>())
            } else {
                Result.failure(
                    NotificationException("Failed to load preferences: ${response.status}"),
                )
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /** Update notification preferences (`PUT /api/v1/users/me/notification-preferences`). */
    suspend fun updatePreferences(
        preferences: NotificationPreferences,
    ): Result<NotificationPreferences> {
        return try {
            val response =
                client.put("$baseUrl/api/v1/users/me/notification-preferences") {
                    configureRequest()
                    contentType(ContentType.Application.Json)
                    setBody(preferences)
                }

            if (response.status.isSuccess()) {
                Result.success(response.body<NotificationPreferences>())
            } else {
                Result.failure(
                    NotificationException("Failed to update preferences: ${response.status}"),
                )
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
}

private fun FavoriteAlert.toNotification(): NotificationEntry =
    NotificationEntry(
        id = id,
        type =
            when {
                isPriceDrop -> NotificationType.PRICE_DROP
                newStatus == "sold" || newStatus == "rented" -> NotificationType.FAVORITE_SOLD
                else -> NotificationType.LISTING_UPDATE
            },
        title = title,
        body =
            when {
                isPriceChange && oldPrice != null && newPrice != null ->
                    "$oldPrice → $newPrice ${currency.orEmpty()}".trim()
                newStatus != null -> "${previousStatus.orEmpty()} → $newStatus".trim()
                else -> ""
            },
        listingId = listingId,
        isRead = !isUnread,
        createdAt = createdAt,
    )

/** Notification-specific exception. */
class NotificationException(message: String) : Exception(message)
