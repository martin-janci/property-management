package three.two.bit.ppt.reality.favorites

import io.ktor.client.HttpClient
import io.ktor.client.engine.mock.MockEngine
import io.ktor.client.engine.mock.respond
import io.ktor.client.plugins.contentnegotiation.ContentNegotiation
import io.ktor.http.HttpHeaders
import io.ktor.http.HttpStatusCode
import io.ktor.http.headersOf
import io.ktor.serialization.kotlinx.json.json
import io.ktor.utils.io.ByteReadChannel
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.Json

/**
 * Repository-level coroutine tests for the favorite price-tracking alert calls added in gap-84-3:
 * [FavoritesRepository.getFavoriteAlerts], [FavoritesRepository.markFavoriteAlertRead] and
 * [FavoritesRepository.markAllFavoriteAlertsRead].
 *
 * Pins the reality-server contract for `/api/v1/favorites/alerts*`:
 * - the list endpoint returns `{alerts, unread_count, limit, offset}` (200);
 * - `.../{id}/read` returns 204 (idempotent — already-read still 204);
 * - `.../read-all` returns 200 `{marked_read}`;
 * - 401 surfaces as a [FavoritesException] so the UI can prompt sign-in.
 */
class FavoriteAlertsRepositoryTest {

    private val json = Json {
        ignoreUnknownKeys = true
        isLenient = true
        encodeDefaults = true
    }

    private fun repoWith(status: HttpStatusCode, body: String): FavoritesRepository {
        val engine = MockEngine {
            respond(
                content = ByteReadChannel(body),
                status = status,
                headers = headersOf(HttpHeaders.ContentType, "application/json"),
            )
        }
        val client = HttpClient(engine) { install(ContentNegotiation) { json(json) } }
        return FavoritesRepository(
            baseUrl = "https://example.test",
            sessionToken = "test-token",
            client = client,
        )
    }

    private val alertsBody =
        """
        {
          "alerts": [
            {
              "id": "al-1",
              "favorite_id": "fav-1",
              "listing_id": "lst-1",
              "title": "Bright 2br in Old Town",
              "alert_type": "price_change",
              "old_price": "200000.00",
              "new_price": "185000.00",
              "currency": "EUR",
              "change_percentage": "-7.50",
              "status": "pending",
              "created_at": "2026-07-10T10:00:00Z"
            },
            {
              "id": "al-2",
              "favorite_id": "fav-2",
              "listing_id": "lst-2",
              "title": "Studio near the river",
              "alert_type": "status_change",
              "previous_status": "active",
              "new_status": "sold",
              "status": "sent",
              "created_at": "2026-07-09T09:00:00Z",
              "processed_at": "2026-07-09T09:05:00Z"
            }
          ],
          "unread_count": 1,
          "limit": 100,
          "offset": 0
        }
        """
            .trimIndent()

    @Test
    fun getFavoriteAlerts_decodes_alerts_and_unread_count() = runTest {
        val repo = repoWith(HttpStatusCode.OK, alertsBody)
        val result = repo.getFavoriteAlerts()
        assertTrue(result.isSuccess, "expected success, got $result")
        val body = result.getOrNull()!!
        assertEquals(2, body.alerts.size)
        assertEquals(1L, body.unreadCount)

        val price = body.alerts[0]
        assertEquals("al-1", price.id)
        assertTrue(price.isPriceChange, "al-1 is a price_change alert")
        assertTrue(price.isUnread, "al-1 status is pending → unread")
        assertTrue(price.isPriceDrop, "al-1 has a negative change_percentage → drop")
        assertEquals(200000L, price.oldPrice)
        assertEquals(185000L, price.newPrice)
        assertEquals(-7.5, price.changePercentage)

        val status = body.alerts[1]
        assertFalse(status.isPriceChange, "al-2 is a status_change alert")
        assertFalse(status.isUnread, "al-2 status is sent → read")
        assertEquals("sold", status.newStatus)
    }

    @Test
    fun getFavoriteAlerts_maps_unauthorized_to_failure() = runTest {
        val repo = repoWith(HttpStatusCode.Unauthorized, """{"error":"unauthenticated"}""")
        val result = repo.getFavoriteAlerts()
        assertTrue(result.isFailure, "401 should surface as failure, got $result")
        assertTrue(result.exceptionOrNull() is FavoritesException)
    }

    @Test
    fun markFavoriteAlertRead_returns_success_on_204() = runTest {
        val repo = repoWith(HttpStatusCode.NoContent, "")
        val result = repo.markFavoriteAlertRead("al-1")
        assertTrue(result.isSuccess, "204 should map to success, got $result")
    }

    @Test
    fun markFavoriteAlertRead_returns_failure_on_404() = runTest {
        val repo = repoWith(HttpStatusCode.NotFound, """{"error":"not found"}""")
        val result = repo.markFavoriteAlertRead("al-x")
        assertTrue(result.isFailure, "404 should surface as failure, got $result")
        assertTrue(result.exceptionOrNull() is FavoritesException)
    }

    @Test
    fun markAllFavoriteAlertsRead_decodes_marked_read_count() = runTest {
        val repo = repoWith(HttpStatusCode.OK, """{"marked_read": 3}""")
        val result = repo.markAllFavoriteAlertsRead()
        assertTrue(result.isSuccess, "expected success, got $result")
        assertEquals(3L, result.getOrNull()!!.markedRead)
    }
}
