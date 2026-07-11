package three.two.bit.ppt.reality.listing

import io.ktor.client.HttpClient
import io.ktor.client.engine.mock.MockEngine
import io.ktor.client.engine.mock.respond
import io.ktor.client.plugins.contentnegotiation.ContentNegotiation
import io.ktor.client.plugins.defaultRequest
import io.ktor.http.ContentType
import io.ktor.http.HttpHeaders
import io.ktor.http.HttpStatusCode
import io.ktor.http.contentType
import io.ktor.http.headersOf
import io.ktor.serialization.kotlinx.json.json
import io.ktor.utils.io.ByteReadChannel
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.async
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runCurrent
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
 * [StandardTestDispatcher], so launched work is deferred until the scheduler runs, letting us
 * assert the optimistic intermediate state before the server responds.
 *
 * NOTE on waiting: [MockEngine] resumes suspended HTTP calls on a real IO thread, so
 * [advanceUntilIdle] does NOT reliably wait for a round-trip — the vm coroutine may still be parked
 * off the virtual scheduler when it returns (surfaced by #2125 once the host-test task was
 * enabled). Completion is therefore awaited by suspending on the observable outcome itself via
 * `state.first { … }`, which `runTest` supports deterministically.
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

    /** A ListingRepository whose every request returns [status] with [body]. */
    private fun listingRepo(status: HttpStatusCode, body: String): ListingRepository {
        val engine = MockEngine {
            respond(
                content = ByteReadChannel(body),
                status = status,
                headers = headersOf(HttpHeaders.ContentType, "application/json"),
            )
        }
        return ListingRepository(
            baseUrl = "https://example.test",
            client = HttpClient(engine) { install(ContentNegotiation) { json(json) } },
        )
    }

    /** An InquiryRepository whose every request returns [status] with [body]. */
    private fun inquiryRepo(status: HttpStatusCode, body: String): InquiryRepository {
        val engine = MockEngine {
            respond(
                content = ByteReadChannel(body),
                status = status,
                headers = headersOf(HttpHeaders.ContentType, "application/json"),
            )
        }
        return InquiryRepository(
            baseUrl = "https://example.test",
            sessionToken = "test-token",
            client =
                HttpClient(engine) {
                    install(ContentNegotiation) { json(json) }
                    // The production HttpClientProvider sets this too; without it Ktor cannot
                    // serialize the @Serializable request body passed to setBody().
                    defaultRequest { contentType(ContentType.Application.Json) }
                },
        )
    }

    /** Repositories that are constructed but never exercised in a given test. */
    private fun unusedListingRepo(): ListingRepository = listingRepo(HttpStatusCode.OK, "")

    private fun unusedInquiryRepo(): InquiryRepository = inquiryRepo(HttpStatusCode.OK, "")

    /**
     * A minimal-but-valid `ListingDetail` JSON body the repository can decode — only the fields
     * required by the `ListingDetail` data class plus a couple the render binds.
     */
    private val listingBody =
        """
        {
          "id": "lst-1",
          "title": "Bright 3br with terrace",
          "description": "A lovely flat in Old Town.",
          "type": "sale",
          "category": "apartment",
          "status": "active",
          "price": 285000,
          "area_sqm": 75.0,
          "address": { "street": "Hlavna 1", "city": "Bratislava",
            "postal_code": "81101", "country": "SK" },
          "created_at": "2026-04-26T10:00:00Z",
          "updated_at": "2026-04-27T09:00:00Z"
        }
        """
            .trimIndent()

    private fun viewModel(
        favorites: FavoritesRepository,
        scope: kotlinx.coroutines.CoroutineScope,
        isAuthenticated: Boolean = true,
        listing: ListingRepository = unusedListingRepo(),
        inquiry: InquiryRepository = unusedInquiryRepo(),
    ) =
        ListingDetailViewModel(
            listingId = "lst-1",
            listingRepository = listing,
            favoritesRepositoryFactory = { favorites },
            inquiryRepositoryFactory = { inquiry },
            scope = scope,
            initialSessionToken = if (isAuthenticated) "test-token" else null,
            errorMapper = { it.message ?: "error" },
        )

    @Test
    fun favoriteToggle_success_keeps_optimistic_true() =
        runTest(StandardTestDispatcher()) {
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

            vm.state.first { !it.isFavoriteLoading }

            // 201 → the optimistic value is confirmed, spinner cleared.
            assertTrue(vm.state.value.isFavorite, "successful add keeps the heart on")
            assertFalse(vm.state.value.isFavoriteLoading)
        }

    @Test
    fun favoriteToggle_failure_rolls_back_to_previous() =
        runTest(StandardTestDispatcher()) {
            // 500 → repository returns Result.failure, so the toggle must roll back.
            val repo = favoritesRepo(HttpStatusCode.InternalServerError, """{"error":"boom"}""")
            val vm = viewModel(repo, scope = backgroundScope)

            vm.onFavoriteToggle()

            // Optimistic flip applied first.
            assertTrue(vm.state.value.isFavorite)
            assertTrue(vm.state.value.isFavoriteLoading)

            vm.state.first { !it.isFavoriteLoading }

            // Rollback: heart returns to its previous (off) state and the spinner clears.
            assertFalse(vm.state.value.isFavorite, "failed add must roll back to previous value")
            assertFalse(vm.state.value.isFavoriteLoading)
        }

    @Test
    fun favoriteToggle_ignored_when_not_authenticated() =
        runTest(StandardTestDispatcher()) {
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
    fun favoriteToggle_is_reentrancy_guarded_while_in_flight() =
        runTest(StandardTestDispatcher()) {
            val repo =
                favoritesRepo(
                    HttpStatusCode.Created,
                    """{"id":"fav-1","listing_id":"lst-1","created_at":"2026-04-26T10:00:00Z"}""",
                )
            val vm = viewModel(repo, scope = backgroundScope)

            vm.onFavoriteToggle() // starts add → optimistic true, loading
            vm.onFavoriteToggle() // must be ignored while the first write is in flight
            assertTrue(vm.state.value.isFavorite)

            vm.state.first { !it.isFavoriteLoading }

            // Only the first toggle took effect; state settled on favorited.
            assertTrue(vm.state.value.isFavorite)
            assertFalse(vm.state.value.isFavoriteLoading)
        }

    // ─── start(): initial listing load ────────────────────────────────────────

    @Test
    fun start_loads_listing_and_clears_spinner() =
        runTest(StandardTestDispatcher()) {
            val vm =
                viewModel(
                    favorites = favoritesRepo(HttpStatusCode.OK, "{}"),
                    scope = backgroundScope,
                    isAuthenticated = false,
                    listing = listingRepo(HttpStatusCode.OK, listingBody),
                )

            // Default state is the full-screen spinner until start() resolves.
            assertTrue(vm.state.value.isLoading)
            assertNull(vm.state.value.listing)

            vm.start()
            vm.state.first { !it.isLoading }

            val listing = assertNotNull(vm.state.value.listing, "listing should be loaded")
            assertEquals("lst-1", listing.id)
            assertFalse(vm.state.value.isLoading, "spinner clears once the listing arrives")
            assertNull(vm.state.value.errorMessage)
        }

    @Test
    fun start_maps_error_then_retry_recovers() =
        runTest(StandardTestDispatcher()) {
            // A mock backend that 500s first, then is flipped to 200 before retry().
            var status = HttpStatusCode.InternalServerError
            var body = """{"error":"boom"}"""
            val engine = MockEngine {
                respond(
                    content = ByteReadChannel(body),
                    status = status,
                    headers = headersOf(HttpHeaders.ContentType, "application/json"),
                )
            }
            val repo =
                ListingRepository(
                    baseUrl = "https://example.test",
                    client = HttpClient(engine) { install(ContentNegotiation) { json(json) } },
                )
            val vm =
                viewModel(
                    favorites = favoritesRepo(HttpStatusCode.OK, "{}"),
                    scope = backgroundScope,
                    isAuthenticated = false,
                    listing = repo,
                )

            vm.start()
            vm.state.first { it.errorMessage != null }

            assertNull(vm.state.value.listing, "a failed load leaves no listing")
            assertNotNull(vm.state.value.errorMessage, "5xx surfaces an error message")
            assertFalse(vm.state.value.isLoading)

            // Backend recovers → retry() reloads the listing and clears the error.
            status = HttpStatusCode.OK
            body = listingBody
            vm.retry()
            vm.state.first { it.listing != null }

            assertNotNull(vm.state.value.listing, "retry recovers the listing")
            assertNull(vm.state.value.errorMessage, "error cleared on successful retry")
            assertFalse(vm.state.value.isLoading)
        }

    // ─── updateAuth(): session changes (issue #2125) ─────────────────────────

    /**
     * A FavoritesRepository whose `/check` probe returns `{"is_favorited": isFavorited}` and whose
     * add/remove writes return [writeStatus] — lets one repo drive both the probe and the toggle.
     */
    private fun favoritesRepoCheckAndWrite(
        isFavorited: Boolean,
        writeStatus: HttpStatusCode = HttpStatusCode.InternalServerError,
        writeBody: String = """{"error":"boom"}""",
    ): FavoritesRepository {
        val engine = MockEngine { request ->
            if (request.url.encodedPath.endsWith("/check")) {
                respond(
                    content = ByteReadChannel("""{"is_favorited":$isFavorited}"""),
                    status = HttpStatusCode.OK,
                    headers = headersOf(HttpHeaders.ContentType, "application/json"),
                )
            } else {
                respond(
                    content = ByteReadChannel(writeBody),
                    status = writeStatus,
                    headers = headersOf(HttpHeaders.ContentType, "application/json"),
                )
            }
        }
        val client = HttpClient(engine) { install(ContentNegotiation) { json(json) } }
        return FavoritesRepository(
            baseUrl = "https://example.test",
            sessionToken = "test-token",
            client = client,
        )
    }

    @Test
    fun updateAuth_newToken_keeps_listing_and_does_not_respin() =
        runTest(StandardTestDispatcher()) {
            val vm =
                viewModel(
                    favorites = favoritesRepoCheckAndWrite(isFavorited = false),
                    scope = backgroundScope,
                    listing = listingRepo(HttpStatusCode.OK, listingBody),
                )
            vm.start()
            vm.state.first { it.listing != null }

            // Session token changes (re-login) → the listing must survive untouched and the
            // screen must NOT flip back to the full-screen spinner (the #2108 regression).
            vm.updateAuth("new-token")
            assertNotNull(vm.state.value.listing, "auth change must not clear the loaded listing")
            assertFalse(vm.state.value.isLoading, "auth change must not restart the spinner")
        }

    @Test
    fun updateAuth_logout_clears_favorite_state() =
        runTest(StandardTestDispatcher()) {
            // Authenticated start(): the /check probe reports the listing as favorited.
            val vm =
                viewModel(
                    favorites = favoritesRepoCheckAndWrite(isFavorited = true),
                    scope = backgroundScope,
                    listing = listingRepo(HttpStatusCode.OK, listingBody),
                )
            vm.start()
            vm.state.first { it.isFavorite && it.listing != null }

            vm.updateAuth(null) // logout — clears the heart synchronously

            assertFalse(vm.state.value.isFavorite, "logout must clear the heart")
            assertFalse(vm.state.value.isFavoriteLoading)
            assertNotNull(vm.state.value.listing, "logout must not clear the loaded listing")
        }

    @Test
    fun updateAuth_login_reprobes_favorite_state() =
        runTest(StandardTestDispatcher()) {
            // Token-aware factory: unauthenticated at construction, favorited once logged in.
            val vm =
                ListingDetailViewModel(
                    listingId = "lst-1",
                    listingRepository = listingRepo(HttpStatusCode.OK, listingBody),
                    favoritesRepositoryFactory = { favoritesRepoCheckAndWrite(isFavorited = true) },
                    inquiryRepositoryFactory = { unusedInquiryRepo() },
                    scope = backgroundScope,
                    initialSessionToken = null,
                    errorMapper = { it.message ?: "error" },
                )
            vm.start()
            vm.state.first { it.listing != null }
            assertFalse(vm.state.value.isFavorite, "unauthenticated start leaves the heart off")

            vm.updateAuth("test-token") // login
            vm.state.first { it.isFavorite } // login must re-probe and light the heart

            assertNotNull(vm.state.value.listing)
        }

    @Test
    fun updateAuth_sameToken_is_noop() =
        runTest(StandardTestDispatcher()) {
            var factoryCalls = 0
            val vm =
                ListingDetailViewModel(
                    listingId = "lst-1",
                    listingRepository = listingRepo(HttpStatusCode.OK, listingBody),
                    favoritesRepositoryFactory = {
                        factoryCalls++
                        favoritesRepoCheckAndWrite(isFavorited = true)
                    },
                    inquiryRepositoryFactory = { unusedInquiryRepo() },
                    scope = backgroundScope,
                    initialSessionToken = "test-token",
                    errorMapper = { it.message ?: "error" },
                )
            assertEquals(1, factoryCalls, "construction builds the repo once")

            vm.start()
            // Wait for BOTH async loads (listing + favorite probe) so the state is settled.
            vm.state.first { it.listing != null && it.isFavorite }
            val before = vm.state.value

            vm.updateAuth("test-token") // unchanged token → must be a complete no-op
            advanceUntilIdle()

            assertEquals(1, factoryCalls, "same-token updateAuth must not rebuild the repo")
            assertEquals(before, vm.state.value, "same-token updateAuth must not touch state")
        }

    /**
     * The #2125 correctness guard: a favorite write launched under one session must not write
     * `isFavorite` after the session has changed. Here the toggle-OFF write 500s, which would
     * normally roll the heart back to `true` — but the session switched mid-flight, so the guard
     * must skip the rollback and only clear the spinner. The session change is token→token
     * (re-login) rather than logout so the guard's effect is observable: on logout `updateAuth`
     * itself already resets the heart, masking the same code path.
     */
    @Test
    fun favoriteToggle_rollback_is_skipped_after_session_change_mid_flight() =
        runTest(StandardTestDispatcher()) {
            val vm =
                ListingDetailViewModel(
                    listingId = "lst-1",
                    listingRepository = listingRepo(HttpStatusCode.OK, listingBody),
                    favoritesRepositoryFactory = { token ->
                        if (token == "test-token") {
                            // Original session: favorited; the remove write will 500.
                            favoritesRepoCheckAndWrite(
                                isFavorited = true,
                                writeStatus = HttpStatusCode.InternalServerError,
                            )
                        } else {
                            // New session: not favorited.
                            favoritesRepoCheckAndWrite(isFavorited = false)
                        }
                    },
                    inquiryRepositoryFactory = { unusedInquiryRepo() },
                    scope = backgroundScope,
                    initialSessionToken = "test-token",
                    errorMapper = { it.message ?: "error" },
                )
            vm.start()
            vm.state.first { it.isFavorite }

            vm.onFavoriteToggle() // optimistic OFF, failing remove-write in flight
            assertFalse(vm.state.value.isFavorite)
            assertTrue(vm.state.value.isFavoriteLoading)

            vm.updateAuth("other-token") // session changes while the write is still in flight

            // The failing write completes and clears the spinner, but its rollback (back to
            // isFavorite=true) must be skipped — it belongs to the superseded session.
            vm.state.first { !it.isFavoriteLoading }
            assertFalse(
                vm.state.value.isFavorite,
                "a failed write from the superseded session must not resurrect the heart",
            )
        }

    // ─── onSubmitInquiry() ────────────────────────────────────────────────────

    @Test
    fun submitInquiry_success_emits_event_and_closes_dialog() =
        runTest(StandardTestDispatcher()) {
            val vm =
                viewModel(
                    favorites = favoritesRepo(HttpStatusCode.OK, "{}"),
                    scope = backgroundScope,
                    inquiry =
                        inquiryRepo(
                            HttpStatusCode.Created,
                            """{"id":"inq-1","listing_id":"lst-1","status":"pending",""" +
                                """"created_at":"2026-04-26T10:00:00Z"}""",
                        ),
                )
            // Await the first emitted event; subscribe (via runCurrent) BEFORE the submit — a
            // SharedFlow without replay drops emissions made while it has no subscribers.
            val firstEvent = backgroundScope.async { vm.events.first() }
            runCurrent()

            vm.onShowInquiryDialog()
            vm.onSubmitInquiry("Hello", "Jana", "jana@example.test", "+421900000000")

            // Optimistic: the submit spinner is on before the server answers.
            assertTrue(vm.state.value.isInquirySubmitting)

            vm.state.first { !it.isInquirySubmitting }

            assertFalse(vm.state.value.isInquirySubmitting, "spinner clears on success")
            assertFalse(vm.state.value.showInquiryDialog, "dialog closes on success")
            assertNull(vm.state.value.inquiryError)
            assertEquals(
                ListingDetailEvent.InquirySubmitted,
                firstEvent.await(),
                "a 201 must emit InquirySubmitted so the UI can navigate",
            )
        }

    @Test
    fun submitInquiry_failure_sets_error_and_keeps_dialog() =
        runTest(StandardTestDispatcher()) {
            val vm =
                viewModel(
                    favorites = favoritesRepo(HttpStatusCode.OK, "{}"),
                    scope = backgroundScope,
                    inquiry =
                        inquiryRepo(HttpStatusCode.InternalServerError, """{"error":"boom"}"""),
                )
            val events = mutableListOf<ListingDetailEvent>()
            backgroundScope.launch { vm.events.collect { events += it } }

            vm.onShowInquiryDialog()
            vm.onSubmitInquiry("Hello", null, null, null)
            vm.state.first { it.inquiryError != null }
            advanceUntilIdle()

            assertNotNull(vm.state.value.inquiryError, "5xx surfaces an inquiry error")
            assertFalse(vm.state.value.isInquirySubmitting, "spinner clears on failure")
            assertTrue(vm.state.value.showInquiryDialog, "dialog stays open so the user can retry")
            assertTrue(events.isEmpty(), "no success event on a failed submit")
        }
}
