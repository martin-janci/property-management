package three.two.bit.ppt.reality.auth

/**
 * Single-slot CSRF nonce store for the mobile SSO deep-link flow (Epic 10A-SSO).
 *
 * The Reality Portal SSO round-trip re-enters the app through the OS deep-link handler
 * (`reality://sso?token=…&state=…`). Because that intent-filter is `exported` (any browser,
 * notification, or other app can deliver it), a `token` on its own is not trustworthy: an attacker
 * can hand the victim a link carrying the attacker's session token and silently sign the victim
 * into the attacker's account (session fixation → account takeover).
 *
 * This store is the Kotlin counterpart of the iOS `AuthManager.beginSsoFlow()` /
 * `consumeSsoState(_:)` pair — closing the Android-vs-iOS parity gap. Before starting the SSO
 * browser hop, a caller mints a nonce with [mint] and appends it to the outbound URL as `&state=…`;
 * when the callback deep link arrives, [MainActivity][consume] verifies the echoed `state` matches
 * the pending nonce and only then validates the token.
 *
 * Semantics (mirroring iOS exactly):
 * - **Single slot** — a new flow overwrites any previously pending nonce (last-writer-wins).
 * - **Single use** — [consume] clears the pending nonce regardless of outcome, so a replay of the
 *   same deep link is rejected on the second delivery.
 * - **Default-reject** — with no pending nonce (fresh process, or after a consume), [consume]
 *   returns `false` for any input. An unsolicited, externally-injected `reality://sso` deep link is
 *   therefore rejected out of the box.
 *
 * A [SsoStateStore] instance is process-local, single-threaded in practice (mint from the UI,
 * consume from the deep-link handler on the main thread) — matching the plain-`var` design of the
 * iOS implementation.
 */
object SsoStateStore {

    private var pending: String? = null

    /**
     * Mint a fresh CSRF nonce, persist it as the single pending value, and return it. Callers MUST
     * append the returned value to the outbound SSO URL as the `state` query parameter so the
     * server echoes it back to the [DeepLinkTarget.Sso] callback.
     */
    fun mint(): String {
        val nonce = generateSsoNonce()
        pending = nonce
        return nonce
    }

    /**
     * Consume the pending nonce.
     *
     * Returns `true` iff [received] is non-empty and equals the value most recently produced by
     * [mint] (and a nonce is in fact outstanding). The stored value is cleared after a single
     * check, regardless of outcome, so a replay or a probe with a missing/empty `state` is
     * rejected.
     */
    fun consume(received: String?): Boolean {
        val expected = pending
        pending = null
        return !expected.isNullOrEmpty() && !received.isNullOrEmpty() && received == expected
    }

    /** True when a nonce is currently outstanding (mostly for introspection/tests). */
    fun hasPending(): Boolean = pending != null

    /** Clear any pending nonce (e.g. on logout, or between tests). */
    fun reset() {
        pending = null
    }
}

/**
 * Platform-provided 64-character hex CSRF nonce (≈256 bits). Android uses
 * `java.security.SecureRandom`; iOS provides a UUID-based value (the KMP store is not exercised by
 * the iOS app, which owns its own nonce in `AuthManager`, but an `actual` is required for the
 * shared module to compile).
 */
internal expect fun generateSsoNonce(): String
