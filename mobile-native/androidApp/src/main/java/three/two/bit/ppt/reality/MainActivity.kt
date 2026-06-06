package three.two.bit.ppt.reality

import android.content.Intent
import android.os.Bundle
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
import three.two.bit.ppt.reality.listing.ListingRepository
import three.two.bit.ppt.reality.navigation.DeepLinkRouter
import three.two.bit.ppt.reality.navigation.DeepLinkTarget as SharedDeepLinkTarget
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

    private fun handleDeepLink(intent: Intent?) {
        val uri = intent?.data ?: return

        if (uri.scheme != "reality") return

        when (uri.host) {
            "sso" -> {
                // Handle SSO deep-link: reality://sso?token=xxx
                val token = uri.getQueryParameter("token")
                if (token != null) {
                    lifecycleScope.launch { ssoService.validateAndLogin(token) }
                }
            }
            "listing" -> {
                // Handle listing deep-link: reality://listing/{id}
                val listingId = uri.pathSegments.firstOrNull()
                if (listingId != null) {
                    pendingDeepLink.value = DeepLinkTarget.Listing(listingId)
                }
            }
            "search" -> {
                // Handle search deep-link: reality://search
                pendingDeepLink.value = DeepLinkTarget.Search
            }
            "favorites" -> {
                // Handle favorites deep-link: reality://favorites
                pendingDeepLink.value = DeepLinkTarget.Favorites
            }
            "inquiries" -> {
                // Handle inquiries deep-link: reality://inquiries
                pendingDeepLink.value = DeepLinkTarget.Inquiries
            }
        }
    }
}

/** Deep link navigation target (Epic 122) */
sealed class DeepLinkTarget {
    data class Listing(val id: String) : DeepLinkTarget()

    data object Search : DeepLinkTarget()

    data object Favorites : DeepLinkTarget()

    data object Inquiries : DeepLinkTarget()
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
    )
}

/**
 * Navigate to a deep link target (Epic 122). Route resolution is delegated to the shared
 * [DeepLinkRouter] (commonMain) so Android and iOS resolve the same nav-graph routes (story 82-2,
 * AC-4).
 */
private fun navigateToDeepLink(navController: NavHostController, target: DeepLinkTarget) {
    val shared =
        when (target) {
            is DeepLinkTarget.Listing -> SharedDeepLinkTarget.Listing(target.id)
            is DeepLinkTarget.Search -> SharedDeepLinkTarget.Search
            is DeepLinkTarget.Favorites -> SharedDeepLinkTarget.Favorites
            is DeepLinkTarget.Inquiries -> SharedDeepLinkTarget.Inquiries
        }
    DeepLinkRouter.route(shared)?.let { route -> navController.navigate(route) }
}
