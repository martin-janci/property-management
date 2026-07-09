package three.two.bit.ppt.reality.agency

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
 * Security regression (#2195): agency ids and slugs are deep-link / nav-sourced and are spliced
 * into the request path, so they must be percent-encoded via the shared `String.asPathSegment()`
 * helper — a `/`, `?`, `#`, or `..`-bearing value must NOT escape its path segment (path-injection).
 * Mirrors `ListingDetailRepositoryTest.getListingDetail_url_encodes_listing_id_path_segment`.
 */
class AgencyRepositoryPathEncodingTest {

    private val json = Json {
        ignoreUnknownKeys = true
        isLenient = true
        encodeDefaults = true
    }

    private class Captured {
        var method: HttpMethod? = null
        var path: String? = null
    }

    private fun repoCapturing(captured: Captured): AgencyRepository {
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
        return AgencyRepository(
            baseUrl = "https://example.test",
            sessionToken = "test-token",
            client = client,
        )
    }

    @Test
    fun agency_id_and_slug_path_segments_are_percent_encoded() = runTest {
        val getAgency = Captured()
        repoCapturing(getAgency).getAgency("../admin/secret")
        assertEquals("/api/v1/agencies/..%2Fadmin%2Fsecret", getAgency.path)

        val bySlug = Captured()
        repoCapturing(bySlug).getAgencyBySlug("../admin/secret")
        assertEquals("/api/v1/agencies/by-slug/..%2Fadmin%2Fsecret", bySlug.path)

        val members = Captured()
        repoCapturing(members).listMembers("../admin/secret")
        assertEquals("/api/v1/agencies/..%2Fadmin%2Fsecret/members", members.path)
    }
}
