package three.two.bit.ppt.reality.ui.listing

import android.content.Intent
import android.net.Uri
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import three.two.bit.ppt.reality.R
import three.two.bit.ppt.reality.api.ApiConfig
import three.two.bit.ppt.reality.auth.AuthState
import three.two.bit.ppt.reality.auth.SsoService
import three.two.bit.ppt.reality.listing.*
import three.two.bit.ppt.reality.util.isNetworkError

/**
 * Listing Detail screen — KMP / Compose M3 redesign matching the design bundle
 * (`guest-registration-v2-design-system/project/ui_kits/mobile-native/ screens.jsx` →
 * `KmpListingDetailScreen`).
 *
 * Layout (top → bottom):
 * 1. Hero gallery — 280dp, full-bleed swipeable images, white-pill back (top-left), share + heart
 *    pills (top-right), page dots (bottom- center) + counter "1 / N" (bottom-right).
 * 2. Sticky agent bar — gradient avatar, realtor name + "Verified" pill, agency caption, primary
 *    "Call" pill.
 * 3. Header section — Featured + transaction-type uppercase badges, large display price + €/m²
 *    subline, title, location.
 * 4. Quick stats strip — 4 small cards (Area, Year built, Energy rating, Floor) with icon + value +
 *    uppercase label.
 * 5. Tab strip — Prehľad / Building / Nearby / Price history (active tab gets brand-600 ink + 2dp
 *    underline).
 * 6. Description body.
 * 7. Building passport card — 2-col grid of K/V labels (year, floor, energy, parking, etc) reusing
 *    the existing `ListingDetail` fields.
 * 8. Features chip wrap (preserved from previous impl).
 * 9. Bottom action bar — circle "Message" tile + full-width primary "Žiadať obhliadku".
 *
 * Inquiry dialog + Share sheet preserved from the prior implementation — the design references both
 * as flows (E2 / F2) that live in screens-extension.jsx and aren't fully re-mocked here.
 *
 * Epic 48 - Story 48.2: Portal Mobile Listing View.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalFoundationApi::class)
@Composable
fun ListingDetailScreen(
    listingId: String,
    repository: ListingRepository,
    ssoService: SsoService,
    onBackClick: () -> Unit,
    onInquirySuccess: () -> Unit,
) {
    val scope = rememberCoroutineScope()
    val context = LocalContext.current
    val authState by ssoService.authState.collectAsState()
    val networkErrorMsg = stringResource(R.string.error_network)
    val genericErrorMsg = stringResource(R.string.error_generic)

    val sessionToken = (authState as? AuthState.Authenticated)?.sessionToken
    // State + side-effects are hoisted into ListingDetailViewModel (commonMain). This composable
    // is now a `collectAsState()` render + intent wiring — see issue #2079. The VM is keyed only on
    // `listingId`: an auth change no longer re-creates it (which would flip the whole screen back
    // to
    // the full-screen spinner and reload the listing — issue #2108). Instead `updateAuth` below
    // rebuilds the auth-scoped repositories and re-syncs just the favorite heart.
    val viewModel =
        remember(listingId) {
            ListingDetailViewModel.create(
                listingId = listingId,
                listingRepository = repository,
                baseUrl = ApiConfig.requireBaseUrl(),
                sessionToken = sessionToken,
                scope = scope,
                errorMapper = { e -> if (e.isNetworkError()) networkErrorMsg else genericErrorMsg },
            )
        }
    val state by viewModel.state.collectAsState()

    LaunchedEffect(viewModel) { viewModel.start() }
    // Re-key the auth-scoped repos + re-probe the favorite heart on login/logout, without
    // disturbing the loaded listing. No-op on first composition (token unchanged).
    LaunchedEffect(sessionToken) { viewModel.updateAuth(sessionToken) }
    LaunchedEffect(viewModel) {
        viewModel.events.collect { event ->
            when (event) {
                ListingDetailEvent.InquirySubmitted -> onInquirySuccess()
            }
        }
    }

    Surface(modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
        when {
            state.isLoading ->
                Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    CircularProgressIndicator()
                }
            state.errorMessage != null ->
                ErrorContent(message = state.errorMessage!!, onRetry = viewModel::retry)
            state.listing != null ->
                Box(modifier = Modifier.fillMaxSize()) {
                    ListingContent(
                        listing = state.listing!!,
                        isFavorite = state.isFavorite,
                        onBackClick = onBackClick,
                        onShareClick = viewModel::onShowShareSheet,
                        onFavoriteClick = viewModel::onFavoriteToggle,
                        onCallClick = {
                            state.listing!!.realtor?.phone?.let { phone ->
                                context.startActivity(
                                    Intent(Intent.ACTION_DIAL).apply {
                                        data = Uri.parse("tel:$phone")
                                    }
                                )
                            }
                        },
                        onMessageClick = viewModel::onShowInquiryDialog,
                        onInquiryClick = viewModel::onShowInquiryDialog,
                    )
                }
        }
    }

    if (state.showInquiryDialog && state.listing != null) {
        InquiryDialog(
            listing = state.listing!!,
            isAuthenticated = authState is AuthState.Authenticated,
            isSubmitting = state.isInquirySubmitting,
            errorMessage = state.inquiryError,
            onDismiss = viewModel::onDismissInquiryDialog,
            onSubmit = { message, name, email, phone ->
                viewModel.onSubmitInquiry(message, name, email, phone)
            },
        )
    }
    if (state.showShareSheet && state.listing != null) {
        ShareListingSheet(listing = state.listing!!, onDismiss = viewModel::onDismissShareSheet)
    }
}

// ─── Error ──────────────────────────────────────────────────────────────────

@Composable
private fun ErrorContent(message: String, onRetry: () -> Unit, modifier: Modifier = Modifier) {
    Column(
        modifier = modifier.fillMaxSize().padding(32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Icon(
            Icons.Default.Error,
            contentDescription = null,
            modifier = Modifier.size(64.dp),
            tint = MaterialTheme.colorScheme.error,
        )
        Spacer(modifier = Modifier.height(16.dp))
        Text(
            text = message,
            style = MaterialTheme.typography.bodyLarge,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(modifier = Modifier.height(16.dp))
        Button(onClick = onRetry) { Text(stringResource(R.string.retry)) }
    }
}
