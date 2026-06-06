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
     * SSO callback: `reality://sso?token=…`. Handled by validating the token, not by navigation.
     */
    data class Sso(val token: String) : DeepLinkTarget()
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
            "sso" -> queryParam(queryPart, "token")?.let { DeepLinkTarget.Sso(it) }
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
            if (pair.substring(0, eq) == key) {
                val value = pair.substring(eq + 1)
                return value.ifEmpty { null }
            }
        }
        return null
    }
}
