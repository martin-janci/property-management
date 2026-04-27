package three.two.bit.ppt.reality.navigation

import android.net.Uri
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.navigation.NavHostController
import androidx.navigation.NavType
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import androidx.navigation.navArgument
import kotlinx.coroutines.launch
import three.two.bit.ppt.reality.auth.SsoService
import three.two.bit.ppt.reality.favorites.FavoritesRepository
import three.two.bit.ppt.reality.favorites.SavedSearch as ApiSavedSearch
import three.two.bit.ppt.reality.inquiry.InquiryRepository
import three.two.bit.ppt.reality.listing.ListingRepository
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
import three.two.bit.ppt.reality.ui.realtor.CreateListingScreen
import three.two.bit.ppt.reality.ui.realtor.ListingAnalyticsScreen
import three.two.bit.ppt.reality.ui.realtor.MyListingsScreen
import three.two.bit.ppt.reality.ui.savedsearches.SavedSearchesScreen
import three.two.bit.ppt.reality.ui.search.SearchScreen

/**
 * Navigation routes for Reality Portal.
 *
 * Epic 48 - All Stories
 */
sealed class Screen(val route: String) {
    data object Home : Screen("home")

    data object Search : Screen("search")

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
    startDestination: String = Screen.Home.route,
) {
    NavHost(navController = navController, startDestination = startDestination) {
        composable(Screen.Home.route) {
            HomeScreen(
                repository = listingRepository,
                ssoService = ssoService,
                onSearchClick = { navController.navigate(Screen.Search.route) },
                onListingClick = { id ->
                    navController.navigate(Screen.ListingDetail.createRoute(id))
                },
                onFavoritesClick = { navController.navigate(Screen.Favorites.route) },
                onAccountClick = { navController.navigate(Screen.Account.route) },
                onInquiriesClick = { navController.navigate(Screen.Inquiries.route) }
            )
        }

        composable(Screen.Search.route) {
            SearchScreen(
                repository = listingRepository,
                onListingClick = { id ->
                    navController.navigate(Screen.ListingDetail.createRoute(id))
                },
                onBackClick = { navController.popBackStack() }
            )
        }

        composable(
            route = Screen.ListingDetail.route,
            arguments = listOf(navArgument("listingId") { type = NavType.StringType })
        ) { backStackEntry ->
            val listingId = backStackEntry.arguments?.getString("listingId") ?: return@composable
            ListingDetailScreen(
                listingId = listingId,
                repository = listingRepository,
                ssoService = ssoService,
                onBackClick = { navController.popBackStack() },
                onInquirySuccess = { navController.navigate(Screen.Inquiries.route) }
            )
        }

        composable(Screen.Favorites.route) {
            FavoritesScreen(
                repository = listingRepository,
                ssoService = ssoService,
                onListingClick = { id ->
                    navController.navigate(Screen.ListingDetail.createRoute(id))
                },
                onBackClick = { navController.popBackStack() }
            )
        }

        composable(Screen.Account.route) {
            AccountScreen(
                ssoService = ssoService,
                onBackClick = { navController.popBackStack() },
                onLogout = {
                    ssoService.logout()
                    navController.popBackStack(Screen.Home.route, inclusive = false)
                }
            )
        }

        composable(Screen.Inquiries.route) {
            InquiriesScreen(
                repository = listingRepository,
                ssoService = ssoService,
                onListingClick = { id ->
                    navController.navigate(Screen.ListingDetail.createRoute(id))
                },
                onBackClick = { navController.popBackStack() }
            )
        }

        // Auth & profile (UC-47).
        composable(Screen.Login.route) {
            LoginScreen(
                onBackClick = { navController.popBackStack() },
                onSubmit = { email, password ->
                    ssoService.loginWithPassword(email, password).map {}
                },
                onForgotPasswordClick = { navController.navigate(Screen.ForgotPassword.route) },
                onRegisterClick = { navController.navigate(Screen.Register.route) },
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
                        favoritesRepository.toggleSearchAlert(id, enabled).onSuccess { updated ->
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
            // Load the realtor's inquiries via InquiryRepository. Tapping
            // a row drops the user back into the resident-side
            // `Screen.Inquiries` list scoped to that one inquiry; a
            // dedicated agency-side detail screen is a separate task.
            var inquiries by remember {
                mutableStateOf<List<three.two.bit.ppt.reality.ui.agency.AgencyInquiry>>(emptyList())
            }
            var isLoading by remember { mutableStateOf(true) }

            LaunchedEffect(Unit) {
                isLoading = true
                inquiryRepository
                    .getInquiries()
                    .onSuccess { resp ->
                        inquiries =
                            resp.inquiries.map { i ->
                                three.two.bit.ppt.reality.ui.agency.AgencyInquiry(
                                    id = i.id,
                                    listingTitle = i.listing?.title ?: "Listing #${i.listingId}",
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
            MyListingsScreen(
                listings = emptyList(),
                isLoading = false,
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
            ListingAnalyticsScreen(
                metrics = emptyList(),
                isLoading = false,
                onBackClick = { navController.popBackStack() },
            )
        }
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
