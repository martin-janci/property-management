package three.two.bit.ppt.reality.auth

import three.two.bit.ppt.reality.navigation.DeepLinkRouter

/**
 * Builds the outbound SSO hop that hands the login flow to the Property Management app, and — as its
 * one and only responsibility — is the production call site for [SsoStateStore.mint].
 *
 * This is the Android/KMP mirror of iOS `LoginView.loginWithSso()`:
 * ```swift
 * let state = authManager.beginSsoFlow()
 * "propertymanagement://sso?callback=realityportal://sso&state=\(state)"
 * ```
 * The two platforms differ only in the callback scheme — iOS re-enters through
 * `realityportal://sso`, Android through `reality://sso` ([DeepLinkRouter.SCHEME]).
 *
 * The CSRF contract (see `mobile-native/CLAUDE.md` → "SSO deep-link CSRF contract"): before opening
 * the hop the app [mint][SsoStateStore.mint]s a single-use nonce and appends it as `&state=`; the PM
 * app round-trips it back on the `reality://sso?token=…&state=…` callback, where
 * `MainActivity.handleDeepLink` calls [SsoStateStore.consume] and validates the token only on a
 * match. Without this initiation leg no nonce is ever pending, so the default-reject `consume` drops
 * every callback — which is exactly the half-wired gap this closes.
 */
object SsoInitiation {

    /** Custom URL scheme of the Property Management app that owns the SSO login. */
    const val PM_APP_SCHEME: String = "propertymanagement"

    /**
     * The callback URL the PM app must redirect back to after authenticating. Derived from
     * [DeepLinkRouter.SCHEME] so it stays in lockstep with the `reality://sso` intent-filter and the
     * shared deep-link parser.
     */
    val CALLBACK: String = "${DeepLinkRouter.SCHEME}://sso"

    /**
     * Pure URL builder — assembles the outbound SSO URL for a given CSRF [state] nonce. Kept
     * separate from [begin] so the URL shape is unit-testable without touching the process-global
     * [SsoStateStore].
     */
    fun buildUrl(state: String): String = "$PM_APP_SCHEME://sso?callback=$CALLBACK&state=$state"

    /**
     * Mint a fresh single-use CSRF nonce, register it as the pending value in [SsoStateStore], and
     * return the outbound SSO URL carrying it as `state`. The caller launches the returned URL (on
     * Android via an `ACTION_VIEW` intent); the echoed `state` is later verified by
     * [SsoStateStore.consume] on the callback.
     */
    fun begin(): String = buildUrl(SsoStateStore.mint())
}
