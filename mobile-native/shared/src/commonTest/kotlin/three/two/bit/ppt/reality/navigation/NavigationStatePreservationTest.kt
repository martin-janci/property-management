package three.two.bit.ppt.reality.navigation

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

/**
 * Story 82-2, AC-4 — behaviour proof for **navigation state preservation** on Reality mobile.
 *
 * The Compose nav graph (`androidApp` `Navigation.kt`) preserves each bottom-bar tab's back stack
 * by driving every tab tap through one shared set of nav options:
 * ```kotlin
 * navController.navigate(route) {
 *     popUpTo(DeepLinkRouter.TAB_NAV_OPTIONS.popUpToRoute) {
 *         saveState = DeepLinkRouter.TAB_NAV_OPTIONS.popUpToSaveState
 *     }
 *     launchSingleTop = DeepLinkRouter.TAB_NAV_OPTIONS.launchSingleTop
 *     restoreState = DeepLinkRouter.TAB_NAV_OPTIONS.restoreState
 * }
 * ```
 *
 * The Compose `navigate { ... }` lambda itself needs the Android `NavController` (and therefore an
 * instrumented test) to exercise, but the *correctness of state preservation* lives entirely in
 * which option flags are set. By pinning those flags in framework-free
 * [DeepLinkRouter.TabNavOptions] the graph consumes, this JVM unit test proves the contract without
 * ADB:
 * - `saveState`/`restoreState` true ⇒ a tab the user drilled into is restored when re-selected (the
 *   AC-4 requirement).
 * - `popUpTo(home)` ⇒ each tab is a single saved sub-stack rooted at the start destination, so the
 *   back stack can't grow unbounded as the user hops tabs.
 * - `launchSingleTop` ⇒ re-tapping the current tab doesn't push a duplicate copy.
 *
 * If any of these flags regress (e.g. an edit drops `restoreState`), this test fails — which is the
 * proof that was missing before.
 */
class NavigationStatePreservationTest {

    @Test
    fun tab_nav_options_save_and_restore_state() {
        val opts = DeepLinkRouter.TAB_NAV_OPTIONS
        // Saving on the way out + restoring on the way in is what makes a tab return the user to
        // where they left off — the core of AC-4 navigation state preservation.
        assertTrue(opts.popUpToSaveState, "tab switches must save back-stack state")
        assertTrue(opts.restoreState, "tab switches must restore back-stack state")
    }

    @Test
    fun tab_nav_options_pop_up_to_home_single_top() {
        val opts = DeepLinkRouter.TAB_NAV_OPTIONS
        // Each tab is a saved sub-stack rooted at the start destination so the stack stays bounded.
        assertEquals(DeepLinkRouter.ROUTE_HOME, opts.popUpToRoute)
        // Re-tapping the active tab must not stack a duplicate copy.
        assertTrue(opts.launchSingleTop, "tab switches must be single-top")
    }

    @Test
    fun pop_up_to_target_is_a_state_preserving_top_level_tab() {
        // The route we pop up to must itself be one of the state-preserving tabs, otherwise the
        // saved/restored sub-stacks wouldn't hang off a preserved root.
        assertTrue(DeepLinkRouter.preservesState(DeepLinkRouter.TAB_NAV_OPTIONS.popUpToRoute))
    }

    @Test
    fun only_top_level_tabs_preserve_state() {
        // Sanity-pin the partition the graph relies on: every bottom-bar tab preserves state...
        for (tab in DeepLinkRouter.TOP_LEVEL_ROUTES) {
            assertTrue(DeepLinkRouter.preservesState(tab), "$tab is a tab and must preserve state")
        }
        // ...and detail/auth/realtor routes do not, so they aren't saved/restored as tabs.
        assertFalse(DeepLinkRouter.preservesState("listing/abc-123"))
        assertFalse(DeepLinkRouter.preservesState("auth/login"))
        assertFalse(DeepLinkRouter.preservesState("realtor/listings"))
        assertFalse(DeepLinkRouter.preservesState(null))
    }
}
