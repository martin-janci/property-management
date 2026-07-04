package three.two.bit.ppt.reality.listing

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
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.Json
import three.two.bit.ppt.reality.favorites.FavoritesRepository
import three.two.bit.ppt.reality.inquiry.InquiryRepository

/**
 * Unit tests for [ListingDetailViewModel]'s optimistic favorite toggle.
 *
 * Pins the gap-82-4 behavior that made `ListingDetailScreen` a churn hotspot (issue #2079): the
 * heart flips immediately (optimistic), then reconciles with the server — keeping the new value on
 * success and rolling back to the previous value on failure. Previously this logic lived inline in
 * the composable and was pinned only by a byte-identity script, not an executable test.
 *
 * The favorites HTTP layer is driven through a real [FavoritesRepository] over a Ktor [MockEngine]
 * (same harness as `FavoritesToggleRepositoryTest`). The view-model runs on the `runTest`
 * [StandardTestDispatcher], so launched work is deferred until [advanceUntilIdle], letting us assert
 * the optimistic intermediate state before the server responds.
 */
class ListingDetailViewModelTest {

    private val json = Json {
        ignoreUnknownKeys = true
        isLenient = true
        encodeDefaults = true
    }

    /** A FavoritesRepository whose every request returns [status] with [body]. */
    private fun favoritesRepo(status: HttpStatusCode, body: String): FavoritesRepository {
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

    /** Repositories that are constructed but never exercised in these toggle tests. */
    private fun unusedListingRepo(): ListingRepository {
        val engine = MockEngine { respond(content = ByteReadChannel(""), status = HttpStatusCode.OK) }
        return ListingRepository(
            baseUrl = "https://example.test",
            client = HttpClient(engine) { install(ContentNegotiation) { json(json) } },
        )
    }

    private fun unusedInquiryRepo(): InquiryRepository {
        val engine = MockEngine { respond(content = ByteReadChannel(""), status = HttpStatusCode.OK) }
        return InquiryRepository(
            baseUrl = "https://example.test",
            client = HttpClient(engine) { install(ContentNegotiation) { json(json) } },
        )
    }

    private fun viewModel(
        favorites: FavoritesRepository,
        scope: kotlinx.coroutines.CoroutineScope,
        isAuthenticated: Boolean = true,
    ) =
        ListingDetailViewModel(
            listingId = "lst-1",
            listingRepository = unusedListingRepo(),
            favoritesRepository = favorites,
            inquiryRepository = unusedInquiryRepo(),
            scope = scope,
            isAuthenticated = isAuthenticated,
            errorMapper = { it.message ?: "error" },
        )

    @Test
    fun favoriteToggle_success_keeps_optimistic_true() = runTest(StandardTestDispatcher()) {
        val repo =
            favoritesRepo(
                HttpStatusCode.Created,
                """{"id":"fav-1","listing_id":"lst-1","created_at":"2026-04-26T10:00:00Z"}""",
            )
        val vm = viewModel(repo, scope = backgroundScope)

        vm.onFavoriteToggle()

        // Optimistic: heart is already on and a write is in flight before the server answers.
        assertTrue(vm.state.value.isFavorite, "heart should flip on immediately (optimistic)")
        assertTrue(vm.state.value.isFavoriteLoading)

        advanceUntilIdle()

        // 201 → the optimistic value is confirmed, spinner cleared.
        assertTrue(vm.state.value.isFavorite, "successful add keeps the heart on")
        assertFalse(vm.state.value.isFavoriteLoading)
    }

    @Test
    fun favoriteToggle_failure_rolls_back_to_previous() = runTest(StandardTestDispatcher()) {
        // 500 → repository returns Result.failure, so the toggle must roll back.
        val repo = favoritesRepo(HttpStatusCode.InternalServerError, """{"error":"boom"}""")
        val vm = viewModel(repo, scope = backgroundScope)

        vm.onFavoriteToggle()

        // Optimistic flip applied first.
        assertTrue(vm.state.value.isFavorite)
        assertTrue(vm.state.value.isFavoriteLoading)

        advanceUntilIdle()

        // Rollback: heart returns to its previous (off) state and the spinner clears.
        assertFalse(vm.state.value.isFavorite, "failed add must roll back to previous value")
        assertFalse(vm.state.value.isFavoriteLoading)
    }

    @Test
    fun favoriteToggle_ignored_when_not_authenticated() = runTest(StandardTestDispatcher()) {
        val repo =
            favoritesRepo(
                HttpStatusCode.Created,
                """{"id":"fav-1","listing_id":"lst-1","created_at":"2026-04-26T10:00:00Z"}""",
            )
        val vm = viewModel(repo, scope = backgroundScope, isAuthenticated = false)

        vm.onFavoriteToggle()
        advanceUntilIdle()

        assertFalse(vm.state.value.isFavorite, "unauthenticated toggle must be a no-op")
        assertFalse(vm.state.value.isFavoriteLoading)
    }

    @Test
    fun favoriteToggle_is_reentrancy_guarded_while_in_flight() = runTest(StandardTestDispatcher()) {
        val repo =
            favoritesRepo(
                HttpStatusCode.Created,
                """{"id":"fav-1","listing_id":"lst-1","created_at":"2026-04-26T10:00:00Z"}""",
            )
        val vm = viewModel(repo, scope = backgroundScope)

        vm.onFavoriteToggle() // starts add → optimistic true, loading
        vm.onFavoriteToggle() // must be ignored while the first write is in flight
        assertTrue(vm.state.value.isFavorite)

        advanceUntilIdle()

        // Only the first toggle took effect; state settled on favorited.
        assertTrue(vm.state.value.isFavorite)
        assertFalse(vm.state.value.isFavoriteLoading)
    }
}
