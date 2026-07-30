package three.two.bit.ppt.reality.auth

import three.two.bit.ppt.reality.navigation.DeepLinkRouter
import three.two.bit.ppt.reality.navigation.DeepLinkTarget
import kotlin.test.AfterTest
import kotlin.test.BeforeTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

/**
 * Contract for the SSO initiation leg — the production call site for [SsoStateStore.mint] that was
 * missing after PR #2568 (issue #2574), leaving every `reality://sso` callback default-rejected.
 *
 * The key case is [mint_then_callback_consume_round_trips]: it exercises the full loop the two
 * halves of the CSRF guard form — [SsoInitiation.begin] mints + emits `&state=`, and a matching
 * callback parsed by [DeepLinkRouter] is accepted by [SsoStateStore.consume]. On `dev` (no
 * `SsoInitiation`) this file does not compile — the regression guard for the missing-call-site fix.
 */
class SsoInitiationTest {

    @BeforeTest fun clear() = SsoStateStore.reset()

    @AfterTest fun cleanup() = SsoStateStore.reset()

    @Test
    fun build_url_targets_pm_app_with_reality_callback_and_state() {
        val url = SsoInitiation.buildUrl("abc123")
        assertEquals("propertymanagement://sso?callback=reality://sso&state=abc123", url)
    }

    @Test
    fun callback_scheme_matches_the_reality_deep_link_scheme() {
        // The callback must re-enter through the same scheme the exported intent-filter listens on,
        // otherwise the PM app would redirect into a dead URL.
        assertEquals("${DeepLinkRouter.SCHEME}://sso", SsoInitiation.CALLBACK)
    }

    @Test
    fun begin_leaves_a_pending_nonce_matching_the_url_state() {
        val url = SsoInitiation.begin()
        assertTrue(SsoStateStore.hasPending(), "begin() must mint a pending nonce")
        val state = url.substringAfter("&state=")
        assertTrue(state.isNotEmpty(), "the outbound URL must carry a state nonce")
    }

    @Test
    fun mint_then_callback_consume_round_trips() {
        // 1. App initiates SSO: mint + build the outbound hop.
        val outbound = SsoInitiation.begin()
        val state = outbound.substringAfter("&state=")

        // 2. PM app authenticates and redirects back to the reality:// callback echoing the state.
        val callback = "reality://sso?token=session-token-xyz&state=$state"
        val target = DeepLinkRouter.parse(callback)
        assertTrue(target is DeepLinkTarget.Sso, "the callback must parse as an SSO target")
        assertEquals("session-token-xyz", target.token)
        assertEquals(state, target.state)

        // 3. The receiver accepts the token only because the state matches the minted nonce.
        assertTrue(
            SsoStateStore.consume(target.state),
            "a callback echoing the minted state must be accepted",
        )
    }

    @Test
    fun callback_with_forged_state_is_rejected_after_begin() {
        SsoInitiation.begin()
        val forged = "reality://sso?token=attacker-token&state=forged-nonce"
        val target = DeepLinkRouter.parse(forged)
        assertTrue(target is DeepLinkTarget.Sso)
        assertFalse(
            SsoStateStore.consume(target.state),
            "a callback whose state does not match the minted nonce must be rejected",
        )
    }
}
