package three.two.bit.ppt.reality.navigation

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

/**
 * Story 82-2, AC-4 — Reality mobile navigation state preservation and deep-link URL schemes.
 *
 * These tests pin the platform-agnostic contract behind the Android `MainActivity` deep-link
 * handler and the Compose nav graph:
 * 1. Deep-link URL schemes (`reality://...`) parse into a stable [DeepLinkTarget].
 * 2. Each target resolves to the exact nav-graph route string used by the Compose graph.
 * 3. The set of top-level tab routes (the ones whose back-stack state must be saved/restored when
 *    switching tabs) is well-defined and matches what the bottom bar drives.
 */
class DeepLinkRouterTest {

    // -------- URL scheme parsing (AC-4: deep-link URL schemes) --------

    @Test
    fun parse_listing_deep_link_extracts_id() {
        val target = DeepLinkRouter.parse("reality://listing/abc-123")
        assertEquals(DeepLinkTarget.Listing("abc-123"), target)
    }

    @Test
    fun parse_listing_deep_link_with_trailing_segments_uses_first() {
        // reality://listing/{id}/whatever — the id is the first path segment.
        val target = DeepLinkRouter.parse("reality://listing/xyz/extra")
        assertEquals(DeepLinkTarget.Listing("xyz"), target)
    }

    @Test
    fun parse_listing_deep_link_without_id_is_null() {
        assertNull(DeepLinkRouter.parse("reality://listing"))
        assertNull(DeepLinkRouter.parse("reality://listing/"))
    }

    @Test
    fun parse_search_deep_link() {
        assertEquals(DeepLinkTarget.Search, DeepLinkRouter.parse("reality://search"))
    }

    @Test
    fun parse_favorites_deep_link() {
        assertEquals(DeepLinkTarget.Favorites, DeepLinkRouter.parse("reality://favorites"))
    }

    @Test
    fun parse_inquiries_deep_link() {
        assertEquals(DeepLinkTarget.Inquiries, DeepLinkRouter.parse("reality://inquiries"))
    }

    @Test
    fun parse_sso_deep_link_extracts_token() {
        val target = DeepLinkRouter.parse("reality://sso?token=tok-42")
        assertEquals(DeepLinkTarget.Sso("tok-42"), target)
    }

    @Test
    fun parse_sso_deep_link_without_token_is_null() {
        assertNull(DeepLinkRouter.parse("reality://sso"))
    }

    @Test
    fun parse_rejects_foreign_scheme() {
        assertNull(DeepLinkRouter.parse("https://listing/abc-123"))
        assertNull(DeepLinkRouter.parse("ppt://listing/abc-123"))
    }

    @Test
    fun parse_rejects_unknown_host() {
        assertNull(DeepLinkRouter.parse("reality://unknown"))
    }

    @Test
    fun parse_rejects_blank_or_garbage() {
        assertNull(DeepLinkRouter.parse(""))
        assertNull(DeepLinkRouter.parse("not a uri"))
    }

    // -------- target -> nav route resolution --------

    @Test
    fun route_for_listing_matches_compose_graph_route() {
        assertEquals("listing/abc-123", DeepLinkRouter.route(DeepLinkTarget.Listing("abc-123")))
    }

    @Test
    fun route_for_search() {
        assertEquals("search", DeepLinkRouter.route(DeepLinkTarget.Search))
    }

    @Test
    fun route_for_favorites() {
        assertEquals("favorites", DeepLinkRouter.route(DeepLinkTarget.Favorites))
    }

    @Test
    fun route_for_inquiries() {
        assertEquals("inquiries", DeepLinkRouter.route(DeepLinkTarget.Inquiries))
    }

    @Test
    fun route_for_sso_is_null_handled_out_of_band() {
        // SSO is handled by validating the token, not by navigating the graph.
        assertNull(DeepLinkRouter.route(DeepLinkTarget.Sso("tok-42")))
    }

    // -------- top-level tab routes (AC-4: navigation state preservation) --------

    @Test
    fun top_level_routes_are_the_five_bottom_bar_tabs() {
        assertEquals(
            setOf("home", "search", "favorites", "inquiries", "account"),
            DeepLinkRouter.TOP_LEVEL_ROUTES,
        )
    }

    @Test
    fun tab_routes_preserve_state_but_detail_routes_do_not() {
        assertTrue(DeepLinkRouter.preservesState("home"))
        assertTrue(DeepLinkRouter.preservesState("favorites"))
        // A listing detail route is not a state-preserving tab.
        assertFalse(DeepLinkRouter.preservesState("listing/abc-123"))
        assertFalse(DeepLinkRouter.preservesState("auth/login"))
        assertFalse(DeepLinkRouter.preservesState(null))
    }
}
