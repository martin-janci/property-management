package three.two.bit.ppt.reality.favorites

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
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.Json

/**
 * Security regression (#2237): favorites + saved-search ids are deep-link / nav-sourced and are
 * spliced into the request path, so they must be percent-encoded via the shared
 * `String.asPathSegment()` helper — a `/`, `?`, `#`, or `..`-bearing value must NOT escape its path
 * segment (path-injection).
 *
 * PR #2226 refactored [FavoritesRepository] off its private `pathSegment` copy onto the shared
 * helper but left those already-encoded call sites (addFavorite / removeFavorite / isFavorite /
 * updateSavedSearch / deleteSavedSearch) with no direct coverage. This mirrors
 * [three.two.bit.ppt.reality.agency.AgencyRepositoryPathEncodingTest] and closes that gap so a
 * future edit to any of these splices can't silently regress the `..%2F` behavior.
 */
class FavoritesRepositoryPathEncodingTest {

    private val json = Json {
        ignoreUnknownKeys = true
        isLenient = true
        encodeDefaults = true
    }

    private class Captured {
        var method: HttpMethod? = null
        var path: String? = null
    }

    private fun repoCapturing(captured: Captured): FavoritesRepository {
        val engine = MockEngine { request ->
            captured.method = request.method
            captured.path = request.url.encodedPath
            respond(
                content = ByteReadChannel("{}"),
                status = HttpStatusCode.OK,
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

    // A hostile id that would path-traverse if spliced raw. `..%2Fadmin%2Fsecret` is the
    // percent-encoded expectation: the `/` characters stay inside the segment.
    private val hostileId = "../admin/secret"
    private val encodedId = "..%2Fadmin%2Fsecret"

    @Test
    fun addFavorite_encodes_listing_id_path_segment() = runTest {
        val cap = Captured()
        repoCapturing(cap).addFavorite(hostileId)
        assertEquals(HttpMethod.Post, cap.method)
        assertEquals("/api/v1/favorites/$encodedId", cap.path)
    }

    @Test
    fun removeFavorite_encodes_listing_id_path_segment() = runTest {
        val cap = Captured()
        repoCapturing(cap).removeFavorite(hostileId)
        assertEquals(HttpMethod.Delete, cap.method)
        assertEquals("/api/v1/favorites/$encodedId", cap.path)
    }

    @Test
    fun isFavorite_encodes_listing_id_path_segment() = runTest {
        val cap = Captured()
        repoCapturing(cap).isFavorite(hostileId)
        assertEquals(HttpMethod.Get, cap.method)
        assertEquals("/api/v1/favorites/$encodedId/check", cap.path)
    }

    @Test
    fun updateSavedSearch_encodes_search_id_path_segment() = runTest {
        val cap = Captured()
        repoCapturing(cap).updateSavedSearch(hostileId, UpdateSavedSearchRequest(name = "x"))
        assertEquals(HttpMethod.Patch, cap.method)
        assertEquals("/api/v1/saved-searches/$encodedId", cap.path)
    }

    @Test
    fun deleteSavedSearch_encodes_search_id_path_segment() = runTest {
        val cap = Captured()
        repoCapturing(cap).deleteSavedSearch(hostileId)
        assertEquals(HttpMethod.Delete, cap.method)
        assertEquals("/api/v1/saved-searches/$encodedId", cap.path)
    }
}
