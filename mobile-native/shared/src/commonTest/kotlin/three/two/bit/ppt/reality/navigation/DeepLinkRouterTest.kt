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
    fun parse_sso_deep_link_extracts_state_nonce() {
        // CSRF `state` nonce must be carried through parsing so the handler can compare it against
        // the pending flow nonce (parity with iOS `consumeSsoState`). Regression guard for the
        // account-takeover fix.
        val target = DeepLinkRouter.parse("reality://sso?token=tok-42&state=nonce-abc")
        assertEquals(DeepLinkTarget.Sso("tok-42", "nonce-abc"), target)
    }

    @Test
    fun parse_sso_deep_link_state_is_order_independent() {
        // `state` may precede `token` in the query string.
        val target = DeepLinkRouter.parse("reality://sso?state=nonce-abc&token=tok-42")
        assertEquals(DeepLinkTarget.Sso("tok-42", "nonce-abc"), target)
    }

    @Test
    fun parse_sso_deep_link_without_state_leaves_state_null() {
        // An externally-injected link with no `state` must parse to a null state so the handler can
        // reject it. `Sso("tok-42")` is `Sso("tok-42", null)` via the default arg.
        val target = DeepLinkRouter.parse("reality://sso?token=tok-42")
        assertEquals(DeepLinkTarget.Sso("tok-42", null), target)
    }

    @Test
    fun parse_sso_deep_link_percent_decodes_state() {
        val target = DeepLinkRouter.parse("reality://sso?token=t&state=ab%2Bcd")
        assertEquals(DeepLinkTarget.Sso("t", "ab+cd"), target)
    }

    @Test
    fun parse_sso_deep_link_percent_decodes_token() {
        // A real JWT/opaque token may contain reserved chars that get percent-encoded in the URL.
        // The shared router must decode them so the token matches what Android's
        // `Uri.getQueryParameter` (which decodes) hands to the same SSO validation path.
        // %2B -> '+', %2F -> '/', %3D -> '='  (a base64url/base64 token with padding)
        val target = DeepLinkRouter.parse("reality://sso?token=ab%2Bcd%2Fef%3D%3D")
        assertEquals(DeepLinkTarget.Sso("ab+cd/ef=="), target)
    }

    @Test
    fun parse_sso_deep_link_decodes_plus_as_space() {
        // Query semantics: '+' decodes to a space (form-urlencoded), matching platform URI
        // decoders.
        val target = DeepLinkRouter.parse("reality://sso?token=a+b")
        assertEquals(DeepLinkTarget.Sso("a b"), target)
    }

    @Test
    fun parse_sso_deep_link_decodes_utf8_multibyte() {
        // %C3%A1 is the UTF-8 encoding of 'á' — two bytes must combine into one char.
        val target = DeepLinkRouter.parse("reality://sso?token=caf%C3%A9")
        assertEquals(DeepLinkTarget.Sso("café"), target)
    }

    @Test
    fun parse_sso_deep_link_leaves_unencoded_token_untouched() {
        // No escapes -> fast path returns the token verbatim.
        val target = DeepLinkRouter.parse("reality://sso?token=plain-token-123")
        assertEquals(DeepLinkTarget.Sso("plain-token-123"), target)
    }

    @Test
    fun parse_sso_deep_link_tolerates_malformed_escape() {
        // A stray '%' not followed by two hex digits is passed through literally rather than
        // throwing.
        val target = DeepLinkRouter.parse("reality://sso?token=50%off")
        assertEquals(DeepLinkTarget.Sso("50%off"), target)
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
