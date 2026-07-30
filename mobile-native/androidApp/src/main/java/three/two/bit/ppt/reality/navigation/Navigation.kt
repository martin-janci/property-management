package three.two.bit.ppt.reality.navigation

import android.content.Intent
import android.net.Uri
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Email
import androidx.compose.material.icons.filled.FavoriteBorder
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Person
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.Icon
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.navigation.NavHostController
import androidx.navigation.NavType
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import androidx.navigation.navArgument
import kotlinx.coroutines.launch
import three.two.bit.ppt.reality.R
import three.two.bit.ppt.reality.auth.SsoInitiation
import three.two.bit.ppt.reality.auth.SsoService
import three.two.bit.ppt.reality.favorites.FavoritesRepository
import three.two.bit.ppt.reality.favorites.SavedSearch as ApiSavedSearch
import three.two.bit.ppt.reality.inquiry.InquiryRepository
import three.two.bit.ppt.reality.listing.ListingRepository
import three.two.bit.ppt.reality.listing.ListingType
import three.two.bit.ppt.reality.listing.PropertyCategory
import three.two.bit.ppt.reality.realtor.PortalListing
import three.two.bit.ppt.reality.realtor.PortalListingsRepository
import three.two.bit.ppt.reality.ui.account.AccountScreen
import three.two.bit.ppt.reality.ui.agency.AgencyHubScreen
import three.two.bit.ppt.reality.ui.agency.AgencyInquiriesScreen
import three.two.bit.ppt.reality.ui.auth.ForgotPasswordScreen
import three.two.bit.ppt.reality.ui.auth.LoginScreen
import three.two.bit.ppt.reality.ui.auth.RegisterScreen
import three.two.bit.ppt.reality.ui.auth.ResetPasswordScreen
import three.two.bit.ppt.reality.ui.auth.TwoFactorScreen
import three.two.bit.ppt.reality.ui.favorites.FavoritesScreen
import three.two.bit.ppt.reality.ui.home.HomeScreen
import three.two.bit.ppt.reality.ui.inquiries.InquiriesScreen
import three.two.bit.ppt.reality.ui.listing.ListingDetailScreen
import three.two.bit.ppt.reality.ui.profile.ProfileEditScreen
import three.two.bit.ppt.reality.ui.realtor.AnalyticsMetric
import three.two.bit.ppt.reality.ui.realtor.CreateListingScreen
import three.two.bit.ppt.reality.ui.realtor.ListingAnalyticsScreen
import three.two.bit.ppt.reality.ui.realtor.MyListingsScreen
import three.two.bit.ppt.reality.ui.realtor.RealtorListing
import three.two.bit.ppt.reality.ui.savedsearches.SavedSearchesScreen
import three.two.bit.ppt.reality.ui.search.SearchScreen
import three.two.bit.ppt.reality.util.FormatUtils

/**
 * Navigation routes for Reality Portal.
 *
 * Epic 48 - All Stories
 */
sealed class Screen(val route: String) {
    data object Home : Screen("home")

    data object Search : Screen("search?type={type}&category={category}") {
        fun createRoute(type: ListingType? = null, category: PropertyCategory? = null): String {
            val params = buildList {
                type?.let { add("type=${it.name.lowercase()}") }
                category?.let { add("category=${it.name.lowercase()}") }
            }
            return if (params.isEmpty()) "search" else "search?${params.joinToString("&")}"
        }
    }

    data object ListingDetail : Screen("listing/{listingId}") {
        fun createRoute(listingId: String) = "listing/$listingId"
    }

    data object Favorites : Screen("favorites")

    data object Alerts : Screen("alerts")

    data object Account : Screen("account")

    data object Inquiries : Screen("inquiries")

    data object Login : Screen("auth/login")

    data object Register : Screen("auth/register")

    data object ForgotPassword : Screen("auth/forgot-password")

    data object ResetPassword : Screen("auth/reset-password?token={token}") {
        /**
         * Builds the nav route with the reset token. The token is URL-encoded because real reset
         * tokens often contain `+`, `/`, `=` or `?`, which would otherwise break
         * Compose-Navigation's argument parsing.
         */
        fun createRoute(token: String) = "auth/reset-password?token=${Uri.encode(token)}"
    }

    data object TwoFactor : Screen("auth/two-factor")

    data object ProfileEdit : Screen("account/profile")

    data object SavedSearches : Screen("saved-searches")

    data object AgencyHub : Screen("agency")

    data object AgencyInquiries : Screen("agency/inquiries")

    data object MyListings : Screen("realtor/listings")

    data object CreateListing : Screen("realtor/listings/new")

    data object ListingAnalytics : Screen("realtor/analytics")
}

@Composable
fun RealityNavHost(
    navController: NavHostController = rememberNavController(),
    ssoService: SsoService,
    listingRepository: ListingRepository,
    favoritesRepository: FavoritesRepository,
    inquiryRepository: InquiryRepository,
    portalListingsRepository: PortalListingsRepository,
    startDestination: String = Screen.Home.route,
) {
    val navBackStackEntry by navController.currentBackStackEntryAsState()
    val currentRoute = navBackStackEntry?.destination?.route
    val onSignInClick: () -> Unit = { navController.navigate(Screen.Login.route) }

    // The tab-switch nav options come from the shared DeepLinkRouter so the
    // state-preservation contract (popUpTo(home){saveState}; launchSingleTop;
    // restoreState) has one definition that is unit-tested in commonTest and
    // shared with iOS. Story 82-2, AC-4.
    val tabOptions = DeepLinkRouter.TAB_NAV_OPTIONS
    val tabNavigate: (String) -> Unit = { route ->
        navController.navigate(route) {
            popUpTo(tabOptions.popUpToRoute) { saveState = tabOptions.popUpToSaveState }
            launchSingleTop = tabOptions.launchSingleTop
            restoreState = tabOptions.restoreState
        }
    }

    Scaffold(
        bottomBar = {
            if (currentRoute in TOP_LEVEL_ROUTES) {
                RealityBottomBar(
                    currentRoute = currentRoute,
                    onHomeClick = { tabNavigate(Screen.Home.route) },
                    onSearchClick = { tabNavigate(Screen.Search.createRoute()) },
                    onFavoritesClick = { tabNavigate(Screen.Favorites.route) },
                    onInquiriesClick = { tabNavigate(Screen.Inquiries.route) },
                    onAccountClick = { tabNavigate(Screen.Account.route) },
                )
            }
        }
    ) { innerPadding ->
        NavHost(
            navController = navController,
            startDestination = startDestination,
            modifier = Modifier.padding(innerPadding),
        ) {
            composable(Screen.Home.route) {
                HomeScreen(
                    repository = listingRepository,
                    ssoService = ssoService,
                    onSearchClick = { type, category ->
                        navController.navigate(Screen.Search.createRoute(type, category))
                    },
                    onListingClick = { id ->
                        navController.navigate(Screen.ListingDetail.createRoute(id))
                    },
                    onFavoritesClick = { navController.navigate(Screen.Favorites.route) },
                    onAccountClick = { navController.navigate(Screen.Account.route) },
                    onInquiriesClick = { navController.navigate(Screen.Inquiries.route) },
                )
            }

            composable(
                route = Screen.Search.route,
                arguments =
                    listOf(
                        navArgument("type") {
                            type = NavType.StringType
                            nullable = true
                            defaultValue = null
                        },
                        navArgument("category") {
                            type = NavType.StringType
                            nullable = true
                            defaultValue = null
                        },
                    ),
            ) { entry ->
                val initialType =
                    entry.arguments?.getString("type")?.let { raw ->
                        runCatching { ListingType.valueOf(raw.uppercase()) }.getOrNull()
                    }
                val initialCategory =
                    entry.arguments?.getString("category")?.let { raw ->
                        runCatching { PropertyCategory.valueOf(raw.uppercase()) }.getOrNull()
                    }
                SearchScreen(
                    repository = listingRepository,
                    onListingClick = { id ->
                        navController.navigate(Screen.ListingDetail.createRoute(id))
                    },
                    onBackClick = { navController.popBackStack() },
                    initialType = initialType,
                    initialCategory = initialCategory,
                )
            }

            composable(
                route = Screen.ListingDetail.route,
                arguments = listOf(navArgument("listingId") { type = NavType.StringType }),
            ) { backStackEntry ->
                val listingId =
                    backStackEntry.arguments?.getString("listingId") ?: return@composable
                ListingDetailScreen(
                    listingId = listingId,
                    repository = listingRepository,
                    ssoService = ssoService,
                    onBackClick = { navController.popBackStack() },
                    onInquirySuccess = { navController.navigate(Screen.Inquiries.route) },
                )
            }

            composable(Screen.Favorites.route) {
                FavoritesScreen(
                    repository = listingRepository,
                    ssoService = ssoService,
                    onListingClick = { id ->
                        navController.navigate(Screen.ListingDetail.createRoute(id))
                    },
                    onBackClick = { navController.popBackStack() },
                    onSignInClick = onSignInClick,
                )
            }

            composable(Screen.Account.route) {
                AccountScreen(
                    ssoService = ssoService,
                    onBackClick = { navController.popBackStack() },
                    onLogout = {
                        ssoService.logout()
                        navController.popBackStack(Screen.Home.route, inclusive = false)
                    },
                    onProfileEditClick = { navController.navigate(Screen.ProfileEdit.route) },
                    onSavedSearchesClick = { navController.navigate(Screen.SavedSearches.route) },
                    onSignInClick = onSignInClick,
                )
            }

            composable(Screen.Inquiries.route) {
                InquiriesScreen(
                    repository = listingRepository,
                    ssoService = ssoService,
                    onListingClick = { id ->
                        navController.navigate(Screen.ListingDetail.createRoute(id))
                    },
                    onBackClick = { navController.popBackStack() },
                    onSignInClick = onSignInClick,
                )
            }

            // Auth & profile (UC-47).
            composable(Screen.Login.route) {
                val context = LocalContext.current
                LoginScreen(
                    onBackClick = { navController.popBackStack() },
                    onSubmit = { email, password ->
                        ssoService.loginWithPassword(email, password).map {}
                    },
                    onForgotPasswordClick = { navController.navigate(Screen.ForgotPassword.route) },
                    onRegisterClick = { navController.navigate(Screen.Register.route) },
                    // SSO initiation leg (parity with iOS `LoginView.loginWithSso`): mint the
                    // per-flow CSRF nonce and hand off to the PM app. `SsoStateStore` now has a
                    // pending nonce, so the `reality://sso` callback `MainActivity` receives passes
                    // `consume()` instead of being default-rejected. `ACTION_VIEW` fails only when
                    // the PM app isn't installed — swallow that so the email/password fallback
                    // remains usable.
                    onSsoLoginClick = {
                        runCatching {
                            context.startActivity(
                                Intent(Intent.ACTION_VIEW, Uri.parse(SsoInitiation.begin()))
                            )
                        }
                    },
                    // On success, drop the whole auth stack and land on Home. Home (and
                    // every top-level screen) collects authState, so it re-renders signed in.
                    onLoginSuccess = {
                        navController.navigate(Screen.Home.route) {
                            popUpTo(Screen.Home.route) { inclusive = true }
                            launchSingleTop = true
                        }
                    },
                )
            }
            composable(Screen.Register.route) {
                RegisterScreen(
                    onBackClick = { navController.popBackStack() },
                    onSubmit = { displayName, email, password ->
                        ssoService.register(email, password, displayName)
                    },
                    onSignInClick = { navController.popBackStack(Screen.Login.route, false) },
                )
            }
            composable(Screen.ForgotPassword.route) {
                ForgotPasswordScreen(
                    onBackClick = { navController.popBackStack() },
                    onSubmit = { email -> ssoService.requestPasswordReset(email) },
                    onSignInClick = { navController.popBackStack(Screen.Login.route, false) },
                )
            }
            composable(
                route = Screen.ResetPassword.route,
                arguments =
                    listOf(
                        navArgument("token") {
                            type = NavType.StringType
                            defaultValue = ""
                        }
                    ),
            ) { backStackEntry ->
                val token = backStackEntry.arguments?.getString("token") ?: ""
                ResetPasswordScreen(
                    token = token,
                    onBackClick = { navController.popBackStack() },
                    onSubmit = { resetToken, newPassword ->
                        ssoService.confirmPasswordReset(resetToken, newPassword)
                    },
                    onSuccess = { navController.popBackStack(Screen.Login.route, false) },
                )
            }
            composable(Screen.TwoFactor.route) {
                TwoFactorScreen(
                    onBackClick = { navController.popBackStack() },
                    onDone = { navController.popBackStack() },
                )
            }
            composable(Screen.ProfileEdit.route) {
                ProfileEditScreen(
                    ssoService = ssoService,
                    onBackClick = { navController.popBackStack() },
                    onSubmit = { displayName -> ssoService.updateProfile(displayName).map {} },
                )
            }

            // Saved searches & agency/realtor surfaces (UC-45, UC-49, UC-51).
            // Data sources will plug in once the mobile-native API client exposes
            // the corresponding endpoints; for now the screens render with empty
            // collections so the navigation graph is complete.
            composable(Screen.SavedSearches.route) {
                // Load saved searches via FavoritesRepository on first
                // composition, then expose toggle/delete callbacks that call
                // the repo and update local state on success.
                val scope = rememberCoroutineScope()
                var searches by remember { mutableStateOf<List<ApiSavedSearch>>(emptyList()) }
                var isLoading by remember { mutableStateOf(true) }

                LaunchedEffect(Unit) {
                    isLoading = true
                    favoritesRepository
                        .getSavedSearches()
                        .onSuccess { searches = it.searches }
                        .onFailure { searches = emptyList() }
                    isLoading = false
                }

                SavedSearchesScreen(
                    searches = searches.map { it.toUiModel() },
                    isLoading = isLoading,
                    onBackClick = { navController.popBackStack() },
                    onSearchClick = { _ ->
                        // The Search route doesn't yet accept saved-search
                        // parameters; navigate without filters as a baseline.
                        navController.navigate(Screen.Search.route)
                    },
                    onToggleAlerts = { id, enabled ->
                        scope.launch {
                            favoritesRepository.toggleSearchAlert(id, enabled).onSuccess { updated
                                ->
                                searches = searches.map { if (it.id == id) updated else it }
                            }
                        }
                    },
                    onDelete = { id ->
                        scope.launch {
                            favoritesRepository.deleteSavedSearch(id).onSuccess {
                                searches = searches.filterNot { it.id == id }
                            }
                        }
                    },
                )
            }
            composable(Screen.AgencyHub.route) {
                AgencyHubScreen(
                    onBackClick = { navController.popBackStack() },
                    onInquiriesClick = { navController.navigate(Screen.AgencyInquiries.route) },
                    onMyListingsClick = { navController.navigate(Screen.MyListings.route) },
                    onCreateListingClick = { navController.navigate(Screen.CreateListing.route) },
                    onAnalyticsClick = { navController.navigate(Screen.ListingAnalytics.route) },
                )
            }
            composable(Screen.AgencyInquiries.route) {
                // Load the realtor's inbox via /realtors/inquiries — that's the
                // agency-side dataset. `getInquiries()` would return the
                // signed-in user's own outbound (resident-side) inquiries
                // instead. Tapping a row drops into `Screen.Inquiries` for now;
                // a dedicated realtor-side detail screen is a separate task.
                var inquiries by remember {
                    mutableStateOf<List<three.two.bit.ppt.reality.ui.agency.AgencyInquiry>>(
                        emptyList()
                    )
                }
                var isLoading by remember { mutableStateOf(true) }

                LaunchedEffect(Unit) {
                    isLoading = true
                    inquiryRepository
                        .getRealtorInquiries()
                        .onSuccess { resp ->
                            inquiries =
                                resp.inquiries.map { i ->
                                    three.two.bit.ppt.reality.ui.agency.AgencyInquiry(
                                        id = i.id,
                                        listingTitle =
                                            i.listing?.title ?: "Listing #${i.listingId}",
                                        contactName = "",
                                        contactEmail = "",
                                        message = i.message,
                                        status = i.status.name.lowercase(),
                                    )
                                }
                        }
                        .onFailure { inquiries = emptyList() }
                    isLoading = false
                }

                AgencyInquiriesScreen(
                    inquiries = inquiries,
                    isLoading = isLoading,
                    onBackClick = { navController.popBackStack() },
                    onInquiryClick = { _ -> navController.navigate(Screen.Inquiries.route) },
                )
            }
            composable(Screen.MyListings.route) {
                // Load the caller's own listings via GET /api/v1/my/listings. The screen owns the
                // client-side status-chip filter, so we fetch the full page (limit 100) unfiltered
                // and let it slice locally. On failure (incl. 401 when signed out) we fall back to
                // the empty state the screen already renders.
                var listings by remember { mutableStateOf<List<RealtorListing>>(emptyList()) }
                var isLoading by remember { mutableStateOf(true) }

                LaunchedEffect(Unit) {
                    isLoading = true
                    portalListingsRepository
                        .listMyListings(limit = 100)
                        .onSuccess { resp ->
                            listings = resp.listings.map { it.toRealtorListing() }
                        }
                        .onFailure { listings = emptyList() }
                    isLoading = false
                }

                MyListingsScreen(
                    listings = listings,
                    isLoading = isLoading,
                    onBackClick = { navController.popBackStack() },
                    onCreateClick = { navController.navigate(Screen.CreateListing.route) },
                    onListingClick = { id ->
                        navController.navigate(Screen.ListingDetail.createRoute(id))
                    },
                )
            }
            composable(Screen.CreateListing.route) {
                CreateListingScreen(
                    onBackClick = { navController.popBackStack() },
                    onSubmit = { _ -> Result.failure(NotImplementedError("Wire to listing API")) },
                    onCreated = { navController.popBackStack(Screen.MyListings.route, false) },
                )
            }
            composable(Screen.ListingAnalytics.route) {
                // Realtor-wide analytics: the backend exposes analytics per listing, so the
                // repository fans out one call per listing and folds them into portfolio totals +
                // trends (see PortalListingsRepository.getPortfolioAnalytics).
                val viewsLabel = stringResource(R.string.realtor_analytics_metric_views)
                val inquiriesLabel = stringResource(R.string.realtor_analytics_metric_inquiries)
                val favoritesLabel = stringResource(R.string.realtor_analytics_metric_favorites)
                val listingsLabel = stringResource(R.string.realtor_analytics_metric_listings)
                val activeLabel = stringResource(R.string.realtor_analytics_metric_active)

                var metrics by remember { mutableStateOf<List<AnalyticsMetric>>(emptyList()) }
                var isLoading by remember { mutableStateOf(true) }

                LaunchedEffect(Unit) {
                    isLoading = true
                    portalListingsRepository
                        .getPortfolioAnalytics()
                        .onSuccess { p ->
                            metrics =
                                listOf(
                                    AnalyticsMetric(
                                        label = viewsLabel,
                                        value = p.totalViews.toString(),
                                        trend = p.viewsTrend,
                                    ),
                                    AnalyticsMetric(
                                        label = inquiriesLabel,
                                        value = p.totalInquiries.toString(),
                                        trend = p.inquiriesTrend,
                                    ),
                                    AnalyticsMetric(
                                        label = favoritesLabel,
                                        value = p.totalFavorites.toString(),
                                    ),
                                    AnalyticsMetric(
                                        label = listingsLabel,
                                        value = p.totalListings.toString(),
                                    ),
                                    AnalyticsMetric(
                                        label = activeLabel,
                                        value = p.activeListings.toString(),
                                    ),
                                )
                        }
                        .onFailure { metrics = emptyList() }
                    isLoading = false
                }

                ListingAnalyticsScreen(
                    metrics = metrics,
                    isLoading = isLoading,
                    onBackClick = { navController.popBackStack() },
                )
            }
        }
    }
}

/**
 * Map a wire [PortalListing] into the [RealtorListing] the MyListings screen renders.
 *
 * The list endpoint (`GET /api/v1/my/listings`) is deliberately lightweight — it carries no per-row
 * view/inquiry counters (those live behind the per-listing analytics endpoint) and no media, so
 * [RealtorListing.views]/[RealtorListing.inquiries] stay 0 and [RealtorListing.imageUrl] stays null
 * here rather than triggering an N+1 analytics fan-out just to fill two badges.
 */
private fun PortalListing.toRealtorListing(): RealtorListing =
    RealtorListing(
        id = id,
        title = title,
        formattedPrice = FormatUtils.formatPrice(price, currency),
        transactionType = transactionType,
        views = 0,
        inquiries = 0,
        status = status.replaceFirstChar { it.uppercase() },
        meta =
            listOfNotNull(
                    city.takeIf { it.isNotBlank() },
                    propertyType.takeIf { it.isNotBlank() }?.replaceFirstChar { it.uppercase() },
                    rooms?.let { "$it rooms" },
                )
                .joinToString(" · "),
        imageUrl = null,
    )

/**
 * The five tab destinations that show the persistent bottom navigation bar. Any route outside this
 * set (auth flows, listing detail, realtor surfaces) hides the bar so the screen can own its full
 * top-to-bottom layout.
 */
private val TOP_LEVEL_ROUTES =
    setOf(
        Screen.Home.route,
        Screen.Search.route,
        Screen.Favorites.route,
        Screen.Inquiries.route,
        Screen.Account.route,
    )

@Composable
private fun RealityBottomBar(
    currentRoute: String?,
    onHomeClick: () -> Unit,
    onSearchClick: () -> Unit,
    onFavoritesClick: () -> Unit,
    onInquiriesClick: () -> Unit,
    onAccountClick: () -> Unit,
) {
    NavigationBar {
        NavigationBarItem(
            selected = currentRoute == Screen.Home.route,
            onClick = onHomeClick,
            icon = { Icon(Icons.Default.Home, contentDescription = null) },
            label = { Text(stringResource(R.string.tab_home)) },
        )
        NavigationBarItem(
            selected = currentRoute == Screen.Search.route,
            onClick = onSearchClick,
            icon = { Icon(Icons.Default.Search, contentDescription = null) },
            label = { Text(stringResource(R.string.tab_search)) },
        )
        NavigationBarItem(
            selected = currentRoute == Screen.Favorites.route,
            onClick = onFavoritesClick,
            icon = { Icon(Icons.Default.FavoriteBorder, contentDescription = null) },
            label = { Text(stringResource(R.string.tab_favorites)) },
        )
        NavigationBarItem(
            selected = currentRoute == Screen.Inquiries.route,
            onClick = onInquiriesClick,
            icon = { Icon(Icons.Default.Email, contentDescription = null) },
            label = { Text(stringResource(R.string.tab_inquiries)) },
        )
        NavigationBarItem(
            selected = currentRoute == Screen.Account.route,
            onClick = onAccountClick,
            icon = { Icon(Icons.Default.Person, contentDescription = null) },
            label = { Text(stringResource(R.string.tab_account)) },
        )
    }
}

/**
 * Map the wire-level `SavedSearch` from FavoritesRepository onto the lighter UI-shape the
 * SavedSearchesScreen expects.
 */
private fun ApiSavedSearch.toUiModel(): three.two.bit.ppt.reality.ui.savedsearches.SavedSearch {
    val summaryParts = buildList {
        query?.takeIf { it.isNotBlank() }?.let { add(it) }
        filters?.city?.takeIf { it.isNotBlank() }?.let { add(it) }
        filters?.minRooms?.let { add("${it}+ rooms") }
    }
    val summary = if (summaryParts.isEmpty()) "All listings" else summaryParts.joinToString(" · ")
    return three.two.bit.ppt.reality.ui.savedsearches.SavedSearch(
        id = id,
        name = name,
        summary = summary,
        alertsEnabled = alertEnabled,
    )
}
