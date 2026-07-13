package three.two.bit.ppt.reality.favorites

import io.ktor.client.HttpClient
import io.ktor.client.engine.mock.MockEngine
import io.ktor.client.engine.mock.respond
import io.ktor.client.plugins.contentnegotiation.ContentNegotiation
import io.ktor.http.HttpHeaders
import io.ktor.http.HttpStatusCode
import io.ktor.http.headersOf
import io.ktor.utils.io.ByteReadChannel
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.Json

/**
 * Unit tests for [PriceTrackingViewModel] — the mobile price-tracking surface added in gap-84-3.
 *
 * The favorites HTTP layer is driven through a real [FavoritesRepository] over a path-routing Ktor
 * [MockEngine], mirroring the [ListingDetailViewModel] test harness. The view-model runs on the
 * `runTest` [StandardTestDispatcher] with `backgroundScope`; because [MockEngine] resumes calls off
 * the virtual scheduler, completion is awaited by suspending on the observable outcome via
 * `state.first { … }` rather than `advanceUntilIdle()`.
 */
class PriceTrackingViewModelTest {

    private val json = Json {
        ignoreUnknownKeys = true
        isLenient = true
        encodeDefaults = true
    }

    private val favoritesBody =
        """
        {
          "favorites": [
            {
              "id": "fav-1",
              "listing_id": "lst-1",
              "created_at": "2026-07-01T10:00:00Z",
              "title": "Bright 2br in Old Town",
              "current_price": 185000,
              "currency": "EUR",
              "city": "Bratislava",
              "price_changed": true,
              "price_change_percentage": -7.5,
              "price_alert_enabled": true
            },
            {
              "id": "fav-2",
              "listing_id": "lst-2",
              "created_at": "2026-07-02T10:00:00Z",
              "title": "Studio near the river",
              "current_price": 120000,
              "currency": "EUR",
              "city": "Kosice",
              "price_changed": false,
              "price_alert_enabled": false
            }
          ]
        }
        """
            .trimIndent()

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
              "old_price": 200000,
              "new_price": 185000,
              "currency": "EUR",
              "change_percentage": -7.5,
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
              "created_at": "2026-07-09T09:00:00Z"
            }
          ],
          "unread_count": 1,
          "limit": 100,
          "offset": 0
        }
        """
            .trimIndent()

    /**
     * A [FavoritesRepository] whose responses are routed by request path, so a single instance can
     * serve the favorites load, the alerts load, and the mark-read writes the view-model fires.
     */
    private fun routingRepo(
        token: String? = "test-token",
        readStatus: HttpStatusCode = HttpStatusCode.NoContent,
    ): FavoritesRepository {
        val engine = MockEngine { request ->
            val path = request.url.encodedPath
            val jsonHeaders = headersOf(HttpHeaders.ContentType, "application/json")
            when {
                path.endsWith("/favorites/alerts/read-all") ->
                    respond("""{"marked_read": 1}""", HttpStatusCode.OK, jsonHeaders)
                path.endsWith("/read") -> respond("", readStatus, jsonHeaders)
                path.endsWith("/favorites/alerts") ->
                    respond(alertsBody, HttpStatusCode.OK, jsonHeaders)
                path.endsWith("/favorites") -> respond(favoritesBody, HttpStatusCode.OK, jsonHeaders)
                else -> respond("{}", HttpStatusCode.OK, jsonHeaders)
            }
        }
        val client = HttpClient(engine) { install(ContentNegotiation) { json(json) } }
        return FavoritesRepository(baseUrl = "https://example.test", sessionToken = token, client = client)
    }

    private fun viewModel(
        repo: FavoritesRepository,
        scope: CoroutineScope,
        token: String? = "test-token",
    ) =
        PriceTrackingViewModel(
            favoritesRepositoryFactory = { repo },
            scope = scope,
            initialSessionToken = token,
            errorMapper = { it.message ?: "error" },
        )

    @Test
    fun start_loads_favorites_alerts_and_derives_price_changes() =
        runTest(StandardTestDispatcher()) {
            val vm = viewModel(routingRepo(), scope = backgroundScope)
            vm.start()

            vm.state.first { !it.isLoading }
            val s = vm.state.value

            assertEquals(2, s.favorites.size)
            assertEquals(2, s.alerts.size)
            assertEquals(1L, s.unreadCount)
            // Derived views used by the surface.
            assertEquals(1, s.priceChangedFavorites.size, "only fav-1 has price_changed=true")
            assertEquals("lst-1", s.priceChangedFavorites.first().listingId)
            assertEquals(1, s.priceAlertEnabledCount)
            assertEquals(1, s.priceAlerts.size, "one price_change alert, one status_change alert")
            assertFalse(s.isEmpty)
            assertTrue(s.errorMessage == null)
        }

    @Test
    fun markAlertRead_is_optimistic_and_decrements_unread() =
        runTest(StandardTestDispatcher()) {
            val vm = viewModel(routingRepo(), scope = backgroundScope)
            vm.start()
            vm.state.first { !it.isLoading }
            assertEquals(1L, vm.state.value.unreadCount)

            vm.onMarkAlertRead("al-1")

            // Optimistic: alert flips to read and the unread badge drops immediately.
            val optimistic = vm.state.value
            assertEquals(0L, optimistic.unreadCount)
            assertFalse(optimistic.alerts.first { it.id == "al-1" }.isUnread)

            vm.state.first { !it.isMutating }
            // 204 confirms — stays read.
            assertEquals(0L, vm.state.value.unreadCount)
            assertFalse(vm.state.value.alerts.first { it.id == "al-1" }.isUnread)
        }

    @Test
    fun markAlertRead_rolls_back_on_failure() =
        runTest(StandardTestDispatcher()) {
            val repo = routingRepo(readStatus = HttpStatusCode.InternalServerError)
            val vm = viewModel(repo, scope = backgroundScope)
            vm.start()
            vm.state.first { !it.isLoading }

            vm.onMarkAlertRead("al-1")
            // Optimistic drop first…
            assertEquals(0L, vm.state.value.unreadCount)

            vm.state.first { !it.isMutating }
            // …then rollback to the pre-write state on the 500.
            assertEquals(1L, vm.state.value.unreadCount, "failed mark-read must roll back the badge")
            assertTrue(vm.state.value.alerts.first { it.id == "al-1" }.isUnread)
        }

    @Test
    fun markAllRead_clears_unread_optimistically() =
        runTest(StandardTestDispatcher()) {
            val vm = viewModel(routingRepo(), scope = backgroundScope)
            vm.start()
            vm.state.first { !it.isLoading }

            vm.onMarkAllRead()
            assertEquals(0L, vm.state.value.unreadCount)
            assertTrue(vm.state.value.alerts.none { it.isUnread })

            vm.state.first { !it.isMutating }
            assertEquals(0L, vm.state.value.unreadCount)
        }

    @Test
    fun start_when_unauthenticated_renders_signed_out_empty_state() =
        runTest(StandardTestDispatcher()) {
            val vm = viewModel(routingRepo(token = null), scope = backgroundScope, token = null)
            vm.start()

            // No network is fired; state resolves synchronously to the signed-out empty state.
            val s = vm.state.value
            assertFalse(s.isLoading)
            assertFalse(s.isAuthenticated)
            assertTrue(s.favorites.isEmpty())
            assertTrue(s.alerts.isEmpty())
            assertEquals(0L, s.unreadCount)
        }
}
