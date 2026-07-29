package three.two.bit.ppt.reality.auth

import kotlin.test.AfterTest
import kotlin.test.BeforeTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotEquals
import kotlin.test.assertTrue

/**
 * Contract for the SSO CSRF nonce store — the Kotlin counterpart of iOS
 * `AuthManager.beginSsoFlow()` / `consumeSsoState(_:)`.
 *
 * These pin the security-critical behaviour behind the Android `MainActivity` SSO deep-link handler:
 * an SSO callback token is validated ONLY when its `state` matches a nonce this app minted for a
 * pending flow. They fail to even compile on `dev` (the store does not exist there), which is the
 * regression guard for the account-takeover fix.
 */
class SsoStateStoreTest {

    @BeforeTest fun clear() = SsoStateStore.reset()

    @AfterTest fun cleanup() = SsoStateStore.reset()

    @Test
    fun matching_state_is_accepted() {
        val nonce = SsoStateStore.mint()
        assertTrue(SsoStateStore.consume(nonce), "the minted nonce must be accepted")
    }

    @Test
    fun mismatched_state_is_rejected() {
        SsoStateStore.mint()
        assertFalse(SsoStateStore.consume("not-the-real-nonce"))
    }

    @Test
    fun null_state_is_rejected_even_with_pending_nonce() {
        SsoStateStore.mint()
        assertFalse(SsoStateStore.consume(null))
    }

    @Test
    fun empty_state_is_rejected() {
        SsoStateStore.mint()
        assertFalse(SsoStateStore.consume(""))
    }

    @Test
    fun unsolicited_callback_with_no_pending_flow_is_rejected() {
        // Fresh store — no mint() — must reject any input. This is the core account-takeover guard:
        // an externally-injected `reality://sso?token=…` with no legitimate flow behind it fails.
        assertFalse(SsoStateStore.consume("anything"))
        assertFalse(SsoStateStore.consume(null))
    }

    @Test
    fun nonce_is_single_use_second_consume_fails() {
        val nonce = SsoStateStore.mint()
        assertTrue(SsoStateStore.consume(nonce))
        // A replay of the same deep link must be rejected on the second delivery.
        assertFalse(SsoStateStore.consume(nonce))
    }

    @Test
    fun mismatch_clears_pending_so_later_correct_value_also_fails() {
        val nonce = SsoStateStore.mint()
        assertFalse(SsoStateStore.consume("wrong"))
        // The failed probe consumed (cleared) the pending nonce — the real value no longer works.
        assertFalse(SsoStateStore.consume(nonce))
    }

    @Test
    fun new_flow_overwrites_previous_pending_nonce() {
        val first = SsoStateStore.mint()
        val second = SsoStateStore.mint()
        assertNotEquals(first, second)
        assertFalse(SsoStateStore.consume(first), "the superseded nonce must not be accepted")
        // Re-mint because the failed consume cleared state, then verify the latest wins.
        val latest = SsoStateStore.mint()
        assertTrue(SsoStateStore.consume(latest))
    }

    @Test
    fun mint_produces_distinct_high_entropy_nonces() {
        val nonces = (1..16).map { SsoStateStore.mint() }.toSet()
        assertEquals(16, nonces.size, "every mint() must yield a distinct nonce")
        // 32 bytes -> 64 hex chars on Android; the UUID-based iOS value is also 64 chars.
        assertTrue(nonces.all { it.length >= 32 }, "nonce should carry meaningful entropy")
    }
}
