package three.two.bit.ppt.reality.realtor

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
 * Security regression (#2195): the realtor userId is deep-link / nav-sourced and is spliced into
 * the request path, so it must be percent-encoded via the shared `String.asPathSegment()` helper —
 * a `/`, `?`, `#`, or `..`-bearing id must NOT escape its path segment (path-injection). Mirrors
 * `ListingDetailRepositoryTest.getListingDetail_url_encodes_listing_id_path_segment`.
 */
class RealtorRepositoryPathEncodingTest {

    private val json = Json {
        ignoreUnknownKeys = true
        isLenient = true
        encodeDefaults = true
    }

    private class Captured {
        var method: HttpMethod? = null
        var path: String? = null
    }

    private fun repoCapturing(captured: Captured): RealtorRepository {
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
        return RealtorRepository(
            baseUrl = "https://example.test",
            sessionToken = "test-token",
            client = client,
        )
    }

    @Test
    fun realtor_user_id_path_segment_is_percent_encoded() = runTest {
        val cap = Captured()
        repoCapturing(cap).getProfile("../admin/secret")
        assertEquals("/api/v1/realtors/..%2Fadmin%2Fsecret/profile", cap.path)
    }
}
