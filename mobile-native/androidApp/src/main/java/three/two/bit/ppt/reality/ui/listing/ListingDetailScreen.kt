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
import kotlinx.coroutines.launch
import three.two.bit.ppt.reality.R
import three.two.bit.ppt.reality.api.ApiConfig
import three.two.bit.ppt.reality.auth.AuthState
import three.two.bit.ppt.reality.auth.SsoService
import three.two.bit.ppt.reality.favorites.FavoritesRepository
import three.two.bit.ppt.reality.inquiry.CreateInquiryRequest
import three.two.bit.ppt.reality.inquiry.InquiryRepository
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
    fun friendlyError(e: Throwable) = if (e.isNetworkError()) networkErrorMsg else genericErrorMsg

    var listing by remember { mutableStateOf<ListingDetail?>(null) }
    var isLoading by remember { mutableStateOf(true) }
    var errorMessage by remember { mutableStateOf<String?>(null) }
    var isFavorite by remember { mutableStateOf(false) }
    var isFavoriteLoading by remember { mutableStateOf(false) }
    var showInquiryDialog by remember { mutableStateOf(false) }
    var isInquirySubmitting by remember { mutableStateOf(false) }
    var inquiryError by remember { mutableStateOf<String?>(null) }
    var showShareSheet by remember { mutableStateOf(false) }

    val sessionToken = (authState as? AuthState.Authenticated)?.sessionToken
    val favoritesRepository =
        remember(sessionToken) {
            FavoritesRepository(baseUrl = ApiConfig.requireBaseUrl(), sessionToken = sessionToken)
        }
    val inquiryRepository =
        remember(sessionToken) {
            InquiryRepository(baseUrl = ApiConfig.requireBaseUrl(), sessionToken = sessionToken)
        }

    LaunchedEffect(listingId, authState) {
        if (authState is AuthState.Authenticated) {
            favoritesRepository
                .isFavorite(listingId)
                .fold(onSuccess = { isFavorite = it }, onFailure = { /* non-critical */ })
        }
    }
    LaunchedEffect(listingId) {
        isLoading = true
        repository
            .getListingDetail(listingId)
            .fold(
                onSuccess = {
                    listing = it
                    isLoading = false
                },
                onFailure = {
                    errorMessage = friendlyError(it)
                    isLoading = false
                },
            )
    }

    Surface(modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
        when {
            isLoading ->
                Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    CircularProgressIndicator()
                }
            errorMessage != null ->
                ErrorContent(
                    message = errorMessage!!,
                    onRetry = {
                        scope.launch {
                            isLoading = true
                            errorMessage = null
                            repository
                                .getListingDetail(listingId)
                                .fold(
                                    onSuccess = { listing = it },
                                    onFailure = { errorMessage = friendlyError(it) },
                                )
                            isLoading = false
                        }
                    },
                )
            listing != null ->
                Box(modifier = Modifier.fillMaxSize()) {
                    ListingContent(
                        listing = listing!!,
                        isFavorite = isFavorite,
                        onBackClick = onBackClick,
                        onShareClick = { showShareSheet = true },
                        onFavoriteClick = {
                            if (authState is AuthState.Authenticated && !isFavoriteLoading) {
                                val previousFav = isFavorite
                                val newFav = !isFavorite
                                // Optimistic UI: flip the heart immediately, then reconcile with
                                // the server response. On failure we roll the state back to
                                // `previousFav` so the icon never lies about persisted state.
                                isFavorite = newFav
                                isFavoriteLoading = true
                                scope.launch {
                                    val result =
                                        if (newFav) favoritesRepository.addFavorite(listingId)
                                        else favoritesRepository.removeFavorite(listingId)
                                    result.fold(
                                        onSuccess = { /* optimistic value already applied */ },
                                        onFailure = { isFavorite = previousFav },
                                    )
                                    isFavoriteLoading = false
                                }
                            }
                        },
                        onCallClick = {
                            listing!!.realtor?.phone?.let { phone ->
                                context.startActivity(
                                    Intent(Intent.ACTION_DIAL).apply {
                                        data = Uri.parse("tel:$phone")
                                    }
                                )
                            }
                        },
                        onMessageClick = { showInquiryDialog = true },
                        onInquiryClick = { showInquiryDialog = true },
                    )
                }
        }
    }

    if (showInquiryDialog && listing != null) {
        InquiryDialog(
            listing = listing!!,
            isAuthenticated = authState is AuthState.Authenticated,
            isSubmitting = isInquirySubmitting,
            errorMessage = inquiryError,
            onDismiss = {
                showInquiryDialog = false
                inquiryError = null
            },
            onSubmit = { message, name, email, phone ->
                isInquirySubmitting = true
                inquiryError = null
                scope.launch {
                    val request =
                        CreateInquiryRequest(
                            listingId = listingId,
                            message = message,
                            name = name,
                            email = email,
                            phone = phone,
                        )
                    inquiryRepository
                        .createInquiry(request)
                        .fold(
                            onSuccess = {
                                isInquirySubmitting = false
                                showInquiryDialog = false
                                onInquirySuccess()
                            },
                            onFailure = { error ->
                                isInquirySubmitting = false
                                inquiryError = friendlyError(error)
                            },
                        )
                }
            },
        )
    }
    if (showShareSheet && listing != null) {
        ShareListingSheet(listing = listing!!, onDismiss = { showShareSheet = false })
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
