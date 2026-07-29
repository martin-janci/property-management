package three.two.bit.ppt.reality

import android.content.Intent
import android.os.Bundle
import android.util.Log
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.lifecycle.lifecycleScope
import androidx.navigation.NavHostController
import androidx.navigation.compose.rememberNavController
import kotlinx.coroutines.launch
import three.two.bit.ppt.reality.api.ApiConfig
import three.two.bit.ppt.reality.auth.SsoService
import three.two.bit.ppt.reality.auth.SsoStateStore
import three.two.bit.ppt.reality.listing.ListingRepository
import three.two.bit.ppt.reality.navigation.DeepLinkRouter
import three.two.bit.ppt.reality.navigation.DeepLinkTarget
import three.two.bit.ppt.reality.navigation.RealityNavHost
import three.two.bit.ppt.reality.ui.theme.RealityPortalTheme

/**
 * Main activity for Reality Portal mobile app.
 *
 * Epic 48: Reality Portal Mobile (KMP) Epic 122: Push Notification Deep Links
 */
class MainActivity : ComponentActivity() {
    private val ssoService = SsoService()

    /** Pending deep link to navigate to after NavHost is ready */
    private var pendingDeepLink = mutableStateOf<DeepLinkTarget?>(null)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // API configuration is now provided by PlatformConfig using expect/actual pattern.
        // See: shared/src/androidMain/kotlin/.../api/PlatformConfig.kt
        // The API_BASE_URL is configured via Gradle product flavors in shared/build.gradle.kts:
        // - Development: http://10.0.2.2:8081 (Android emulator localhost)
        // - Staging: https://staging-api.realityportal.example.com
        // - Production: https://api.realityportal.example.com
        require(ApiConfig.isInitialized) { "ApiConfig not initialized - check PlatformConfig" }

        // Handle deep-link on initial launch
        handleDeepLink(intent)

        setContent {
            RealityPortalTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background,
                ) {
                    RealityPortalApp(
                        ssoService = ssoService,
                        pendingDeepLink = pendingDeepLink.value,
                        onDeepLinkHandled = { pendingDeepLink.value = null },
                    )
                }
            }
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        // Handle deep-link when app is already running
        handleDeepLink(intent)
    }

    /**
     * Dispatch an incoming `reality://…` intent through the shared [DeepLinkRouter] — the single
     * source of truth for deep-link scheme/host/path/query parsing (shared across Android + iOS).
     *
     * Android contributes only the platform-specific bit: pulling the URI string out of the
     * [Intent]. All scheme/host matching and percent-decoding lives in [DeepLinkRouter.parse] so a
     * new shared [DeepLinkTarget] (e.g. [DeepLinkTarget.Sso]) is honoured here automatically rather
     * than silently no-op-ing on Android.
     */
    private fun handleDeepLink(intent: Intent?) {
        val uri = intent?.data ?: return

        when (val target = DeepLinkRouter.parse(uri.toString())) {
            // SSO is handled out-of-band: validate the token instead of navigating.
            //
            // CSRF / session-fixation guard (parity with iOS `AuthManager.consumeSsoState`): the
            // `reality://sso` intent-filter is `exported`, so any browser, notification, or other
            // app can deliver this intent carrying an attacker-controlled token. Only validate the
            // token when the callback's `state` matches the per-flow nonce this app minted (via
            // `SsoStateStore.mint`) before starting the SSO hop. A missing/forged `state` — or an
            // unsolicited deep link with no pending nonce at all — is rejected here, before the
            // token ever reaches the session-creating endpoint.
            is DeepLinkTarget.Sso ->
                if (SsoStateStore.consume(target.state)) {
                    lifecycleScope.launch { ssoService.validateAndLogin(target.token) }
                } else {
                    Log.w(TAG, "Rejected SSO deep link: CSRF state mismatch or no pending flow")
                }
            // Navigable targets are deferred until the NavHost is composed.
            null -> Unit
            else -> pendingDeepLink.value = target
        }
    }

    private companion object {
        private const val TAG = "MainActivity"
    }
}

@Composable
fun RealityPortalApp(
    ssoService: SsoService,
    pendingDeepLink: DeepLinkTarget? = null,
    onDeepLinkHandled: () -> Unit = {},
) {
    val navController = rememberNavController()

    // Repositories — in production these would be injected via DI.
    // The session token comes from SsoService's `authState`; we re-derive
    // (and re-create the repos) when it flips between Unauthenticated and
    // Authenticated so authenticated calls carry the bearer header.
    val baseUrl = ApiConfig.requireBaseUrl()
    val authState by ssoService.authState.collectAsState()
    val sessionToken =
        (authState as? three.two.bit.ppt.reality.auth.AuthState.Authenticated)?.sessionToken
    val listingRepository =
        remember(sessionToken) { ListingRepository(baseUrl = baseUrl, sessionToken = sessionToken) }
    val favoritesRepository =
        remember(sessionToken) {
            three.two.bit.ppt.reality.favorites.FavoritesRepository(
                baseUrl = baseUrl,
                sessionToken = sessionToken,
            )
        }
    val inquiryRepository =
        remember(sessionToken) {
            three.two.bit.ppt.reality.inquiry.InquiryRepository(
                baseUrl = baseUrl,
                sessionToken = sessionToken,
            )
        }
    val portalListingsRepository =
        remember(sessionToken) {
            three.two.bit.ppt.reality.realtor.PortalListingsRepository(
                baseUrl = baseUrl,
                sessionToken = sessionToken,
            )
        }

    // Handle pending deep link navigation (Epic 122)
    LaunchedEffect(pendingDeepLink) {
        pendingDeepLink?.let { target ->
            navigateToDeepLink(navController, target)
            onDeepLinkHandled()
        }
    }

    RealityNavHost(
        navController = navController,
        ssoService = ssoService,
        listingRepository = listingRepository,
        favoritesRepository = favoritesRepository,
        inquiryRepository = inquiryRepository,
        portalListingsRepository = portalListingsRepository,
    )
}

/**
 * Navigate to a deep link target (Epic 122). Route resolution is delegated to the shared
 * [DeepLinkRouter] (commonMain) so Android and iOS resolve the same nav-graph routes (story 82-2,
 * AC-4). Out-of-band targets (e.g. [DeepLinkTarget.Sso]) resolve to a `null` route and are no-ops
 * here — they are handled before navigation in [MainActivity.handleDeepLink].
 */
private fun navigateToDeepLink(navController: NavHostController, target: DeepLinkTarget) {
    DeepLinkRouter.route(target)?.let { route -> navController.navigate(route) }
}
