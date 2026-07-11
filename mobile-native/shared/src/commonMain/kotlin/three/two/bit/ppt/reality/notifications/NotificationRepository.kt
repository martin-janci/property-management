package three.two.bit.ppt.reality.notifications

import io.ktor.client.*
import io.ktor.client.call.*
import io.ktor.client.request.*
import io.ktor.http.*
import three.two.bit.ppt.reality.api.HttpClientProvider
import three.two.bit.ppt.reality.api.asPathSegment

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

    /** Get user's notifications. */
    suspend fun getNotifications(
        page: Int = 1,
        pageSize: Int = 20,
        unreadOnly: Boolean = false,
    ): Result<NotificationsResponse> {
        return try {
            val response =
                client.get("$baseUrl/api/v1/notifications") {
                    configureRequest()
                    parameter("page", page)
                    parameter("page_size", pageSize)
                    if (unreadOnly) parameter("unread_only", true)
                }

            if (response.status.isSuccess()) {
                Result.success(response.body())
            } else if (response.status == HttpStatusCode.Unauthorized) {
                Result.failure(NotificationException("Please sign in to view notifications"))
            } else {
                Result.failure(
                    NotificationException("Failed to load notifications: ${response.status}")
                )
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /** Get unread notification count. */
    suspend fun getUnreadCount(): Result<Int> {
        return try {
            val response =
                client.get("$baseUrl/api/v1/notifications/unread-count") { configureRequest() }

            if (response.status.isSuccess()) {
                val data: Map<String, Int> = response.body()
                Result.success(data["count"] ?: 0)
            } else if (response.status == HttpStatusCode.Unauthorized) {
                Result.success(0)
            } else {
                Result.failure(
                    NotificationException("Failed to get unread count: ${response.status}")
                )
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /** Mark a notification as read. */
    suspend fun markAsRead(notificationId: String): Result<Unit> {
        return try {
            val response =
                client.post(
                    "$baseUrl/api/v1/notifications/${notificationId.asPathSegment()}/read"
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
                client.post("$baseUrl/api/v1/notifications/read-all") { configureRequest() }

            if (response.status.isSuccess()) {
                Result.success(Unit)
            } else {
                Result.failure(
                    NotificationException("Failed to mark all as read: ${response.status}")
                )
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /** Delete a notification. */
    suspend fun deleteNotification(notificationId: String): Result<Unit> {
        return try {
            val response =
                client.delete("$baseUrl/api/v1/notifications/${notificationId.asPathSegment()}") {
                    configureRequest()
                }

            if (response.status.isSuccess()) {
                Result.success(Unit)
            } else {
                Result.failure(
                    NotificationException("Failed to delete notification: ${response.status}")
                )
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    // --- Push Token ---

    /** Register push notification token. */
    suspend fun registerPushToken(
        token: String,
        platform: String,
        deviceId: String? = null,
    ): Result<RegisterPushTokenResponse> {
        return try {
            val response =
                client.post("$baseUrl/api/v1/notifications/push-token") {
                    configureRequest()
                    setBody(RegisterPushTokenRequest(token, platform, deviceId))
                }

            if (response.status.isSuccess()) {
                Result.success(response.body())
            } else {
                Result.failure(
                    NotificationException("Failed to register push token: ${response.status}")
                )
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /** Unregister push notification token. */
    suspend fun unregisterPushToken(token: String): Result<Unit> {
        return try {
            val response =
                client.delete("$baseUrl/api/v1/notifications/push-token") {
                    configureRequest()
                    parameter("token", token)
                }

            if (response.status.isSuccess()) {
                Result.success(Unit)
            } else {
                Result.failure(
                    NotificationException("Failed to unregister push token: ${response.status}")
                )
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    // --- Preferences ---

    /** Get notification preferences. */
    suspend fun getPreferences(): Result<NotificationPreferences> {
        return try {
            val response =
                client.get("$baseUrl/api/v1/notifications/preferences") { configureRequest() }

            if (response.status.isSuccess()) {
                Result.success(response.body())
            } else if (response.status == HttpStatusCode.Unauthorized) {
                Result.failure(NotificationException("Please sign in to view preferences"))
            } else {
                Result.failure(
                    NotificationException("Failed to load preferences: ${response.status}")
                )
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    /** Update notification preferences. */
    suspend fun updatePreferences(
        preferences: NotificationPreferences
    ): Result<NotificationPreferences> {
        return try {
            val response =
                client.put("$baseUrl/api/v1/notifications/preferences") {
                    configureRequest()
                    setBody(UpdateNotificationPreferencesRequest(preferences))
                }

            if (response.status.isSuccess()) {
                Result.success(response.body())
            } else {
                Result.failure(
                    NotificationException("Failed to update preferences: ${response.status}")
                )
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
}

/** Notification-specific exception. */
class NotificationException(message: String) : Exception(message)
