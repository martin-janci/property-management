package three.two.bit.ppt.reality.favorites

import io.ktor.client.HttpClient
import io.ktor.client.engine.mock.MockEngine
import io.ktor.client.engine.mock.respond
import io.ktor.client.plugins.contentnegotiation.ContentNegotiation
import io.ktor.http.HttpHeaders
import io.ktor.http.HttpMethod
import io.ktor.http.HttpStatusCode
import io.ktor.http.headersOf
import io.ktor.utils.io.ByteReadChannel
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.Json

/**
 * Repository-level coroutine tests for the *write* half of the favorite toggle:
 * [FavoritesRepository.addFavorite] and [FavoritesRepository.removeFavorite].
 *
 * The existing [FavoritesRepositoryTest] pins the *read* half (`isFavorite`). The favorite toggle
 * on a listing-detail screen is add-or-remove driven by that read; the two write calls are what
 * actually flip server state, so their status-code contracts (201 decode, 409 Conflict, 401, 404,
 * 5xx) and the path/method/body shape need their own coverage. Verifies the gap-82-7 endpoint
 * contract: `POST/DELETE /api/v1/favorites/{listing_id}` with the id as a path segment.
 */
class FavoritesToggleRepositoryTest {

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
    ): FavoritesRepository {
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
        return FavoritesRepository(
            baseUrl = "https://example.test",
            sessionToken = "test-token",
            client = client,
        )
    }

    // --- addFavorite ---

    @Test
    fun addFavorite_posts_to_path_segment_and_decodes_201_portal_favorite() = runTest {
        val cap = Captured()
        val repo =
            repoCapturing(
                cap,
                HttpStatusCode.Created,
                """{"id":"fav-1","listing_id":"lst-1","created_at":"2026-04-26T10:00:00Z"}""",
            )

        val result = repo.addFavorite("lst-1")

        assertTrue(result.isSuccess, "expected success, got $result")
        assertEquals("fav-1", result.getOrNull()?.id)
        assertEquals("lst-1", result.getOrNull()?.listingId)
        // Endpoint contract: listing id is a path segment, POST, not a body field.
        assertEquals(HttpMethod.Post, cap.method)
        assertEquals("/api/v1/favorites/lst-1", cap.path)
    }

    @Test
    fun addFavorite_maps_409_conflict_to_already_in_favorites() = runTest {
        val cap = Captured()
        val repo = repoCapturing(cap, HttpStatusCode.Conflict, """{"error":"exists"}""")

        val result = repo.addFavorite("lst-1")

        assertTrue(result.isFailure, "409 should surface as failure, got $result")
        val ex = result.exceptionOrNull()
        assertTrue(ex is FavoritesException, "expected FavoritesException, got ${ex?.let { it::class }}")
        assertEquals("Already in favorites", ex?.message)
    }

    @Test
    fun addFavorite_maps_401_to_sign_in_prompt() = runTest {
        val cap = Captured()
        val repo = repoCapturing(cap, HttpStatusCode.Unauthorized, """{"error":"unauth"}""")

        val result = repo.addFavorite("lst-1")

        assertTrue(result.isFailure)
        assertEquals("Please sign in to save favorites", result.exceptionOrNull()?.message)
    }

    @Test
    fun addFavorite_url_encodes_listing_id_path_segment() = runTest {
        val cap = Captured()
        val repo =
            repoCapturing(
                cap,
                HttpStatusCode.Created,
                """{"id":"fav-1","listing_id":"x","created_at":"2026-04-26T10:00:00Z"}""",
            )

        repo.addFavorite("../admin/secret")

        // `/` must be percent-encoded; the request stays anchored under /favorites/.
        assertEquals("/api/v1/favorites/..%2Fadmin%2Fsecret", cap.path)
    }

    // --- removeFavorite ---

    @Test
    fun removeFavorite_deletes_path_segment_and_succeeds_on_2xx() = runTest {
        val cap = Captured()
        val repo = repoCapturing(cap, HttpStatusCode.NoContent, "")

        val result = repo.removeFavorite("lst-1")

        assertTrue(result.isSuccess, "expected success, got $result")
        assertEquals(HttpMethod.Delete, cap.method)
        assertEquals("/api/v1/favorites/lst-1", cap.path)
    }

    @Test
    fun removeFavorite_maps_404_to_favorite_not_found() = runTest {
        val cap = Captured()
        val repo = repoCapturing(cap, HttpStatusCode.NotFound, """{"error":"missing"}""")

        val result = repo.removeFavorite("lst-1")

        assertTrue(result.isFailure, "404 should surface as failure, got $result")
        assertEquals("Favorite not found", result.exceptionOrNull()?.message)
    }

    @Test
    fun removeFavorite_maps_401_to_sign_in_prompt() = runTest {
        val cap = Captured()
        val repo = repoCapturing(cap, HttpStatusCode.Unauthorized, """{"error":"unauth"}""")

        val result = repo.removeFavorite("lst-1")

        assertTrue(result.isFailure)
        assertEquals("Please sign in to manage favorites", result.exceptionOrNull()?.message)
    }

    @Test
    fun removeFavorite_url_encodes_listing_id_path_segment() = runTest {
        val cap = Captured()
        val repo = repoCapturing(cap, HttpStatusCode.NoContent, "")

        repo.removeFavorite("../admin/secret")

        assertEquals("/api/v1/favorites/..%2Fadmin%2Fsecret", cap.path)
    }

    // --- toggle round-trip shape ---

    /**
     * A favorite-toggle UI reads `isFavorite` then calls add or remove. This pins that the read
     * reflects each write: false -> add (201) -> true, then true -> remove -> false, driven purely
     * by the documented status/body contracts above.
     */
    @Test
    fun toggle_sequence_add_then_remove_matches_check_states() = runTest {
        // Start un-favorited.
        val checkBefore =
            repoCapturing(Captured(), HttpStatusCode.OK, """{"is_favorited": false}""")
                .isFavorite("lst-1")
        assertFalse(checkBefore.getOrThrow())

        // Add -> 201.
        val add =
            repoCapturing(
                    Captured(),
                    HttpStatusCode.Created,
                    """{"id":"fav-1","listing_id":"lst-1","created_at":"2026-04-26T10:00:00Z"}""",
                )
                .addFavorite("lst-1")
        assertTrue(add.isSuccess)

        // Now check reports favorited.
        val checkAfterAdd =
            repoCapturing(Captured(), HttpStatusCode.OK, """{"is_favorited": true}""")
                .isFavorite("lst-1")
        assertTrue(checkAfterAdd.getOrThrow())

        // Remove -> 204, check reports un-favorited again.
        val remove = repoCapturing(Captured(), HttpStatusCode.NoContent, "").removeFavorite("lst-1")
        assertTrue(remove.isSuccess)
        val checkAfterRemove =
            repoCapturing(Captured(), HttpStatusCode.OK, """{"is_favorited": false}""")
                .isFavorite("lst-1")
        assertFalse(checkAfterRemove.getOrThrow())

        assertNull(remove.exceptionOrNull())
    }
}
