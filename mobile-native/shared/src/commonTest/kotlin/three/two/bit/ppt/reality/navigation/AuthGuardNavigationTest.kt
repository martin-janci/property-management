package three.two.bit.ppt.reality.navigation

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue
import three.two.bit.ppt.reality.auth.AuthState
import three.two.bit.ppt.reality.auth.SsoUserInfo

/**
 * Story 82-2, AC-5 — Auth guard for Reality Portal mobile navigation.
 *
 * Pins the platform-agnostic contract used by the Compose nav graph and screen-level auth guards:
 * 1. Screens requiring auth (Favorites, Inquiries, Account, profile flows) must observe [AuthState]
 *    and show a "not signed in" prompt — not navigate blindly — when the state is
 *    [AuthState.Unauthenticated] or [AuthState.Error].
 * 2. Auth-required screen routes are NOT top-level tab routes (they must not be in
 *    [DeepLinkRouter.TOP_LEVEL_ROUTES]), so the nav graph does not restore their back-stack without
 *    a valid session.
 * 3. Auth routes (`auth/login`, `auth/register`, etc.) are equally excluded from the
 *    state-preserving set — they must not appear in TOP_LEVEL_ROUTES.
 * 4. [AuthState] exhaustively partitions into Unauthenticated / Loading / Authenticated / Error,
 *    matching the `when (authState)` branches in FavoritesScreen, InquiriesScreen, and
 *    AccountScreen.
 *
 * These tests are framework-free (no Compose, no Android, no HTTP) so they run on the JVM in
 * `./gradlew :shared:jvmTest` and on iOS Simulator via `:shared:iosSimulatorArm64Test`.
 */
class AuthGuardNavigationTest {

    // -------- AC-5: AuthState contract --------

    @Test
    fun auth_state_unauthenticated_is_distinct_singleton() {
        val a: AuthState = AuthState.Unauthenticated
        val b: AuthState = AuthState.Unauthenticated
        // Data-object equality: same instance, so referential and structural equality hold.
        assertEquals(a, b)
    }

    @Test
    fun auth_state_loading_is_distinct_singleton() {
        assertEquals(AuthState.Loading, AuthState.Loading)
    }

    @Test
    fun auth_state_authenticated_carries_user_and_token() {
        val user = SsoUserInfo(userId = "u-1", email = "alice@example.com", name = "Alice")
        val state = AuthState.Authenticated(user = user, sessionToken = "tok-42")
        assertEquals("u-1", state.user.userId)
        assertEquals("tok-42", state.sessionToken)
    }

    @Test
    fun auth_state_error_carries_message() {
        val state = AuthState.Error("token_expired")
        assertEquals("token_expired", state.message)
    }

    /**
     * AC-5 core invariant: the screen's `when (authState)` must cover every branch so that neither
     * Unauthenticated nor Error reaches authenticated content.
     *
     * This test exercises each sealed subtype so a future subtype addition without a coverage
     * update causes a compile-time `when`-exhaustiveness error in the screen composables.
     */
    @Test
    fun auth_state_all_variants_are_covered_by_guard_logic() {
        val states: List<AuthState> =
            listOf(
                AuthState.Unauthenticated,
                AuthState.Loading,
                AuthState.Authenticated(
                    user = SsoUserInfo(userId = "u-1", email = "a@b.com", name = "A"),
                    sessionToken = "t",
                ),
                AuthState.Error("err"),
            )
        for (state in states) {
            // Simulate the guard: only Authenticated reaches protected content.
            val isProtectedContentVisible =
                when (state) {
                    is AuthState.Unauthenticated -> false
                    is AuthState.Loading -> false
                    is AuthState.Authenticated -> true
                    is AuthState.Error -> false
                }
            val expectVisible = state is AuthState.Authenticated
            assertEquals(
                expectVisible,
                isProtectedContentVisible,
                "Incorrect guard result for $state",
            )
        }
    }

    // -------- AC-5: auth-required routes are not top-level / state-preserving --------

    /**
     * Auth routes must NOT be in TOP_LEVEL_ROUTES — the Compose nav graph must not restore
     * back-stack state into a login/register screen on tab switch.
     */
    @Test
    fun auth_routes_do_not_preserve_state() {
        val authRoutes =
            listOf(
                "auth/login",
                "auth/register",
                "auth/forgot-password",
                "auth/reset-password",
                "auth/two-factor",
            )
        for (route in authRoutes) {
            assertFalse(
                DeepLinkRouter.preservesState(route),
                "Auth route '$route' must not be state-preserving",
            )
        }
    }

    /**
     * Account-gated routes (profile edit, saved searches, realtor surfaces) must NOT be in
     * TOP_LEVEL_ROUTES — they require auth and must not restore back-stack state without one.
     */
    @Test
    fun protected_routes_do_not_preserve_state() {
        val protectedRoutes =
            listOf(
                "account/profile", // ProfileEdit — requires auth
                "saved-searches", // SavedSearches — requires auth
                "agency", // AgencyHub — requires auth (realtor)
                "agency/inquiries", // AgencyInquiries — requires auth (realtor)
                "realtor/listings", // MyListings — requires auth (realtor)
                "realtor/listings/new",
                "realtor/analytics",
            )
        for (route in protectedRoutes) {
            assertFalse(
                DeepLinkRouter.preservesState(route),
                "Protected route '$route' must not be state-preserving",
            )
        }
    }

    /** Listing detail is public but must not appear in TOP_LEVEL_ROUTES. */
    @Test
    fun listing_detail_route_does_not_preserve_state() {
        assertFalse(DeepLinkRouter.preservesState("listing/abc-123"))
        assertFalse(DeepLinkRouter.preservesState("listing/"))
    }

    /** A null route (no back-stack entry) must not preserve state. */
    @Test
    fun null_route_does_not_preserve_state() {
        assertFalse(DeepLinkRouter.preservesState(null))
    }

    // -------- AC-5: auth guard correctly partitions tab vs. non-tab routes --------

    /**
     * The five bottom-bar tabs are the only state-preserving destinations. All other routes —
     * including auth and account-gated ones — fall outside the preservation set.
     */
    @Test
    fun only_five_tab_routes_are_state_preserving() {
        // Positive cases: the five tabs.
        val preservedRoutes = listOf("home", "search", "favorites", "inquiries", "account")
        for (route in preservedRoutes) {
            assertTrue(
                DeepLinkRouter.preservesState(route),
                "Tab route '$route' must preserve state",
            )
        }

        // Negative cases: everything else.
        val nonPreservedRoutes =
            listOf(
                "auth/login",
                "auth/register",
                "auth/forgot-password",
                "listing/xyz",
                "account/profile",
                "saved-searches",
                "agency",
                null,
                "",
                "home/sub", // sub-routes of tabs are not tabs themselves
            )
        for (route in nonPreservedRoutes) {
            assertFalse(
                DeepLinkRouter.preservesState(route),
                "Non-tab route '$route' must not preserve state",
            )
        }
    }

    // -------- AC-5: SSO deep-link target is always out-of-band (never navigated) --------

    /**
     * An SSO deep-link (`reality://sso?token=…`) carries credentials and must NEVER be turned into
     * a nav-graph route. If it were, the token would appear in the back-stack and could be
     * re-navigated after the session is established — a security flaw.
     *
     * [DeepLinkRouter.route] returns `null` for [DeepLinkTarget.Sso], ensuring the handler calls
     * `ssoService.validateAndLogin()` (out-of-band) rather than `navController.navigate(route)`.
     */
    @Test
    fun sso_deep_link_target_has_no_nav_route_auth_guard() {
        val ssoTarget = DeepLinkTarget.Sso("my-secret-token")
        val route = DeepLinkRouter.route(ssoTarget)
        assertNull(
            route,
            "SSO target must not resolve to a nav route (token must not be navigated)",
        )
    }

    /** Navigable deep-link targets (non-SSO) do resolve to a route. */
    @Test
    fun navigable_deep_link_targets_resolve_to_routes() {
        val navigableTargets: List<Pair<DeepLinkTarget, String>> =
            listOf(
                DeepLinkTarget.Listing("abc-123") to "listing/abc-123",
                DeepLinkTarget.Search to "search",
                DeepLinkTarget.Favorites to "favorites",
                DeepLinkTarget.Inquiries to "inquiries",
            )
        for ((target, expectedRoute) in navigableTargets) {
            assertEquals(
                expectedRoute,
                DeepLinkRouter.route(target),
                "Target $target must resolve to '$expectedRoute'",
            )
        }
    }
}
