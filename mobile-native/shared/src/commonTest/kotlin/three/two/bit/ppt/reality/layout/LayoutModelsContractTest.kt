package three.two.bit.ppt.reality.layout

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.serialization.decodeFromString
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

/**
 * Contract tests for the layout-domain DTOs.
 *
 * Pins the wire shape produced/consumed by the reality-server layout endpoint so that any drift in
 *
 * @SerialName annotations or model edits is caught at build time. The server's serde output skips
 *   absent optional fields (mode, props) — verified here via round-trip with encodeDefaults=false.
 */
class LayoutModelsContractTest {

    private val json = Json {
        ignoreUnknownKeys = true
        encodeDefaults = false
    }

    // The canonical server-shape fixture. Note: mode and props are absent (null → omitted).
    private val serverFixture =
        """
        {
          "screen": "reality/listing-detail",
          "version": 0,
          "sections": [
            {"type": "gallery.v1", "presentation": "visible"},
            {"type": "listing-header.v1", "presentation": "visible"},
            {"type": "key-details.v1", "presentation": "visible"},
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
    // Decode from server fixture
    // -------------------------------------------------------------------

    @Test
    fun resolvedLayoutScreen_decodes_server_fixture() {
        val screen: ResolvedLayoutScreen = json.decodeFromString(serverFixture)

        assertEquals("reality/listing-detail", screen.screen)
        assertEquals(0, screen.version)
        assertEquals(8, screen.sections.size)
    }

    @Test
    fun resolvedLayoutScreen_sections_decode_in_order() {
        val screen: ResolvedLayoutScreen = json.decodeFromString(serverFixture)

        assertEquals("gallery.v1", screen.sections[0].type)
        assertEquals("listing-header.v1", screen.sections[1].type)
        assertEquals("key-details.v1", screen.sections[2].type)
        assertEquals("description.v1", screen.sections[3].type)
        assertEquals("features.v1", screen.sections[4].type)
        assertEquals("additional-info.v1", screen.sections[5].type)
        assertEquals("resources.v1", screen.sections[6].type)
        assertEquals("agent-contact.v1", screen.sections[7].type)
    }

    @Test
    fun resolvedLayoutSection_visible_presentation_decoded() {
        val screen: ResolvedLayoutScreen = json.decodeFromString(serverFixture)

        for (section in screen.sections) {
            assertEquals("visible", section.presentation, "all fixture sections should be visible")
            assertTrue(section.isVisible)
            assertFalse(section.isPlaceholder)
        }
    }

    @Test
    fun resolvedLayoutSection_absent_mode_and_props_are_null() {
        val screen: ResolvedLayoutScreen = json.decodeFromString(serverFixture)

        for (section in screen.sections) {
            assertNull(section.mode, "mode should be null when absent in JSON")
            assertNull(section.props, "props should be null when absent in JSON")
        }
    }

    // -------------------------------------------------------------------
    // Encode → matches server wire shape (mode/props absent when null)
    // -------------------------------------------------------------------

    @Test
    fun resolvedLayoutSection_null_mode_and_props_omitted_in_encode() {
        val section = ResolvedLayoutSection(type = "gallery.v1", presentation = "visible")
        val encoded = json.encodeToString(section)

        assertFalse(encoded.contains("\"mode\""), "null mode must be omitted: $encoded")
        assertFalse(encoded.contains("\"props\""), "null props must be omitted: $encoded")
        assertTrue(encoded.contains("\"type\":\"gallery.v1\""))
        // With encodeDefaults=false the presentation default ("visible") is omitted in the
        // encoded output — the server always sends it explicitly, but the client omits defaults
        // to keep payloads lean. Decode is the authoritative contract; see round-trip tests.
        assertFalse(
            encoded.contains("\"presentation\""),
            "default presentation omitted with encodeDefaults=false: $encoded",
        )
    }

    // -------------------------------------------------------------------
    // Round-trip
    // -------------------------------------------------------------------

    @Test
    fun resolvedLayoutScreen_round_trips_through_json() {
        val original: ResolvedLayoutScreen = json.decodeFromString(serverFixture)
        val encoded = json.encodeToString(original)
        val decoded: ResolvedLayoutScreen = json.decodeFromString(encoded)

        assertEquals(original, decoded)
    }

    @Test
    fun resolvedLayoutScreen_with_placeholder_round_trips() {
        val screen =
            ResolvedLayoutScreen(
                screen = "reality/listing-detail",
                version = 1,
                sections =
                    listOf(
                        ResolvedLayoutSection(type = "gallery.v1", presentation = "visible"),
                        ResolvedLayoutSection(
                            type = "key-details.v1",
                            presentation = "placeholder",
                        ),
                    ),
            )

        val decoded: ResolvedLayoutScreen = json.decodeFromString(json.encodeToString(screen))

        assertEquals(screen, decoded)
        assertTrue(decoded.sections[1].isPlaceholder)
        assertFalse(decoded.sections[1].isVisible)
    }

    // -------------------------------------------------------------------
    // Placeholder presentation
    // -------------------------------------------------------------------

    @Test
    fun resolvedLayoutSection_placeholder_presentation_parsed() {
        val raw = """{"type":"gallery.v1","presentation":"placeholder"}"""
        val section: ResolvedLayoutSection = json.decodeFromString(raw)

        assertTrue(section.isPlaceholder)
        assertFalse(section.isVisible)
    }

    // -------------------------------------------------------------------
    // Unknown presentation → visible (defensive)
    // -------------------------------------------------------------------

    @Test
    fun resolvedLayoutSection_unknown_presentation_is_visible() {
        val raw = """{"type":"gallery.v1","presentation":"future-unknown-value"}"""
        val section: ResolvedLayoutSection = json.decodeFromString(raw)

        assertEquals("future-unknown-value", section.presentation)
        assertFalse(section.isPlaceholder, "unknown string is not placeholder")
        assertTrue(section.isVisible, "unknown presentation → treated as visible (defensive)")
    }

    // -------------------------------------------------------------------
    // DEFAULT_LISTING_DETAIL_LAYOUT structure
    // -------------------------------------------------------------------

    @Test
    fun defaultListingDetailLayout_has_correct_screen_and_version() {
        assertEquals("reality/listing-detail", DEFAULT_LISTING_DETAIL_LAYOUT.screen)
        assertEquals(0, DEFAULT_LISTING_DETAIL_LAYOUT.version)
    }

    @Test
    fun defaultListingDetailLayout_has_eight_sections_all_visible() {
        assertEquals(8, DEFAULT_LISTING_DETAIL_LAYOUT.sections.size)
        for (section in DEFAULT_LISTING_DETAIL_LAYOUT.sections) {
            assertTrue(section.isVisible, "${section.type} should be visible in default layout")
            assertFalse(section.isPlaceholder)
        }
    }

    @Test
    fun defaultListingDetailLayout_section_types_in_order() {
        val types = DEFAULT_LISTING_DETAIL_LAYOUT.sections.map { it.type }
        assertEquals(
            listOf(
                "gallery.v1",
                "listing-header.v1",
                "key-details.v1",
                "description.v1",
                "features.v1",
                "additional-info.v1",
                "resources.v1",
                "agent-contact.v1",
            ),
            types,
        )
    }

    // -------------------------------------------------------------------
    // LayoutDefaults accessor
    // -------------------------------------------------------------------

    @Test
    fun layoutDefaults_listingDetail_equals_DEFAULT_LISTING_DETAIL_LAYOUT() {
        assertEquals(
            DEFAULT_LISTING_DETAIL_LAYOUT,
            LayoutDefaults.listingDetail,
            "LayoutDefaults.listingDetail must equal the compiled DEFAULT_LISTING_DETAIL_LAYOUT",
        )
    }
}
