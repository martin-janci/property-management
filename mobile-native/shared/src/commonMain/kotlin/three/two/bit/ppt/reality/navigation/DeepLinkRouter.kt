package three.two.bit.ppt.reality.navigation

/**
 * A navigable destination reached through a `reality://` deep link (Epic 122) or surfaced by the
 * SSO callback (Epic 10A-SSO).
 *
 * This is the platform-agnostic mirror of the Android `MainActivity.DeepLinkTarget`. Keeping it in
 * `commonMain` lets the iOS app reuse the exact same parsing + route-resolution contract.
 */
sealed class DeepLinkTarget {
    data class Listing(val id: String) : DeepLinkTarget()

    data object Search : DeepLinkTarget()

    data object Favorites : DeepLinkTarget()

    data object Inquiries : DeepLinkTarget()

    /**
     * SSO callback: `reality://sso?token=…&state=…`. Handled by validating the token, not by
     * navigation.
     *
     * [state] is the per-flow CSRF nonce the app minted (via `SsoStateStore.mint`) before starting
     * the SSO browser hop; the server echoes it back on the callback so the app can detect a forged
     * or replayed deep link. It is nullable because a malicious/external caller can deliver a
     * `reality://sso?token=…` intent with no `state` at all — the handler must be able to represent
     * (and then reject) that case. Both mobile platforms require a matching `state`: iOS via
     * `AuthManager.consumeSsoState`, Android via `SsoStateStore.consume`.
     */
    data class Sso(val token: String, val state: String? = null) : DeepLinkTarget()
}

/**
 * Platform-agnostic deep-link parsing and nav-route resolution for Reality Portal mobile.
 *
 * Story 82-2, AC-4 — owns two things the Compose nav graph and `MainActivity` depend on:
 * - **Deep-link URL schemes**: turning a `reality://…` URI into a stable [DeepLinkTarget].
 * - **Navigation state preservation**: the set of top-level tab routes whose back-stack state is
 *   saved/restored when switching tabs (vs. detail/auth routes that are not).
 *
 * The string parsing here is intentionally framework-free (no `android.net.Uri`) so it is unit
 * testable on the JVM and reusable from iOS.
 */
object DeepLinkRouter {

    const val SCHEME: String = "reality"

    // Route strings — must stay in lockstep with `navigation.Screen` in the Android app.
    const val ROUTE_HOME: String = "home"
    const val ROUTE_SEARCH: String = "search"
    const val ROUTE_FAVORITES: String = "favorites"
    const val ROUTE_INQUIRIES: String = "inquiries"
    const val ROUTE_ACCOUNT: String = "account"

    /**
     * The five bottom-bar tab destinations. Switching between these saves and restores back-stack
     * state (`popUpTo(home){ saveState = true }; restoreState = true`), so a user who drills into a
     * tab and switches away returns to where they left off. Every other route (listing detail,
     * auth, realtor surfaces) is excluded and does not preserve state.
     */
    val TOP_LEVEL_ROUTES: Set<String> =
        setOf(ROUTE_HOME, ROUTE_SEARCH, ROUTE_FAVORITES, ROUTE_INQUIRIES, ROUTE_ACCOUNT)

    /** True when [route] is a state-preserving top-level tab. */
    fun preservesState(route: String?): Boolean = route in TOP_LEVEL_ROUTES

    /**
     * Platform-agnostic description of the Compose `navOptions` that make tab switches preserve
     * back-stack state. It is the data behind the inline lambda in the Android nav graph:
     * ```kotlin
     * navController.navigate(route) {
     *     popUpTo(popUpToRoute) { saveState = true }
     *     launchSingleTop = true
     *     restoreState = true
     * }
     * ```
     *
     * Keeping the option *values* here (rather than only in the Compose lambda) gives the
     * state-preservation behaviour a framework-free unit-test surface and lets iOS adopt the same
     * contract. Story 82-2, AC-4.
     *
     * @property popUpToRoute the route the back stack is popped up to before navigating — the graph
     *   start destination ([ROUTE_HOME]) so each tab is a single saved sub-stack under it.
     * @property popUpToSaveState save the popped destinations' state so it can be restored later.
     * @property launchSingleTop don't stack duplicate copies of an already-current tab.
     * @property restoreState restore any previously-saved state for the destination being navigated
     *   to, so re-selecting a tab returns the user to where they left off.
     */
    data class TabNavOptions(
        val popUpToRoute: String,
        val popUpToSaveState: Boolean,
        val launchSingleTop: Boolean,
        val restoreState: Boolean,
    )

    /**
     * The single set of nav options every bottom-bar tap must use to preserve and restore per-tab
     * back-stack state. Pop up to [ROUTE_HOME] saving state, single-top, and restore on the way in.
     */
    val TAB_NAV_OPTIONS: TabNavOptions =
        TabNavOptions(
            popUpToRoute = ROUTE_HOME,
            popUpToSaveState = true,
            launchSingleTop = true,
            restoreState = true,
        )

    /**
     * Parse a `reality://…` deep-link string into a [DeepLinkTarget], or `null` if the URI is
     * malformed, uses a foreign scheme, or targets an unknown host.
     */
    fun parse(uri: String): DeepLinkTarget? {
        val schemeSep = uri.indexOf("://")
        if (schemeSep <= 0) return null
        if (uri.substring(0, schemeSep) != SCHEME) return null

        val rest = uri.substring(schemeSep + 3)
        if (rest.isEmpty()) return null

        // Split off the query string (everything after the first '?').
        val queryStart = rest.indexOf('?')
        val pathPart = if (queryStart >= 0) rest.substring(0, queryStart) else rest
        val queryPart = if (queryStart >= 0) rest.substring(queryStart + 1) else ""

        val segments = pathPart.split('/').filter { it.isNotEmpty() }
        val host = segments.firstOrNull() ?: return null
        val tail = segments.drop(1)

        return when (host) {
            "listing" -> tail.firstOrNull()?.let { DeepLinkTarget.Listing(it) }
            "search" -> DeepLinkTarget.Search
            "favorites" -> DeepLinkTarget.Favorites
            "inquiries" -> DeepLinkTarget.Inquiries
            "sso" ->
                queryParam(queryPart, "token")?.let { token ->
                    DeepLinkTarget.Sso(token, queryParam(queryPart, "state"))
                }
            else -> null
        }
    }

    /**
     * Resolve a [target] to the nav-graph route string to navigate to, or `null` for targets that
     * are handled out-of-band (e.g. [DeepLinkTarget.Sso], which validates a token instead).
     */
    fun route(target: DeepLinkTarget): String? =
        when (target) {
            is DeepLinkTarget.Listing -> "listing/${target.id}"
            DeepLinkTarget.Search -> ROUTE_SEARCH
            DeepLinkTarget.Favorites -> ROUTE_FAVORITES
            DeepLinkTarget.Inquiries -> ROUTE_INQUIRIES
            is DeepLinkTarget.Sso -> null
        }

    private fun queryParam(query: String, key: String): String? {
        if (query.isEmpty()) return null
        for (pair in query.split('&')) {
            val eq = pair.indexOf('=')
            if (eq <= 0) continue
            // The key itself may be percent-encoded; decode before comparing.
            if (urlDecode(pair.substring(0, eq)) == key) {
                val value = urlDecode(pair.substring(eq + 1))
                return value.ifEmpty { null }
            }
        }
        return null
    }

    /**
     * Percent-decode a query-component string using `application/x-www-form-urlencoded` semantics:
     * `+` decodes to a space and each `%XX` triplet decodes to the byte `0xXX`, with the resulting
     * byte sequence interpreted as UTF-8.
     *
     * This mirrors Android's `android.net.Uri.getQueryParameter`, which decodes its result. The
     * shared router previously returned the raw, still-encoded substring, so the same
     * `reality://sso` deep link produced a decoded token on Android but an encoded token on the
     * shared/iOS path — breaking SSO validation on iOS. Decoding here makes every platform agree on
     * the token string.
     *
     * Malformed escapes (a `%` not followed by two hex digits) are passed through literally rather
     * than throwing, matching the lenient behaviour of platform URI decoders.
     */
    private fun urlDecode(value: String): String {
        // Fast path: nothing to decode.
        if (value.indexOf('%') < 0 && value.indexOf('+') < 0) return value

        val bytes = ArrayList<Byte>(value.length)
        var i = 0
        while (i < value.length) {
            when (val c = value[i]) {
                '+' -> {
                    bytes.add(' '.code.toByte())
                    i += 1
                }
                '%' -> {
                    val hi = if (i + 1 < value.length) hexDigit(value[i + 1]) else -1
                    val lo = if (i + 2 < value.length) hexDigit(value[i + 2]) else -1
                    if (hi >= 0 && lo >= 0) {
                        bytes.add(((hi shl 4) or lo).toByte())
                        i += 3
                    } else {
                        // Not a valid escape — keep the literal '%'.
                        bytes.add(c.code.toByte())
                        i += 1
                    }
                }
                else -> {
                    // Append the raw UTF-8 bytes of this character.
                    for (b in c.toString().encodeToByteArray()) bytes.add(b)
                    i += 1
                }
            }
        }
        return bytes.toByteArray().decodeToString()
    }

    /** Return the value 0..15 for a hex digit char, or -1 if [c] is not a hex digit. */
    private fun hexDigit(c: Char): Int =
        when (c) {
            in '0'..'9' -> c - '0'
            in 'a'..'f' -> c - 'a' + 10
            in 'A'..'F' -> c - 'A' + 10
            else -> -1
        }
}
