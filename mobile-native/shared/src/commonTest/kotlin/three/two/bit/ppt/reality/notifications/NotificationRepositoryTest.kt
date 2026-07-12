package three.two.bit.ppt.reality.notifications

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
 * Repository-level coroutine tests for [NotificationRepository] — the fetch + mark-read paths that
 * feed the Reality Portal mobile alerts screen (Epic 48 / Story 48.4).
 *
 * The [NotificationModelsContractTest] pins the raw DTO encode/decode; this pins the *behaviour
 * contract* as seen through the repository:
 * - `GET /api/v1/notifications` on 200 decodes a [NotificationsResponse] (so the alerts list can
 *   render title/body/type/unread-count),
 * - a 401 on the read paths maps to the user-facing "Please sign in …" [NotificationException]
 *   while `getUnreadCount` degrades a 401 to a silent `0` (badge stays hidden for signed-out
 *   users),
 * - `markAsRead` POSTs to `/{id}/read` and surfaces a non-2xx as a [NotificationException].
 */
class NotificationRepositoryTest {

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
    ): NotificationRepository {
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
        return NotificationRepository(
            baseUrl = "https://example.test",
            sessionToken = "test-token",
            client = client,
        )
    }

    private val notificationsBody =
        """
        {
          "notifications": [
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
          ],
          "total": 1,
          "unread_count": 1
        }
        """
            .trimIndent()

    // -------------------------------------------------------------------
    // getNotifications — fetch + DTO mapping
    // -------------------------------------------------------------------

    @Test
    fun getNotifications_gets_endpoint_and_decodes_entries() = runTest {
        val cap = Captured()
        val repo = repoCapturing(cap, HttpStatusCode.OK, notificationsBody)

        val result = repo.getNotifications()

        assertTrue(result.isSuccess, "expected success, got $result")
        val response = assertNotNull(result.getOrNull())

        // Endpoint contract.
        assertEquals(HttpMethod.Get, cap.method)
        assertEquals("/api/v1/notifications", cap.path)

        // DTO-mapping: the fields the alerts list binds must all decode.
        assertEquals(1, response.total)
        assertEquals(1, response.unreadCount)
        assertEquals(1, response.notifications.size)
        val entry = response.notifications.first()
        assertEquals("n-1", entry.id)
        assertEquals(NotificationType.PRICE_DROP, entry.type)
        assertEquals("Price drop", entry.title)
        assertEquals("lst-1", entry.listingId)
        assertEquals("275000", entry.data["new_price"])
        assertEquals(false, entry.isRead)
    }

    @Test
    fun getNotifications_maps_401_to_sign_in_failure() = runTest {
        val cap = Captured()
        val repo =
            repoCapturing(
                cap,
                HttpStatusCode.Unauthorized,
                """{"error":"unauthenticated"}""",
            )

        val result = repo.getNotifications()

        assertTrue(result.isFailure, "401 should surface as failure, got $result")
        val ex = result.exceptionOrNull()
        assertTrue(
            ex is NotificationException,
            "expected NotificationException, got ${ex?.let { it::class }}",
        )
        assertEquals("Please sign in to view notifications", ex?.message)
    }

    @Test
    fun getNotifications_maps_server_error_to_failure() = runTest {
        val cap = Captured()
        val repo = repoCapturing(cap, HttpStatusCode.InternalServerError, """{"error":"boom"}""")

        val result = repo.getNotifications()

        assertTrue(result.isFailure, "5xx should surface as failure, got $result")
        assertTrue(result.exceptionOrNull() is NotificationException)
    }

    // -------------------------------------------------------------------
    // getUnreadCount — body extraction + 401 degrade-to-zero
    // -------------------------------------------------------------------

    @Test
    fun getUnreadCount_reads_count_from_body() = runTest {
        val cap = Captured()
        val repo = repoCapturing(cap, HttpStatusCode.OK, """{"count": 7}""")

        val result = repo.getUnreadCount()

        assertTrue(result.isSuccess, "expected success, got $result")
        assertEquals(7, result.getOrNull())
        assertEquals(HttpMethod.Get, cap.method)
        assertEquals("/api/v1/notifications/unread-count", cap.path)
    }

    @Test
    fun getUnreadCount_degrades_401_to_zero() = runTest {
        val cap = Captured()
        val repo =
            repoCapturing(
                cap,
                HttpStatusCode.Unauthorized,
                """{"error":"unauthenticated"}""",
            )

        val result = repo.getUnreadCount()

        assertTrue(result.isSuccess, "401 should degrade to Result.success(0), got $result")
        assertEquals(0, result.getOrNull())
    }

    // -------------------------------------------------------------------
    // markAsRead — mutation branch
    // -------------------------------------------------------------------

    @Test
    fun markAsRead_posts_to_read_endpoint_on_success() = runTest {
        val cap = Captured()
        val repo = repoCapturing(cap, HttpStatusCode.OK, "{}")

        val result = repo.markAsRead("n-1")

        assertTrue(result.isSuccess, "expected success, got $result")
        assertEquals(HttpMethod.Post, cap.method)
        assertEquals("/api/v1/notifications/n-1/read", cap.path)
    }

    @Test
    fun markAsRead_maps_non_2xx_to_failure() = runTest {
        val cap = Captured()
        val repo = repoCapturing(cap, HttpStatusCode.NotFound, """{"error":"missing"}""")

        val result = repo.markAsRead("nope")

        assertTrue(result.isFailure, "non-2xx should surface as failure, got $result")
        assertTrue(result.exceptionOrNull() is NotificationException)
    }
}
