package three.two.bit.ppt.reality.ui.listing

import android.content.Intent
import android.net.Uri
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.Message
import androidx.compose.material.icons.automirrored.filled.ShowChart
import androidx.compose.material.icons.automirrored.filled.TrendingDown
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil.compose.AsyncImage
import coil.request.ImageRequest
import kotlinx.coroutines.launch
import three.two.bit.ppt.reality.R
import three.two.bit.ppt.reality.api.ApiConfig
import three.two.bit.ppt.reality.auth.AuthState
import three.two.bit.ppt.reality.auth.SsoService
import three.two.bit.ppt.reality.favorites.FavoritesRepository
import three.two.bit.ppt.reality.inquiry.CreateInquiryRequest
import three.two.bit.ppt.reality.inquiry.InquiryRepository
import three.two.bit.ppt.reality.listing.*
import three.two.bit.ppt.reality.ui.theme.BadgeColors
import three.two.bit.ppt.reality.util.FormatUtils
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

// ─── Content ────────────────────────────────────────────────────────────────

@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun ListingContent(
    listing: ListingDetail,
    isFavorite: Boolean,
    onBackClick: () -> Unit,
    onShareClick: () -> Unit,
    onFavoriteClick: () -> Unit,
    onCallClick: () -> Unit,
    onMessageClick: () -> Unit,
    onInquiryClick: () -> Unit,
) {
    var selectedTab by remember { mutableIntStateOf(0) }
    Box(modifier = Modifier.fillMaxSize()) {
        LazyColumn(
            modifier = Modifier.fillMaxSize(),
            contentPadding = PaddingValues(bottom = 110.dp),
        ) {
            item {
                HeroGallery(
                    images = listing.images,
                    isFavorite = isFavorite,
                    onBackClick = onBackClick,
                    onShareClick = onShareClick,
                    onFavoriteClick = onFavoriteClick,
                )
            }
            item { StickyAgentBar(listing = listing, onCallClick = onCallClick) }
            item { HeaderSection(listing = listing) }
            item { Spacer(modifier = Modifier.height(18.dp)) }
            item { QuickStatsStrip(listing = listing) }
            item { TabStrip(selected = selectedTab, onSelect = { selectedTab = it }) }
            when (selectedTab) {
                0 -> {
                    item { DescriptionBody(description = listing.description) }
                    if (listing.features.isNotEmpty()) {
                        item { FeaturesChips(features = listing.features) }
                    }
                }
                1 -> item { BuildingPassportCard(listing = listing) }
                2 -> item { NearbyPreviewCard() }
                3 -> item { PriceHistoryCard(listing = listing) }
            }
            item { Spacer(modifier = Modifier.height(16.dp)) }
        }
        BottomActionBar(
            modifier = Modifier.align(Alignment.BottomCenter),
            onMessageClick = onMessageClick,
            onInquiryClick = onInquiryClick,
        )
    }
}

// ─── Hero gallery ───────────────────────────────────────────────────────────

@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun HeroGallery(
    images: List<ListingImage>,
    isFavorite: Boolean,
    onBackClick: () -> Unit,
    onShareClick: () -> Unit,
    onFavoriteClick: () -> Unit,
) {
    val pagerState = rememberPagerState(pageCount = { images.size.coerceAtLeast(1) })
    Box(
        modifier =
            Modifier.fillMaxWidth()
                .height(280.dp)
                .background(MaterialTheme.colorScheme.surfaceVariant)
    ) {
        if (images.isEmpty()) {
            Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                Icon(
                    Icons.Default.Image,
                    contentDescription = null,
                    modifier = Modifier.size(64.dp),
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        } else {
            HorizontalPager(state = pagerState, modifier = Modifier.fillMaxSize()) { page ->
                AsyncImage(
                    model =
                        ImageRequest.Builder(LocalContext.current)
                            .data(images[page].url)
                            .crossfade(true)
                            .build(),
                    contentDescription = images[page].caption,
                    contentScale = ContentScale.Crop,
                    modifier = Modifier.fillMaxSize(),
                )
            }
        }

        // Back pill (top-left)
        WhitePill(
            modifier = Modifier.align(Alignment.TopStart).padding(14.dp),
            icon = Icons.AutoMirrored.Filled.ArrowBack,
            tint = MaterialTheme.colorScheme.onSurface,
            onClick = onBackClick,
        )

        // Share + heart pills (top-right)
        Row(
            modifier = Modifier.align(Alignment.TopEnd).padding(14.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            WhitePill(
                icon = Icons.Default.Share,
                tint = MaterialTheme.colorScheme.onSurface,
                onClick = onShareClick,
            )
            WhitePill(
                icon = if (isFavorite) Icons.Default.Favorite else Icons.Default.FavoriteBorder,
                tint =
                    if (isFavorite) MaterialTheme.colorScheme.error
                    else MaterialTheme.colorScheme.onSurface,
                onClick = onFavoriteClick,
            )
        }

        if (images.size > 1) {
            // Page dots
            Row(
                modifier = Modifier.align(Alignment.BottomCenter).padding(14.dp),
                horizontalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                repeat(images.size) { idx ->
                    Box(
                        modifier =
                            Modifier.size(6.dp)
                                .clip(CircleShape)
                                .background(
                                    if (idx == pagerState.currentPage) Color.White
                                    else Color.White.copy(alpha = 0.5f)
                                )
                    )
                }
            }
            // Counter (bottom-right)
            Surface(
                modifier = Modifier.align(Alignment.BottomEnd).padding(14.dp),
                shape = RoundedCornerShape(999.dp),
                color = Color.Black.copy(alpha = 0.6f),
            ) {
                Text(
                    text = "${pagerState.currentPage + 1} / ${images.size}",
                    modifier = Modifier.padding(horizontal = 10.dp, vertical = 4.dp),
                    style =
                        MaterialTheme.typography.labelSmall.copy(fontWeight = FontWeight.SemiBold),
                    color = Color.White,
                )
            }
        }
    }
}

@Composable
private fun WhitePill(
    modifier: Modifier = Modifier,
    icon: ImageVector,
    tint: Color,
    onClick: () -> Unit,
) {
    Box(
        modifier =
            modifier
                .size(40.dp)
                .clip(CircleShape)
                .background(Color.White.copy(alpha = 0.92f))
                .clickable(onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        Icon(
            imageVector = icon,
            contentDescription = null,
            modifier = Modifier.size(18.dp),
            tint = tint,
        )
    }
}

// ─── Sticky agent bar ───────────────────────────────────────────────────────

@Composable
private fun StickyAgentBar(listing: ListingDetail, onCallClick: () -> Unit) {
    Surface(
        modifier = Modifier.fillMaxWidth(),
        color = MaterialTheme.colorScheme.surface,
        shadowElevation = 1.dp,
    ) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 10.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            val realtor = listing.realtor
            val initials =
                remember(realtor) {
                    realtor
                        ?.name
                        ?.split(" ")
                        ?.take(2)
                        ?.mapNotNull { it.firstOrNull()?.uppercase() }
                        ?.joinToString("")
                        .orEmpty()
                        .ifEmpty { "?" }
                }
            if (realtor?.avatarUrl != null) {
                AsyncImage(
                    model =
                        ImageRequest.Builder(LocalContext.current)
                            .data(realtor.avatarUrl)
                            .crossfade(true)
                            .build(),
                    contentDescription = realtor.name,
                    contentScale = ContentScale.Crop,
                    modifier =
                        Modifier.size(36.dp)
                            .clip(CircleShape)
                            .background(MaterialTheme.colorScheme.surfaceVariant),
                )
            } else {
                Box(
                    modifier =
                        Modifier.size(36.dp)
                            .clip(CircleShape)
                            .background(MaterialTheme.colorScheme.primaryContainer),
                    contentAlignment = Alignment.Center,
                ) {
                    Text(
                        text = initials,
                        style = MaterialTheme.typography.labelMedium,
                        fontWeight = FontWeight.SemiBold,
                        color = MaterialTheme.colorScheme.onPrimaryContainer,
                    )
                }
            }
            Spacer(modifier = Modifier.width(10.dp))
            Column(modifier = Modifier.weight(1f)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        text = realtor?.name ?: stringResource(R.string.detail_realtor_unknown),
                        style = MaterialTheme.typography.bodyMedium,
                        fontWeight = FontWeight.SemiBold,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                    Spacer(modifier = Modifier.width(5.dp))
                    SmallVerifiedPill()
                }
                realtor?.agency?.name?.let { agencyName ->
                    Text(
                        text = agencyName,
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
            if (realtor?.phone != null) {
                Surface(
                    onClick = onCallClick,
                    shape = RoundedCornerShape(999.dp),
                    color = MaterialTheme.colorScheme.primary,
                ) {
                    Row(
                        modifier = Modifier.padding(horizontal = 14.dp, vertical = 8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Icon(
                            Icons.Default.Phone,
                            contentDescription = null,
                            modifier = Modifier.size(14.dp),
                            tint = Color.White,
                        )
                        Spacer(modifier = Modifier.width(5.dp))
                        Text(
                            text = stringResource(R.string.action_call),
                            style = MaterialTheme.typography.labelMedium,
                            fontWeight = FontWeight.SemiBold,
                            color = Color.White,
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun SmallVerifiedPill() {
    Surface(shape = RoundedCornerShape(999.dp), color = Color(0xFFD1FAE5)) {
        Row(
            modifier = Modifier.padding(horizontal = 6.dp, vertical = 1.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Icon(
                Icons.Default.Check,
                contentDescription = null,
                modifier = Modifier.size(10.dp),
                tint = Color(0xFF065F46),
            )
            Spacer(modifier = Modifier.width(2.dp))
            Text(
                text = stringResource(R.string.profile_verified).uppercase(),
                style =
                    MaterialTheme.typography.labelSmall.copy(
                        fontWeight = FontWeight.Bold,
                        letterSpacing = 0.04.sp,
                        fontSize = 10.sp,
                    ),
                color = Color(0xFF065F46),
            )
        }
    }
}

// ─── Header (price + title + location + badges) ─────────────────────────────

@Composable
private fun HeaderSection(listing: ListingDetail) {
    Column(modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 14.dp)) {
        Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            if (listing.isFeatured) {
                UppercaseBadge(
                    text = stringResource(R.string.label_featured),
                    bg = BadgeColors.featuredBg,
                    ink = BadgeColors.featuredInk,
                )
            }
            UppercaseBadge(
                text =
                    when (listing.type) {
                        ListingType.SALE -> stringResource(R.string.for_sale)
                        ListingType.RENT -> stringResource(R.string.for_rent)
                    },
                bg =
                    when (listing.type) {
                        ListingType.SALE -> BadgeColors.saleBg
                        ListingType.RENT -> BadgeColors.rentBg
                    },
                ink =
                    when (listing.type) {
                        ListingType.SALE -> BadgeColors.saleInk
                        ListingType.RENT -> BadgeColors.rentInk
                    },
            )
        }
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            text = FormatUtils.formatPrice(listing.price, listing.currency),
            style = MaterialTheme.typography.headlineLarge,
            fontWeight = FontWeight.Bold,
            color = MaterialTheme.colorScheme.onSurface,
        )
        listing.pricePerSqm?.let {
            Spacer(modifier = Modifier.height(2.dp))
            Text(
                text = "${FormatUtils.formatPrice(it, listing.currency)}/m²",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        Spacer(modifier = Modifier.height(14.dp))
        Text(
            text = listing.title,
            style = MaterialTheme.typography.titleLarge,
            fontWeight = FontWeight.SemiBold,
            color = MaterialTheme.colorScheme.onSurface,
        )
        Spacer(modifier = Modifier.height(4.dp))
        Row(verticalAlignment = Alignment.CenterVertically) {
            Icon(
                Icons.Default.LocationOn,
                contentDescription = null,
                modifier = Modifier.size(13.dp),
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(modifier = Modifier.width(4.dp))
            Text(
                text = FormatUtils.buildDetailedLocationString(listing.address),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
private fun UppercaseBadge(text: String, bg: Color, ink: Color) {
    Surface(shape = RoundedCornerShape(6.dp), color = bg) {
        Text(
            text = text.uppercase(),
            modifier = Modifier.padding(horizontal = 9.dp, vertical = 3.dp),
            style =
                MaterialTheme.typography.labelSmall.copy(
                    fontWeight = FontWeight.Bold,
                    letterSpacing = 0.04.sp,
                ),
            color = ink,
        )
    }
}

// ─── Quick stats strip ──────────────────────────────────────────────────────

@Composable
private fun QuickStatsStrip(listing: ListingDetail) {
    Row(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        StatCard(
            icon = Icons.Default.SquareFoot,
            value = "${listing.areaSqm.toInt()} m²",
            label = stringResource(R.string.detail_stat_area),
            modifier = Modifier.weight(1f),
        )
        StatCard(
            icon = Icons.Default.Apartment,
            value = listing.yearBuilt?.toString() ?: "—",
            label = stringResource(R.string.detail_stat_built),
            modifier = Modifier.weight(1f),
        )
        StatCard(
            icon = Icons.Default.Bolt,
            value = listing.energyRating ?: "—",
            label = stringResource(R.string.detail_stat_energy),
            modifier = Modifier.weight(1f),
        )
        StatCard(
            icon = Icons.Default.Layers,
            value =
                listing.floor?.let { floor ->
                    listing.totalFloors?.let { "$floor/$it" } ?: "$floor"
                } ?: "—",
            label = stringResource(R.string.detail_stat_floor),
            modifier = Modifier.weight(1f),
        )
    }
}

@Composable
private fun StatCard(
    icon: ImageVector,
    value: String,
    label: String,
    modifier: Modifier = Modifier,
) {
    Card(
        modifier = modifier,
        shape = RoundedCornerShape(12.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
    ) {
        Column(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 6.dp, vertical = 10.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Icon(
                imageVector = icon,
                contentDescription = null,
                modifier = Modifier.size(18.dp),
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(modifier = Modifier.height(4.dp))
            Text(
                text = value,
                style = MaterialTheme.typography.bodyMedium,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.onSurface,
            )
            Spacer(modifier = Modifier.height(2.dp))
            Text(
                text = label.uppercase(),
                style =
                    MaterialTheme.typography.labelSmall.copy(
                        fontWeight = FontWeight.SemiBold,
                        letterSpacing = 0.04.sp,
                        fontSize = 9.sp,
                    ),
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

// ─── Tab strip ──────────────────────────────────────────────────────────────

@Composable
private fun TabStrip(selected: Int, onSelect: (Int) -> Unit) {
    val tabs =
        listOf(
            stringResource(R.string.detail_tab_overview),
            stringResource(R.string.detail_tab_building),
            stringResource(R.string.detail_tab_nearby),
            stringResource(R.string.detail_tab_price_history),
        )
    Row(
        modifier =
            Modifier.fillMaxWidth().padding(top = 18.dp).horizontalScroll(rememberScrollState())
    ) {
        tabs.forEachIndexed { index, label ->
            val isOn = selected == index
            Column(
                modifier =
                    Modifier.clickable { onSelect(index) }
                        .padding(start = if (index == 0) 16.dp else 0.dp, end = 12.dp)
            ) {
                Text(
                    text = label,
                    modifier = Modifier.padding(vertical = 10.dp),
                    style = MaterialTheme.typography.labelLarge,
                    fontWeight = if (isOn) FontWeight.SemiBold else FontWeight.Medium,
                    color =
                        if (isOn) MaterialTheme.colorScheme.primary
                        else MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Box(
                    modifier =
                        Modifier.height(2.dp)
                            .width(if (isOn) 44.dp else 0.dp)
                            .background(MaterialTheme.colorScheme.primary)
                )
            }
        }
    }
    HorizontalDivider(thickness = 1.dp, color = MaterialTheme.colorScheme.outlineVariant)
}

// ─── Description + features ─────────────────────────────────────────────────

@Composable
private fun DescriptionBody(description: String) {
    Text(
        text = description,
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 14.dp),
        style = MaterialTheme.typography.bodyMedium,
        color = MaterialTheme.colorScheme.onSurface,
    )
}

@Composable
private fun FeaturesChips(features: List<String>) {
    Column(modifier = Modifier.padding(horizontal = 16.dp)) {
        Text(
            text = stringResource(R.string.features),
            style = MaterialTheme.typography.titleSmall,
            fontWeight = FontWeight.SemiBold,
            color = MaterialTheme.colorScheme.onSurface,
        )
        Spacer(modifier = Modifier.height(8.dp))
        LazyRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            items(features) { feature ->
                AssistChip(
                    onClick = {},
                    label = { Text(feature) },
                    leadingIcon = {
                        Icon(
                            Icons.Default.Check,
                            contentDescription = null,
                            modifier = Modifier.size(16.dp),
                        )
                    },
                )
            }
        }
    }
}

// ─── Building passport tab ──────────────────────────────────────────────────

@Composable
private fun BuildingPassportCard(listing: ListingDetail) {
    val rows = buildList {
        listing.yearBuilt?.let { add(stringResource(R.string.detail_year_built) to "$it") }
        listing.yearRenovated?.let { add(stringResource(R.string.detail_year_renovated) to "$it") }
        listing.floor?.let {
            val floorText = listing.totalFloors?.let { tf -> "$it/$tf" } ?: "$it"
            add(stringResource(R.string.detail_floor) to floorText)
        }
        listing.usableAreaSqm?.let {
            add(stringResource(R.string.detail_usable_area) to "${it.toInt()} m²")
        }
        listing.landAreaSqm?.let {
            add(stringResource(R.string.detail_land_area) to "${it.toInt()} m²")
        }
        listing.energyRating?.let { add(stringResource(R.string.detail_energy_rating) to it) }
        listing.heatingType?.let { add(stringResource(R.string.detail_heating) to it) }
        listing.parking?.let { add(stringResource(R.string.detail_parking) to it) }
    }
    if (rows.isEmpty()) {
        PlaceholderSection(stringResource(R.string.detail_building_no_data))
        return
    }
    Card(
        modifier = Modifier.fillMaxWidth().padding(16.dp),
        shape = RoundedCornerShape(16.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
    ) {
        Column(modifier = Modifier.padding(14.dp)) {
            rows.chunked(2).forEachIndexed { rowIdx, pair ->
                if (rowIdx > 0) Spacer(modifier = Modifier.height(14.dp))
                Row(horizontalArrangement = Arrangement.spacedBy(14.dp)) {
                    pair.forEach { (label, value) ->
                        Column(modifier = Modifier.weight(1f)) {
                            Text(
                                text = label.uppercase(),
                                style =
                                    MaterialTheme.typography.labelSmall.copy(
                                        fontWeight = FontWeight.SemiBold,
                                        letterSpacing = 0.04.sp,
                                        fontSize = 10.sp,
                                    ),
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                            Spacer(modifier = Modifier.height(2.dp))
                            Text(
                                text = value,
                                style = MaterialTheme.typography.bodyMedium,
                                fontWeight = FontWeight.SemiBold,
                                color = MaterialTheme.colorScheme.onSurface,
                            )
                        }
                    }
                    if (pair.size == 1) Spacer(modifier = Modifier.weight(1f))
                }
            }
        }
    }
}

// ─── Price history tab ──────────────────────────────────────────────────────
//
// The reality-server listing payload exposes a single `previousPrice` alongside
// `isPriceReduced` rather than a full change log, so we render the one known
// transition when present and fall back to a proper empty state otherwise.

@Composable
private fun PriceHistoryCard(listing: ListingDetail) {
    val previous = listing.previousPrice
    if (!listing.isPriceReduced || previous == null) {
        Column(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 32.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Icon(
                Icons.AutoMirrored.Filled.ShowChart,
                contentDescription = null,
                modifier = Modifier.size(40.dp),
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(modifier = Modifier.height(8.dp))
            Text(
                text = stringResource(R.string.price_history_empty_title),
                style = MaterialTheme.typography.titleSmall,
                fontWeight = FontWeight.SemiBold,
                color = MaterialTheme.colorScheme.onSurface,
            )
            Spacer(modifier = Modifier.height(4.dp))
            Text(
                text = stringResource(R.string.price_history_empty_body),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                textAlign = TextAlign.Center,
            )
        }
        return
    }

    val delta = previous - listing.price
    Card(
        modifier = Modifier.fillMaxWidth().padding(16.dp),
        shape = RoundedCornerShape(16.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            PriceHistoryRow(
                label = stringResource(R.string.price_history_previous),
                value = FormatUtils.formatPrice(previous, listing.currency),
                muted = true,
            )
            Spacer(modifier = Modifier.height(12.dp))
            PriceHistoryRow(
                label = stringResource(R.string.price_history_current),
                value = FormatUtils.formatPrice(listing.price, listing.currency),
                muted = false,
            )
            if (delta > 0) {
                Spacer(modifier = Modifier.height(14.dp))
                Surface(shape = RoundedCornerShape(999.dp), color = Color(0xFFD1FAE5)) {
                    Row(
                        modifier = Modifier.padding(horizontal = 10.dp, vertical = 5.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Icon(
                            Icons.AutoMirrored.Filled.TrendingDown,
                            contentDescription = null,
                            tint = Color(0xFF065F46),
                            modifier = Modifier.size(14.dp),
                        )
                        Spacer(modifier = Modifier.width(4.dp))
                        Text(
                            text =
                                stringResource(
                                    R.string.price_history_reduced_by,
                                    FormatUtils.formatPrice(delta, listing.currency),
                                ),
                            style = MaterialTheme.typography.labelMedium,
                            fontWeight = FontWeight.SemiBold,
                            color = Color(0xFF065F46),
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun PriceHistoryRow(label: String, value: String, muted: Boolean) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = label,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            text = value,
            style =
                if (muted) MaterialTheme.typography.bodyMedium
                else MaterialTheme.typography.titleMedium,
            fontWeight = if (muted) FontWeight.Normal else FontWeight.Bold,
            color =
                if (muted) MaterialTheme.colorScheme.onSurfaceVariant
                else MaterialTheme.colorScheme.onSurface,
        )
    }
}

// ─── Nearby tab placeholder ─────────────────────────────────────────────────

@Composable
private fun NearbyPreviewCard() {
    Box(
        modifier =
            Modifier.fillMaxWidth()
                .padding(16.dp)
                .clip(RoundedCornerShape(16.dp))
                .background(MaterialTheme.colorScheme.surfaceVariant)
                .height(160.dp),
        contentAlignment = Alignment.Center,
    ) {
        Column(horizontalAlignment = Alignment.CenterHorizontally) {
            Icon(
                Icons.Default.Map,
                contentDescription = null,
                modifier = Modifier.size(40.dp),
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(modifier = Modifier.height(8.dp))
            Text(
                text = stringResource(R.string.map_placeholder),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
private fun PlaceholderSection(text: String) {
    Box(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 32.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = text,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

// ─── Bottom action bar ──────────────────────────────────────────────────────

@Composable
private fun BottomActionBar(
    modifier: Modifier = Modifier,
    onMessageClick: () -> Unit,
    onInquiryClick: () -> Unit,
) {
    Surface(
        modifier = modifier.fillMaxWidth(),
        color = MaterialTheme.colorScheme.surface,
        shadowElevation = 8.dp,
    ) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 12.dp),
            horizontalArrangement = Arrangement.spacedBy(10.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Box(
                modifier =
                    Modifier.size(48.dp)
                        .clip(RoundedCornerShape(14.dp))
                        .background(MaterialTheme.colorScheme.surfaceVariant)
                        .clickable(onClick = onMessageClick),
                contentAlignment = Alignment.Center,
            ) {
                Icon(
                    Icons.AutoMirrored.Filled.Message,
                    contentDescription = stringResource(R.string.action_inquire),
                    modifier = Modifier.size(22.dp),
                    tint = MaterialTheme.colorScheme.onSurface,
                )
            }
            Button(
                onClick = onInquiryClick,
                modifier = Modifier.weight(1f).height(48.dp),
                shape = RoundedCornerShape(14.dp),
            ) {
                Text(
                    text = stringResource(R.string.action_request_viewing),
                    style = MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.SemiBold,
                )
            }
        }
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

// ─── Inquiry dialog (preserved) ─────────────────────────────────────────────

@Composable
private fun InquiryDialog(
    listing: ListingDetail,
    isAuthenticated: Boolean,
    isSubmitting: Boolean = false,
    errorMessage: String? = null,
    onDismiss: () -> Unit,
    onSubmit: (message: String, name: String?, email: String?, phone: String?) -> Unit,
) {
    var message by remember {
        mutableStateOf(
            "Hi, I'm interested in ${listing.title}. Please contact me with more information."
        )
    }
    var name by remember { mutableStateOf("") }
    var email by remember { mutableStateOf("") }
    var phone by remember { mutableStateOf("") }

    val canSubmit =
        message.isNotBlank() &&
            !isSubmitting &&
            (isAuthenticated || (name.isNotBlank() && email.isNotBlank()))

    AlertDialog(
        onDismissRequest = { if (!isSubmitting) onDismiss() },
        title = { Text(stringResource(R.string.inquiry_dialog_title)) },
        text = {
            Column {
                if (!isAuthenticated) {
                    Card(
                        colors =
                            CardDefaults.cardColors(
                                containerColor = MaterialTheme.colorScheme.tertiaryContainer
                            )
                    ) {
                        Row(
                            modifier = Modifier.padding(12.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Icon(
                                Icons.Default.Info,
                                contentDescription = null,
                                tint = MaterialTheme.colorScheme.onTertiaryContainer,
                            )
                            Spacer(modifier = Modifier.width(8.dp))
                            Text(
                                text = stringResource(R.string.inquiry_sign_in_hint),
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onTertiaryContainer,
                            )
                        }
                    }
                    Spacer(modifier = Modifier.height(12.dp))
                    OutlinedTextField(
                        value = name,
                        onValueChange = { name = it },
                        label = { Text(stringResource(R.string.label_name_required)) },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true,
                        enabled = !isSubmitting,
                    )
                    Spacer(modifier = Modifier.height(8.dp))
                    OutlinedTextField(
                        value = email,
                        onValueChange = { email = it },
                        label = { Text(stringResource(R.string.label_email_required)) },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true,
                        enabled = !isSubmitting,
                    )
                    Spacer(modifier = Modifier.height(8.dp))
                    OutlinedTextField(
                        value = phone,
                        onValueChange = { phone = it },
                        label = { Text(stringResource(R.string.label_phone_optional)) },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true,
                        enabled = !isSubmitting,
                    )
                    Spacer(modifier = Modifier.height(12.dp))
                }
                OutlinedTextField(
                    value = message,
                    onValueChange = { message = it },
                    label = { Text(stringResource(R.string.label_message)) },
                    modifier = Modifier.fillMaxWidth(),
                    minLines = 4,
                    maxLines = 6,
                    enabled = !isSubmitting,
                )
                errorMessage?.let { error ->
                    Spacer(modifier = Modifier.height(8.dp))
                    Text(
                        text = error,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                    )
                }
            }
        },
        confirmButton = {
            Button(
                onClick = {
                    onSubmit(
                        message,
                        name.takeIf { it.isNotBlank() },
                        email.takeIf { it.isNotBlank() },
                        phone.takeIf { it.isNotBlank() },
                    )
                },
                enabled = canSubmit,
            ) {
                if (isSubmitting) {
                    CircularProgressIndicator(
                        modifier = Modifier.size(16.dp),
                        strokeWidth = 2.dp,
                        color = MaterialTheme.colorScheme.onPrimary,
                    )
                    Spacer(modifier = Modifier.width(8.dp))
                }
                Text(
                    if (isSubmitting) stringResource(R.string.inquiry_sending)
                    else stringResource(R.string.inquiry_send)
                )
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss, enabled = !isSubmitting) {
                Text(stringResource(R.string.cancel))
            }
        },
    )
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ShareListingSheet(listing: ListingDetail, onDismiss: () -> Unit) {
    ModalBottomSheet(onDismissRequest = onDismiss) {
        Column(modifier = Modifier.fillMaxWidth().padding(16.dp)) {
            Text(
                text = stringResource(R.string.share_property),
                style = MaterialTheme.typography.titleLarge,
                fontWeight = FontWeight.Bold,
            )
            Spacer(modifier = Modifier.height(16.dp))
            ListItem(
                headlineContent = { Text(stringResource(R.string.action_share_link)) },
                leadingContent = { Icon(Icons.Default.Link, contentDescription = null) },
                modifier = Modifier.fillMaxWidth(),
            )
            ListItem(
                headlineContent = { Text(stringResource(R.string.share_via)) },
                leadingContent = { Icon(Icons.Default.Share, contentDescription = null) },
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(modifier = Modifier.height(32.dp))
        }
    }
}
