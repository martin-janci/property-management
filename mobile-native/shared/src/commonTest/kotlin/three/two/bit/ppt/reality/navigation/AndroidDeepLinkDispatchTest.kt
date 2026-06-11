package three.two.bit.ppt.reality.navigation

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull

/**
 * Regression guard for the Android `MainActivity` deep-link handler (`androidApp`).
 *
 * `MainActivity.handleDeepLink` used to reimplement `reality://` scheme/host dispatch with
 * `android.net.Uri` and a *second*, Android-only `DeepLinkTarget` sealed class that lacked `Sso` —
 * so a new shared target would silently no-op on Android. It now does nothing platform-specific
 * beyond pulling the URI string off the `Intent` and delegating to [DeepLinkRouter]:
 * ```kotlin
 * when (val target = DeepLinkRouter.parse(uri.toString())) {
 *     is DeepLinkTarget.Sso -> ssoService.validateAndLogin(target.token)   // out-of-band
 *     null -> Unit                                                          // not a deep link
 *     else -> pendingDeepLink.value = target                               // navigate later
 * }
 * ```
 *
 * These tests assert that the Android dispatch the handler relies on matches the shared router:
 * 1. Every host the old Android handler special-cased (`listing`, `search`, `favorites`,
 *    `inquiries`, `sso`) still parses to the expected shared [DeepLinkTarget].
 * 2. The Sso-vs-navigable partition the handler branches on is well-defined: `Sso` is the *only*
 *    target whose [DeepLinkRouter.route] is `null` (handled out-of-band), and every other target
 *    resolves to a navigable route. This is what makes `else -> pendingDeepLink.value = target`
 *    safe — no navigable target is ever dropped, and no out-of-band target is ever navigated.
 */
class AndroidDeepLinkDispatchTest {

    /** The five hosts the legacy Android `when (uri.host)` block handled, plus expected target. */
    private val androidHandledLinks: List<Pair<String, DeepLinkTarget>> =
        listOf(
            "reality://listing/abc-123" to DeepLinkTarget.Listing("abc-123"),
            "reality://search" to DeepLinkTarget.Search,
            "reality://favorites" to DeepLinkTarget.Favorites,
            "reality://inquiries" to DeepLinkTarget.Inquiries,
            "reality://sso?token=tok-42" to DeepLinkTarget.Sso("tok-42"),
        )

    @Test
    fun android_handled_links_parse_via_shared_router() {
        for ((uri, expected) in androidHandledLinks) {
            assertEquals(expected, DeepLinkRouter.parse(uri), "mismatch for $uri")
        }
    }

    @Test
    fun sso_is_the_only_out_of_band_target_rest_are_navigable() {
        for ((uri, target) in androidHandledLinks) {
            val route = DeepLinkRouter.route(target)
            if (target is DeepLinkTarget.Sso) {
                // Handler validates the token; navigateToDeepLink must NOT navigate it.
                assertNull(route, "Sso must be out-of-band for $uri")
            } else {
                // Handler stashes it in pendingDeepLink; navigateToDeepLink must navigate it.
                assertNotNull(route, "navigable target must resolve to a route for $uri")
            }
        }
    }

    @Test
    fun handler_drops_links_the_router_does_not_recognise() {
        // The `null` branch of the handler: foreign scheme / unknown host / garbage are ignored.
        assertNull(DeepLinkRouter.parse("https://listing/abc-123"))
        assertNull(DeepLinkRouter.parse("reality://unknown"))
        assertNull(DeepLinkRouter.parse(""))
    }

    @Test
    fun sso_token_is_percent_decoded_consistently_with_android_uri() {
        // Android's Uri.getQueryParameter decodes; MainActivity now feeds the raw URI string to the
        // shared router, which decodes itself. Both paths must yield the same token (PR #1155).
        val target = DeepLinkRouter.parse("reality://sso?token=ab%2Bcd%2Fef%3D%3D")
        assertEquals(DeepLinkTarget.Sso("ab+cd/ef=="), target)
        // And the navigable side stays out-of-band regardless of token contents.
        val sso = target as? DeepLinkTarget.Sso
        assertNotNull(sso)
        assertNull(DeepLinkRouter.route(sso))
    }
}
