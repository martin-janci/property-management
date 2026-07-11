package three.two.bit.ppt.reality.inquiry

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
 * Security regression (#2195): inquiry and viewing ids are spliced into the request path, so they
 * must be percent-encoded via the shared `String.asPathSegment()` helper — a `/`, `?`, `#`, or
 * `..`-bearing id must NOT escape its path segment (path-injection). Mirrors
 * `ListingDetailRepositoryTest.getListingDetail_url_encodes_listing_id_path_segment`.
 */
class InquiryRepositoryPathEncodingTest {

    private val json = Json {
        ignoreUnknownKeys = true
        isLenient = true
        encodeDefaults = true
    }

    private class Captured {
        var method: HttpMethod? = null
        var path: String? = null
    }

    private fun repoCapturing(captured: Captured): InquiryRepository {
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
        return InquiryRepository(
            baseUrl = "https://example.test",
            sessionToken = "test-token",
            client = client,
        )
    }

    @Test
    fun inquiry_and_viewing_id_path_segments_are_percent_encoded() = runTest {
        val getInquiry = Captured()
        repoCapturing(getInquiry).getInquiry("../admin/secret")
        assertEquals("/api/v1/inquiries/..%2Fadmin%2Fsecret", getInquiry.path)

        val reply = Captured()
        repoCapturing(reply).replyToInquiry("../admin/secret", "hello")
        assertEquals("/api/v1/inquiries/..%2Fadmin%2Fsecret/replies", reply.path)

        val cancel = Captured()
        repoCapturing(cancel).cancelViewing("../admin/secret")
        assertEquals("/api/v1/viewings/..%2Fadmin%2Fsecret", cancel.path)
    }
}
