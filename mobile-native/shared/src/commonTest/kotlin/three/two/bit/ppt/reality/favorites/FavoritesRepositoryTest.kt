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
import kotlin.test.assertTrue
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.Json

/**
 * Repository-level coroutine tests for [FavoritesRepository.isFavorite].
 *
 * Issue #697 — pins the critical regression case: `GET /api/v1/favorites/{id}/check` always returns
 * HTTP 200 with `{"is_favorited": bool}`, so the repository must inspect the body rather than
 * collapse any 2xx to `Result.success(true)`.
 */
class FavoritesRepositoryTest {

    private val json = Json {
        ignoreUnknownKeys = true
        isLenient = true
        encodeDefaults = true
    }

    private fun repoWith(status: HttpStatusCode, body: String): FavoritesRepository {
        val engine = MockEngine { _ ->
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

    @Test
    fun isFavorite_returns_true_when_server_reports_is_favorited_true() = runTest {
        val repo = repoWith(HttpStatusCode.OK, """{"is_favorited": true}""")
        val result = repo.isFavorite("lst-1")
        assertTrue(result.isSuccess, "expected success, got $result")
        assertEquals(true, result.getOrNull())
    }

    /**
     * Key regression case for the gap-82-7 fix: a 200 with `is_favorited: false` must surface as
     * `Result.success(false)`. The pre-fix code collapsed any 2xx to `true`, silently flipping the
     * heart icon on un-favorited listings.
     */
    @Test
    fun isFavorite_returns_false_when_server_reports_is_favorited_false() = runTest {
        val repo = repoWith(HttpStatusCode.OK, """{"is_favorited": false}""")
        val result = repo.isFavorite("lst-1")
        assertTrue(result.isSuccess, "expected success, got $result")
        assertEquals(false, result.getOrNull())
    }

    @Test
    fun isFavorite_returns_false_on_unauthorized() = runTest {
        val repo = repoWith(HttpStatusCode.Unauthorized, """{"error":"unauthenticated"}""")
        val result = repo.isFavorite("lst-1")
        assertTrue(result.isSuccess, "401 should map to Result.success(false), got $result")
        assertEquals(false, result.getOrNull())
    }

    @Test
    fun isFavorite_returns_failure_on_server_error() = runTest {
        val repo = repoWith(HttpStatusCode.InternalServerError, """{"error":"boom"}""")
        val result = repo.isFavorite("lst-1")
        assertTrue(result.isFailure, "5xx should surface as Result.failure, got $result")
        val ex = result.exceptionOrNull()
        assertTrue(
            ex is FavoritesException,
            "expected FavoritesException, got ${ex?.let { it::class }}",
        )
    }

    /**
     * Round-1 review (#788): listing ids are spliced into the request path and must be
     * percent-encoded so a hostile id cannot smuggle extra path segments or query parameters
     * (path-injection). A `/`-bearing id must NOT escape its segment.
     */
    @Test
    fun isFavorite_url_encodes_listing_id_path_segment() = runTest {
        var capturedPath: String? = null
        val engine = MockEngine { request ->
            capturedPath = request.url.encodedPath
            respond(
                content = ByteReadChannel("""{"is_favorited": false}"""),
                status = HttpStatusCode.OK,
                headers = headersOf(HttpHeaders.ContentType, "application/json"),
            )
        }
        val client = HttpClient(engine) { install(ContentNegotiation) { json(json) } }
        val repo =
            FavoritesRepository(
                baseUrl = "https://example.test",
                sessionToken = "test-token",
                client = client,
            )

        repo.isFavorite("../admin/secret")

        // The `/` characters must be percent-encoded; the path stays anchored under /favorites/.
        assertEquals(
            "/api/v1/favorites/..%2Fadmin%2Fsecret/check",
            capturedPath,
            "listingId must be percent-encoded into a single path segment",
        )
    }
}
