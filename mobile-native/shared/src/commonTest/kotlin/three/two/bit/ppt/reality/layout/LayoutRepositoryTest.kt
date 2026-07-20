package three.two.bit.ppt.reality.layout

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
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.Json

/**
 * Repository-level coroutine tests for [LayoutRepository.getListingDetailLayout].
 *
 * Mirrors the MockEngine harness from [ListingDetailRepositoryTest]. The repository must NEVER
 * throw; all failure paths return [DEFAULT_LISTING_DETAIL_LAYOUT].
 */
class LayoutRepositoryTest {

    private val json = Json {
        ignoreUnknownKeys = true
        isLenient = true
        encodeDefaults = true
    }

    private class Captured {
        var method: HttpMethod? = null
        var path: String? = null
        var query: String? = null
    }

    private fun repoCapturing(
        captured: Captured,
        status: HttpStatusCode,
        body: String,
    ): LayoutRepository {
        val engine = MockEngine { request ->
            captured.method = request.method
            captured.path = request.url.encodedPath
            captured.query = request.url.encodedQuery
            respond(
                content = ByteReadChannel(body),
                status = status,
                headers = headersOf(HttpHeaders.ContentType, "application/json"),
            )
        }
        val client = HttpClient(engine) { install(ContentNegotiation) { json(json) } }
        return LayoutRepository(baseUrl = "https://example.test", client = client)
    }

    private val validBody =
        """
        {
          "screen": "reality/listing-detail",
          "version": 1,
          "sections": [
            {"type": "gallery.v1", "presentation": "visible"},
            {"type": "listing-header.v1", "presentation": "visible"},
            {"type": "key-details.v1", "presentation": "placeholder"},
            {"type": "description.v1", "presentation": "visible"},
            {"type": "features.v1", "presentation": "visible"},
            {"type": "additional-info.v1", "presentation": "visible"},
            {"type": "resources.v1", "presentation": "visible"},
            {"type": "agent-contact.v1", "presentation": "visible"}
          ]
        }
        """
            .trimIndent()

    // -------------------------------------------------------------------
    // Happy path: 200 decode
    // -------------------------------------------------------------------

    @Test
    fun getListingDetailLayout_200_decodes_screen_and_sections_in_order() = runTest {
        val cap = Captured()
        val repo = repoCapturing(cap, HttpStatusCode.OK, validBody)

        val layout = repo.getListingDetailLayout()

        // Endpoint contract.
        assertEquals(HttpMethod.Get, cap.method)
        assertEquals("/api/v1/layout/resolved/reality/listing-detail", cap.path)
        assertTrue(cap.query?.contains("platform=mobile") == true, "should include platform=mobile")

        // Decode contract.
        assertEquals("reality/listing-detail", layout.screen)
        assertEquals(1, layout.version)
        assertEquals(8, layout.sections.size)
        assertEquals("gallery.v1", layout.sections[0].type)
        assertEquals("listing-header.v1", layout.sections[1].type)
        assertEquals("agent-contact.v1", layout.sections[7].type)
    }

    @Test
    fun getListingDetailLayout_200_placeholder_presentation_parsed() = runTest {
        val cap = Captured()
        val repo = repoCapturing(cap, HttpStatusCode.OK, validBody)

        val layout = repo.getListingDetailLayout()

        val keyDetails = layout.sections[2]
        assertEquals("key-details.v1", keyDetails.type)
        assertTrue(keyDetails.isPlaceholder, "presentation=placeholder → isPlaceholder")
        assertFalse(keyDetails.isVisible, "placeholder → !isVisible")
    }

    @Test
    fun getListingDetailLayout_200_visible_sections_are_visible() = runTest {
        val cap = Captured()
        val repo = repoCapturing(cap, HttpStatusCode.OK, validBody)

        val layout = repo.getListingDetailLayout()

        assertTrue(layout.sections[0].isVisible, "gallery.v1 should be visible")
        assertFalse(layout.sections[0].isPlaceholder)
    }

    @Test
    fun getListingDetailLayout_200_unknown_extra_json_fields_tolerated() = runTest {
        val bodyWithExtra =
            """
            {
              "screen": "reality/listing-detail",
              "version": 2,
              "future_field": "ignored",
              "sections": [
                {"type": "gallery.v1", "presentation": "visible", "unknown_future": true}
              ]
            }
            """
                .trimIndent()
        val repo = repoCapturing(Captured(), HttpStatusCode.OK, bodyWithExtra)

        val layout = repo.getListingDetailLayout()
        assertEquals("reality/listing-detail", layout.screen)
        assertEquals(1, layout.sections.size)
    }

    @Test
    fun getListingDetailLayout_200_empty_sections_accepted_as_is() = runTest {
        val bodyEmpty =
            """
            {
              "screen": "reality/listing-detail",
              "version": 0,
              "sections": []
            }
            """
                .trimIndent()
        val repo = repoCapturing(Captured(), HttpStatusCode.OK, bodyEmpty)

        val layout = repo.getListingDetailLayout()
        assertTrue(layout.sections.isEmpty(), "empty sections list is accepted")
        assertEquals("reality/listing-detail", layout.screen)
    }

    // -------------------------------------------------------------------
    // Failure paths: all return DEFAULT_LISTING_DETAIL_LAYOUT
    // -------------------------------------------------------------------

    @Test
    fun getListingDetailLayout_non2xx_returns_default() = runTest {
        val repo =
            repoCapturing(Captured(), HttpStatusCode.ServiceUnavailable, """{"error":"down"}""")

        val layout = repo.getListingDetailLayout()

        assertEquals(DEFAULT_LISTING_DETAIL_LAYOUT, layout)
    }

    @Test
    fun getListingDetailLayout_500_returns_default() = runTest {
        val repo =
            repoCapturing(Captured(), HttpStatusCode.InternalServerError, """{"error":"boom"}""")

        val layout = repo.getListingDetailLayout()

        assertEquals(DEFAULT_LISTING_DETAIL_LAYOUT, layout)
    }

    @Test
    fun getListingDetailLayout_404_returns_default() = runTest {
        val repo = repoCapturing(Captured(), HttpStatusCode.NotFound, """{"error":"not found"}""")

        val layout = repo.getListingDetailLayout()

        assertEquals(DEFAULT_LISTING_DETAIL_LAYOUT, layout)
    }

    @Test
    fun getListingDetailLayout_malformed_json_returns_default() = runTest {
        val repo = repoCapturing(Captured(), HttpStatusCode.OK, "not-json-at-all{{{")

        val layout = repo.getListingDetailLayout()

        assertEquals(DEFAULT_LISTING_DETAIL_LAYOUT, layout)
    }

    @Test
    fun getListingDetailLayout_wrong_screen_echo_returns_default() = runTest {
        val wrongScreen =
            """
            {
              "screen": "reality/some-other-screen",
              "version": 0,
              "sections": []
            }
            """
                .trimIndent()
        val repo = repoCapturing(Captured(), HttpStatusCode.OK, wrongScreen)

        val layout = repo.getListingDetailLayout()

        assertEquals(DEFAULT_LISTING_DETAIL_LAYOUT, layout)
    }

    // -------------------------------------------------------------------
    // Sanitization: duplicate section types are deduped (first kept)
    // -------------------------------------------------------------------

    @Test
    fun getListingDetailLayout_duplicate_section_types_deduped_first_kept() = runTest {
        val bodyWithDuplicates =
            """
            {
              "screen": "reality/listing-detail",
              "version": 1,
              "sections": [
                {"type": "gallery.v1", "presentation": "visible"},
                {"type": "listing-header.v1", "presentation": "visible"},
                {"type": "gallery.v1", "presentation": "placeholder"},
                {"type": "listing-header.v1", "presentation": "placeholder"}
              ]
            }
            """
                .trimIndent()
        val repo = repoCapturing(Captured(), HttpStatusCode.OK, bodyWithDuplicates)

        val layout = repo.getListingDetailLayout()

        assertEquals(2, layout.sections.size, "duplicate types must be dropped")
        assertEquals("gallery.v1", layout.sections[0].type)
        assertEquals("listing-header.v1", layout.sections[1].type)
        // First occurrence kept — both were "visible", the placeholder dupes dropped.
        assertTrue(layout.sections[0].isVisible, "first gallery occurrence (visible) is kept")
        assertTrue(layout.sections[1].isVisible, "first header occurrence (visible) is kept")
    }

    // -------------------------------------------------------------------
    // Cancellation must propagate (not swallowed into the default layout)
    // -------------------------------------------------------------------

    @Test
    fun getListingDetailLayout_cancellation_propagates() = runTest {
        val engine = MockEngine { throw CancellationException("cancelled mid-fetch") }
        val client = HttpClient(engine) { install(ContentNegotiation) { json(json) } }
        val repo = LayoutRepository(baseUrl = "https://example.test", client = client)

        assertFailsWith<CancellationException> { repo.getListingDetailLayout() }
    }

    // -------------------------------------------------------------------
    // Unknown presentation string → isVisible true (defensive)
    // -------------------------------------------------------------------

    @Test
    fun getListingDetailLayout_unknown_presentation_string_is_visible() = runTest {
        val bodyUnknownPresentation =
            """
            {
              "screen": "reality/listing-detail",
              "version": 0,
              "sections": [
                {"type": "gallery.v1", "presentation": "some-future-value"},
                {"type": "listing-header.v1", "presentation": "visible"}
              ]
            }
            """
                .trimIndent()
        val repo = repoCapturing(Captured(), HttpStatusCode.OK, bodyUnknownPresentation)

        val layout = repo.getListingDetailLayout()

        val section = layout.sections[0]
        assertEquals("some-future-value", section.presentation)
        assertFalse(section.isPlaceholder, "unknown string is not placeholder")
        assertTrue(section.isVisible, "unknown presentation → treated as visible (defensive)")
    }
}
