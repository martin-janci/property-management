package three.two.bit.ppt.reality.ui.home

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Apartment
import androidx.compose.material.icons.filled.ChevronRight
import androidx.compose.material.icons.filled.DirectionsCar
import androidx.compose.material.icons.filled.Domain
import androidx.compose.material.icons.filled.FavoriteBorder
import androidx.compose.material.icons.filled.Forest
import androidx.compose.material.icons.filled.House
import androidx.compose.material.icons.filled.Landscape
import androidx.compose.material.icons.filled.LocationOn
import androidx.compose.material.icons.filled.NotificationsNone
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil.compose.AsyncImage
import coil.request.ImageRequest
import three.two.bit.ppt.reality.R
import three.two.bit.ppt.reality.auth.AuthState
import three.two.bit.ppt.reality.auth.SsoService
import three.two.bit.ppt.reality.listing.*
import three.two.bit.ppt.reality.ui.theme.BadgeColors
import three.two.bit.ppt.reality.ui.theme.Brand500
import three.two.bit.ppt.reality.ui.theme.Brand800
import three.two.bit.ppt.reality.util.FormatUtils
import three.two.bit.ppt.reality.util.isNetworkError

/**
 * Home screen — KMP / Compose M3 redesign matching the design bundle
 * (`guest-registration-v2-design-system/project/ui_kits/mobile-native/screens.jsx` →
 * `KmpHomeScreen`).
 *
 * Layout (top → bottom):
 * 1. Custom top bar (logo wordmark, notification bell with red dot, avatar circle showing user
 *    initials).
 * 2. Hero card — blue gradient (Brand800 → Brand500) with location, search trigger,
 *    transaction-type pills.
 * 3. Featured listings horizontal carousel (260dp cards).
 * 4. 3-col category grid (6 categories).
 * 5. Recently viewed vertical list (64dp thumbnails).
 *
 * Bottom nav lives in `Navigation.kt`; this composable owns the scrollable body and leaves room for
 * it via `contentPadding`.
 *
 * Screen-map: reality/home — see docs/screens/reality/home.md.
 *
 * Epic 48, Story 48.1 — Portal Mobile Search.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HomeScreen(
    repository: ListingRepository,
    ssoService: SsoService,
    onSearchClick: (type: ListingType?, category: PropertyCategory?) -> Unit,
    onListingClick: (String) -> Unit,
    onFavoritesClick: () -> Unit,
    onAccountClick: () -> Unit,
    onInquiriesClick: () -> Unit,
) {
    val authState by ssoService.authState.collectAsState()

    var featuredListings by remember { mutableStateOf<List<ListingSummary>>(emptyList()) }
    var recentListings by remember { mutableStateOf<List<ListingSummary>>(emptyList()) }
    var isLoading by remember { mutableStateOf(true) }
    var errorMessage by remember { mutableStateOf<String?>(null) }

    val networkErrorMsg = stringResource(R.string.error_network)
    val genericErrorMsg = stringResource(R.string.error_generic)

    LaunchedEffect(Unit) {
        isLoading = true
        val onLoadFailure = { error: Throwable ->
            errorMessage = if (error.isNetworkError()) networkErrorMsg else genericErrorMsg
        }
        repository
            .getFeaturedListings()
            .fold(
                onSuccess = { response -> featuredListings = response.listings },
                onFailure = onLoadFailure,
            )
        repository
            .getRecentListings(limit = 10)
            .fold(
                onSuccess = { response -> recentListings = response.listings },
                onFailure = onLoadFailure,
            )
        isLoading = false
    }

    Surface(modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
        LazyColumn(contentPadding = PaddingValues(bottom = 96.dp)) {
            item {
                HomeTopBar(
                    authState = authState,
                    onBellClick = onInquiriesClick,
                    onFavoritesClick = onFavoritesClick,
                    onAvatarClick = onAccountClick,
                )
            }

            item {
                HeroCard(
                    onSearchClick = { onSearchClick(null, null) },
                    onTabClick = { type -> onSearchClick(type, null) },
                )
            }

            errorMessage?.let { error -> item { ErrorCard(error) } }

            // Featured carousel — only renders when we have data. The
            // hero already gives the user a visible affordance while
            // featured loads, so a separate skeleton isn't needed here.
            if (featuredListings.isNotEmpty()) {
                item {
                    SectionHeader(
                        title = stringResource(R.string.featured_properties),
                        onSeeAllClick = { onSearchClick(null, null) },
                    )
                }
                item { FeaturedRow(featuredListings, onListingClick) }
            } else if (isLoading) {
                item { FeaturedSkeleton() }
            }

            item {
                SectionHeader(
                    title = stringResource(R.string.categories),
                    onSeeAllClick = { onSearchClick(null, null) },
                )
            }
            item { CategoryGrid(onCategoryClick = { category -> onSearchClick(null, category) }) }

            if (recentListings.isNotEmpty()) {
                item {
                    SectionHeader(
                        title = stringResource(R.string.recently_added),
                        onSeeAllClick = { onSearchClick(null, null) },
                    )
                }
                items(recentListings.take(5)) { listing ->
                    RecentRow(listing = listing, onClick = { onListingClick(listing.id) })
                }
            }
        }
    }
}

// ─── Top bar ──────────────────────────────────────────────────────────────────
//
// Custom top bar (not Material's TopAppBar) because the design needs a
// notification bell with a red dot + an avatar circle showing initials,
// and the avatar's primaryContainer fill is hard to get right inside
// TopAppBar's predefined action slot.

@Composable
private fun HomeTopBar(
    authState: AuthState,
    onBellClick: () -> Unit,
    onFavoritesClick: () -> Unit,
    onAvatarClick: () -> Unit,
) {
    Row(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 14.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = stringResource(R.string.app_name),
            style = MaterialTheme.typography.titleLarge,
            fontWeight = FontWeight.Bold,
            color = MaterialTheme.colorScheme.onSurface,
            modifier = Modifier.weight(1f),
        )
        IconButton(onClick = onFavoritesClick) {
            Icon(
                imageVector = Icons.Default.FavoriteBorder,
                contentDescription = stringResource(R.string.tab_favorites),
                tint = MaterialTheme.colorScheme.onSurface,
            )
        }
        IconButton(onClick = onBellClick) {
            Box {
                Icon(
                    imageVector = Icons.Default.NotificationsNone,
                    contentDescription = stringResource(R.string.cd_notifications),
                )
                // Red unread-dot — design uses a 8dp circle with a
                // background-matching ring to look like a notification badge.
                Box(
                    modifier =
                        Modifier.align(Alignment.TopEnd)
                            .offset(x = (-2).dp, y = 2.dp)
                            .size(10.dp)
                            .clip(CircleShape)
                            .background(MaterialTheme.colorScheme.background)
                ) {
                    Box(
                        modifier =
                            Modifier.padding(2.dp)
                                .fillMaxSize()
                                .clip(CircleShape)
                                .background(MaterialTheme.colorScheme.error)
                    )
                }
            }
        }
        Spacer(modifier = Modifier.width(4.dp))
        val initials =
            remember(authState) {
                (authState as? AuthState.Authenticated)?.user?.let { user ->
                    user.name?.let { name ->
                        name
                            .split(" ")
                            .take(2)
                            .mapNotNull { it.firstOrNull()?.uppercase() }
                            .joinToString("")
                    } ?: user.email.take(2).uppercase()
                } ?: "?"
            }
        Box(
            modifier =
                Modifier.size(36.dp)
                    .clip(CircleShape)
                    .background(MaterialTheme.colorScheme.primaryContainer)
                    .clickable(onClick = onAvatarClick),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                text = initials.ifEmpty { "?" },
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onPrimaryContainer,
                fontWeight = FontWeight.SemiBold,
            )
        }
    }
}

// ─── Hero card ────────────────────────────────────────────────────────────────
//
// The hero uses the design system's only gradient: Brand800 → Brand500 at
// 135°. The search field is a white pill on top of the gradient; the
// three transaction-type chips use rgba(255,255,255,0.18) background per
// the design (translucent white on blue) — implemented as Color.White
// with 18% alpha to match.

@Composable
private fun HeroCard(onSearchClick: () -> Unit, onTabClick: (ListingType?) -> Unit) {
    Box(
        modifier =
            Modifier.padding(horizontal = 16.dp)
                .fillMaxWidth()
                .clip(RoundedCornerShape(16.dp))
                .drawBehind {
                    drawRect(
                        brush =
                            Brush.linearGradient(
                                colors = listOf(Brand800, Brand500),
                                start = Offset(0f, 0f),
                                end = Offset(size.width, size.height),
                            )
                    )
                }
                .padding(18.dp)
    ) {
        Column {
            Text(
                text = stringResource(R.string.home_hero_welcome),
                style = MaterialTheme.typography.bodySmall,
                color = Color.White.copy(alpha = 0.85f),
            )
            Spacer(modifier = Modifier.height(2.dp))
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(
                    imageVector = Icons.Default.LocationOn,
                    contentDescription = null,
                    tint = Color.White,
                    modifier = Modifier.size(16.dp),
                )
                Spacer(modifier = Modifier.width(6.dp))
                Text(
                    text = stringResource(R.string.home_hero_location_placeholder),
                    style = MaterialTheme.typography.titleMedium,
                    color = Color.White,
                    fontWeight = FontWeight.SemiBold,
                )
            }
            Spacer(modifier = Modifier.height(14.dp))
            // Search trigger — white pill, accepts tap to navigate to /search.
            Surface(
                onClick = onSearchClick,
                shape = RoundedCornerShape(28.dp),
                color = Color.White.copy(alpha = 0.95f),
            ) {
                Row(
                    modifier =
                        Modifier.fillMaxWidth().padding(horizontal = 14.dp, vertical = 10.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Icon(
                        imageVector = Icons.Default.Search,
                        contentDescription = null,
                        tint = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.size(20.dp),
                    )
                    Spacer(modifier = Modifier.width(10.dp))
                    Text(
                        text = stringResource(R.string.home_hero_search_placeholder),
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
            Spacer(modifier = Modifier.height(12.dp))
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                // "New builds" maps to no type (it would need a separate filter once
                // newly-built inventory is tagged in the backend; until then we just
                // open Search without a type preselect).
                listOf(
                        R.string.home_hero_tab_sale to ListingType.SALE,
                        R.string.home_hero_tab_rent to ListingType.RENT,
                        R.string.home_hero_tab_new to null,
                    )
                    .forEach { (labelRes, type) ->
                        Surface(
                            onClick = { onTabClick(type) },
                            shape = RoundedCornerShape(999.dp),
                            color = Color.White.copy(alpha = 0.18f),
                        ) {
                            Text(
                                text = stringResource(labelRes),
                                style = MaterialTheme.typography.labelMedium,
                                color = Color.White,
                                fontWeight = FontWeight.SemiBold,
                                modifier = Modifier.padding(horizontal = 14.dp, vertical = 6.dp),
                            )
                        }
                    }
            }
        }
    }
}

// ─── Section header ───────────────────────────────────────────────────────────

@Composable
private fun SectionHeader(title: String, onSeeAllClick: () -> Unit) {
    Row(
        modifier =
            Modifier.fillMaxWidth()
                .padding(horizontal = 16.dp, vertical = 12.dp)
                .padding(top = 10.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = title,
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.SemiBold,
            color = MaterialTheme.colorScheme.onSurface,
        )
        TextButton(onClick = onSeeAllClick, contentPadding = PaddingValues(horizontal = 8.dp)) {
            Text(
                text = stringResource(R.string.see_all),
                style = MaterialTheme.typography.labelLarge,
                color = MaterialTheme.colorScheme.primary,
                fontWeight = FontWeight.Medium,
            )
            Icon(
                imageVector = Icons.Default.ChevronRight,
                contentDescription = null,
                modifier = Modifier.size(18.dp),
                tint = MaterialTheme.colorScheme.primary,
            )
        }
    }
}

// ─── Featured row ─────────────────────────────────────────────────────────────

@Composable
private fun FeaturedRow(listings: List<ListingSummary>, onListingClick: (String) -> Unit) {
    LazyRow(
        contentPadding = PaddingValues(horizontal = 16.dp),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        items(listings) { listing ->
            FeaturedCard(listing = listing, onClick = { onListingClick(listing.id) })
        }
    }
}

@Composable
private fun FeaturedCard(listing: ListingSummary, onClick: () -> Unit) {
    Card(
        modifier = Modifier.width(260.dp).clickable(onClick = onClick),
        shape = RoundedCornerShape(16.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
    ) {
        Column {
            Box(modifier = Modifier.fillMaxWidth().height(140.dp)) {
                AsyncImage(
                    model =
                        ImageRequest.Builder(LocalContext.current)
                            .data(listing.primaryImage?.url ?: "")
                            .crossfade(true)
                            .build(),
                    contentDescription = listing.title,
                    contentScale = ContentScale.Crop,
                    modifier =
                        Modifier.fillMaxSize()
                            .clip(RoundedCornerShape(topStart = 16.dp, topEnd = 16.dp)),
                )
                Surface(
                    modifier = Modifier.align(Alignment.TopStart).padding(10.dp),
                    shape = RoundedCornerShape(4.dp),
                    color = BadgeColors.featuredBg,
                ) {
                    Text(
                        text = stringResource(R.string.label_featured).uppercase(),
                        style =
                            MaterialTheme.typography.labelSmall.copy(
                                fontWeight = FontWeight.Bold,
                                letterSpacing = 0.04.sp,
                            ),
                        color = BadgeColors.featuredInk,
                        modifier = Modifier.padding(horizontal = 8.dp, vertical = 3.dp),
                    )
                }
            }
            Column(modifier = Modifier.padding(14.dp)) {
                Text(
                    text = FormatUtils.formatPrice(listing.price, listing.currency),
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.onSurface,
                )
                Spacer(modifier = Modifier.height(2.dp))
                Text(
                    text = listing.title,
                    style = MaterialTheme.typography.bodyMedium,
                    fontWeight = FontWeight.Medium,
                    color = MaterialTheme.colorScheme.onSurface,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    text = listing.address.city + listingMeta(listing),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

@Composable
private fun listingMeta(listing: ListingSummary): String {
    val parts = buildList {
        listing.rooms?.let {
            add(
                androidx.compose.ui.res.pluralStringResource(R.plurals.listing_rooms_plural, it, it)
            )
        }
        listing.areaSqm?.toInt()?.let { add(stringResource(R.string.area_m2, it)) }
    }
    return if (parts.isEmpty()) "" else " · " + parts.joinToString(" · ")
}

@Composable
private fun FeaturedSkeleton() {
    LazyRow(
        contentPadding = PaddingValues(horizontal = 16.dp),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        items(3) {
            Card(
                modifier = Modifier.width(260.dp).height(240.dp),
                shape = RoundedCornerShape(16.dp),
                colors =
                    CardDefaults.cardColors(
                        containerColor = MaterialTheme.colorScheme.surfaceVariant
                    ),
                content = {},
            )
        }
    }
}

// ─── Category grid ────────────────────────────────────────────────────────────
//
// Six categories in a 3×2 grid. Each cell has a 44dp rounded-square icon
// tile in a tinted background, then the category name + listing count.
// Backgrounds use Brand100 / Warning / Success / Violet / Cyan / Danger
// pastel tints — these are the design system's "soft accent" colors and
// pair correctly across themes via the theme's surfaceContainer/light
// tokens. See AppColors.kt.

private data class HomeCategory(
    val labelRes: Int,
    val icon: ImageVector,
    val iconTint: Color,
    val iconBg: Color,
    val category: PropertyCategory?,
)

private val CATEGORIES: List<HomeCategory> =
    listOf(
        HomeCategory(
            R.string.cat_apartments,
            Icons.Default.Apartment,
            Color(0xFF1E40AF),
            Color(0xFFDBEAFE),
            PropertyCategory.APARTMENT,
        ),
        HomeCategory(
            R.string.cat_houses,
            Icons.Default.House,
            Color(0xFF92400E),
            Color(0xFFFEF3C7),
            PropertyCategory.HOUSE,
        ),
        HomeCategory(
            R.string.cat_commercial,
            Icons.Default.Domain,
            Color(0xFF065F46),
            Color(0xFFD1FAE5),
            PropertyCategory.COMMERCIAL,
        ),
        HomeCategory(
            R.string.cat_land,
            Icons.Default.Landscape,
            Color(0xFF5B21B6),
            Color(0xFFEDE9FE),
            PropertyCategory.LAND,
        ),
        // No PropertyCategory match for "recreation" yet — tapping opens
        // Search without preselection, same as today.
        HomeCategory(
            R.string.cat_recreation,
            Icons.Default.Forest,
            Color(0xFF155E75),
            Color(0xFFCFFAFE),
            null,
        ),
        HomeCategory(
            R.string.cat_parking,
            Icons.Default.DirectionsCar,
            Color(0xFF991B1B),
            Color(0xFFFEE2E2),
            PropertyCategory.GARAGE,
        ),
    )

@Composable
private fun CategoryGrid(onCategoryClick: (PropertyCategory?) -> Unit) {
    // LazyVerticalGrid can't be nested inside LazyColumn so we use a
    // fixed-height non-lazy alternative: two Rows of three Cards each.
    Column(
        modifier = Modifier.padding(horizontal = 16.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        CATEGORIES.chunked(3).forEach { row ->
            Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                row.forEach { cat ->
                    CategoryCard(
                        category = cat,
                        modifier = Modifier.weight(1f),
                        onClick = { onCategoryClick(cat.category) },
                    )
                }
            }
        }
    }
}

@Composable
private fun CategoryCard(
    category: HomeCategory,
    modifier: Modifier = Modifier,
    onClick: () -> Unit,
) {
    Card(
        modifier = modifier.clickable(onClick = onClick),
        shape = RoundedCornerShape(16.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
    ) {
        Column(
            modifier = Modifier.padding(vertical = 14.dp, horizontal = 10.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Box(
                modifier =
                    Modifier.size(44.dp)
                        .clip(RoundedCornerShape(12.dp))
                        .background(category.iconBg),
                contentAlignment = Alignment.Center,
            ) {
                Icon(
                    imageVector = category.icon,
                    contentDescription = null,
                    tint = category.iconTint,
                    modifier = Modifier.size(22.dp),
                )
            }
            Spacer(modifier = Modifier.height(8.dp))
            Text(
                text = stringResource(category.labelRes),
                style = MaterialTheme.typography.labelLarge,
                fontWeight = FontWeight.SemiBold,
                color = MaterialTheme.colorScheme.onSurface,
            )
            // Per-category listing counts removed — the previous values were
            // hardcoded placeholders. Re-introduce once a /listings/counts
            // endpoint exposes real numbers.
        }
    }
}

// ─── Recent row ───────────────────────────────────────────────────────────────

@Composable
private fun RecentRow(listing: ListingSummary, onClick: () -> Unit) {
    Card(
        modifier =
            Modifier.padding(horizontal = 16.dp, vertical = 4.dp)
                .fillMaxWidth()
                .clickable(onClick = onClick),
        shape = RoundedCornerShape(16.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
    ) {
        Row(modifier = Modifier.padding(10.dp), verticalAlignment = Alignment.CenterVertically) {
            AsyncImage(
                model =
                    ImageRequest.Builder(LocalContext.current)
                        .data(listing.primaryImage?.url ?: "")
                        .crossfade(true)
                        .build(),
                contentDescription = listing.title,
                contentScale = ContentScale.Crop,
                modifier =
                    Modifier.size(64.dp)
                        .clip(RoundedCornerShape(12.dp))
                        .background(MaterialTheme.colorScheme.surfaceVariant),
            )
            Spacer(modifier = Modifier.width(12.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = FormatUtils.formatPrice(listing.price, listing.currency),
                    style = MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.onSurface,
                )
                Spacer(modifier = Modifier.height(2.dp))
                Text(
                    text = listing.title,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurface,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Spacer(modifier = Modifier.height(2.dp))
                Text(
                    text = listing.address.city + listingMeta(listing),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Icon(
                imageVector = Icons.Default.ChevronRight,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

// ─── Error card ───────────────────────────────────────────────────────────────

@Composable
private fun ErrorCard(message: String) {
    Card(
        modifier = Modifier.fillMaxWidth().padding(16.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.errorContainer),
    ) {
        Text(
            text = message,
            modifier = Modifier.padding(16.dp),
            color = MaterialTheme.colorScheme.onErrorContainer,
        )
    }
}
