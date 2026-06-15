package three.two.bit.ppt.reality.ui.favorites

import android.util.Log
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import coil.compose.AsyncImage
import coil.request.ImageRequest
import kotlinx.coroutines.launch
import three.two.bit.ppt.reality.R
import three.two.bit.ppt.reality.api.ApiConfig
import three.two.bit.ppt.reality.auth.AuthState
import three.two.bit.ppt.reality.auth.GuardDecision
import three.two.bit.ppt.reality.auth.authGuardDecision
import three.two.bit.ppt.reality.auth.SsoService
import three.two.bit.ppt.reality.favorites.*
import three.two.bit.ppt.reality.listing.ListingRepository
import three.two.bit.ppt.reality.listing.ListingSummary
import three.two.bit.ppt.reality.util.FormatUtils
import three.two.bit.ppt.reality.util.isNetworkError

private const val TAG = "FavoritesScreen"

/**
 * Favorites screen — KMP / Compose M3 redesign matching the design bundle
 * (`guest-registration-v2-design-system/project/ui_kits/mobile-native/screens.jsx` →
 * `KmpFavoritesScreen`).
 *
 * Layout (top → bottom):
 * 1. Large-title header ("Obľúbené" + summary + share/export actions).
 * 2. Transaction-type filter chip strip (Všetko / Predaj / Prenájom).
 * 3. Sort segmented control (Najnovšie / Cena ↑ / Cena ↓).
 * 4. Saved-properties tab: 2-column grid of cards with 4:3 photo + heart pill overlay; price +
 *    truncated title + meta.
 * 5. Saved-searches tab: kept from existing implementation (the design only covers the "Saved
 *    properties" tab; saved-search list lives in `KmpSavedSearchesScreen` and is reachable from the
 *    Profile screen).
 * 6. FAB: "Vytvoriť zoznam" (create list).
 *
 * Bottom nav lives in `Navigation.kt`; this composable owns the scrollable body and reserves bottom
 * padding (96dp) for it.
 *
 * Epic 48 - Story 48.3: Portal Mobile Favorites.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun FavoritesScreen(
    repository: ListingRepository,
    ssoService: SsoService,
    onListingClick: (String) -> Unit,
    onBackClick: () -> Unit,
    onSignInClick: () -> Unit = {},
) {
    val scope = rememberCoroutineScope()
    val authState by ssoService.authState.collectAsState()
    val networkErrorMsg = stringResource(R.string.error_network)
    val genericErrorMsg = stringResource(R.string.error_generic)
    fun friendlyError(e: Throwable) = if (e.isNetworkError()) networkErrorMsg else genericErrorMsg

    var selectedTab by remember { mutableIntStateOf(0) }
    var transactionFilter by remember { mutableStateOf(TransactionFilter.ALL) }
    var sortMode by remember { mutableStateOf(SortMode.NEWEST) }
    var favorites by remember { mutableStateOf<List<FavoriteEntry>>(emptyList()) }
    var savedSearches by remember { mutableStateOf<List<SavedSearch>>(emptyList()) }
    var isLoading by remember { mutableStateOf(true) }
    var errorMessage by remember { mutableStateOf<String?>(null) }

    val favoritesRepository =
        remember(authState) {
            val token = (authState as? AuthState.Authenticated)?.sessionToken
            FavoritesRepository(baseUrl = ApiConfig.requireBaseUrl(), sessionToken = token)
        }

    LaunchedEffect(authState) {
        if (authState is AuthState.Authenticated) {
            isLoading = true
            errorMessage = null
            favoritesRepository
                .getFavorites()
                .fold(
                    onSuccess = { response -> favorites = response.favorites },
                    onFailure = { error -> errorMessage = friendlyError(error) },
                )
            favoritesRepository
                .getSavedSearches()
                .fold(
                    onSuccess = { response -> savedSearches = response.searches },
                    onFailure = { error -> Log.e(TAG, "Failed to load saved searches", error) },
                )
            isLoading = false
        } else {
            isLoading = false
        }
    }

    Surface(modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
        when (authGuardDecision(authState)) {
            GuardDecision.NOT_SIGNED_IN -> NotSignedInContent(onSignInClick = onSignInClick)
            GuardDecision.LOADING -> {
                Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    CircularProgressIndicator()
                }
            }
            GuardDecision.CONTENT -> {
                Column(modifier = Modifier.fillMaxSize()) {
                    FavoritesHeader(
                        favoriteCount = favorites.size,
                        savedSearchCount = savedSearches.size,
                    )

                    // Tab row — Properties / Searches
                    TabRow(
                        selectedTabIndex = selectedTab,
                        containerColor = MaterialTheme.colorScheme.background,
                        contentColor = MaterialTheme.colorScheme.primary,
                    ) {
                        Tab(
                            selected = selectedTab == 0,
                            onClick = { selectedTab = 0 },
                            text = {
                                TabLabel(
                                    label = stringResource(R.string.favorites_tab_properties),
                                    count = favorites.size,
                                )
                            },
                        )
                        Tab(
                            selected = selectedTab == 1,
                            onClick = { selectedTab = 1 },
                            text = {
                                TabLabel(
                                    label = stringResource(R.string.favorites_tab_searches),
                                    count = savedSearches.size,
                                )
                            },
                        )
                    }

                    when {
                        isLoading -> LoadingBox()
                        errorMessage != null ->
                            ErrorContent(
                                message = errorMessage!!,
                                onRetry = {
                                    scope.launch {
                                        isLoading = true
                                        errorMessage = null
                                        favoritesRepository
                                            .getFavorites()
                                            .fold(
                                                onSuccess = { favorites = it.favorites },
                                                onFailure = { errorMessage = friendlyError(it) },
                                            )
                                        isLoading = false
                                    }
                                },
                            )
                        selectedTab == 0 -> {
                            val filtered =
                                remember(favorites, transactionFilter, sortMode) {
                                    applyFilterAndSort(favorites, transactionFilter, sortMode)
                                }
                            PropertyTabContent(
                                items = filtered,
                                transactionFilter = transactionFilter,
                                onFilterChange = { transactionFilter = it },
                                sortMode = sortMode,
                                onSortChange = { sortMode = it },
                                onListingClick = onListingClick,
                                onRemoveFavorite = { listingId ->
                                    scope.launch {
                                        favoritesRepository
                                            .removeFavorite(listingId)
                                            .fold(
                                                onSuccess = {
                                                    favorites =
                                                        favorites.filter {
                                                            it.listingId != listingId
                                                        }
                                                },
                                                onFailure = {
                                                    Log.e(TAG, "Failed to remove favorite", it)
                                                    errorMessage = friendlyError(it)
                                                },
                                            )
                                    }
                                },
                            )
                        }
                        selectedTab == 1 -> {
                            SavedSearchesTabContent(
                                searches = savedSearches,
                                onSearchClick = { /* Navigate to search with filters */ },
                                onToggleAlert = { id, enabled ->
                                    scope.launch {
                                        favoritesRepository
                                            .toggleSearchAlert(id, enabled)
                                            .fold(
                                                onSuccess = { updated ->
                                                    savedSearches =
                                                        savedSearches.map {
                                                            if (it.id == id) updated else it
                                                        }
                                                },
                                                onFailure = { errorMessage = friendlyError(it) },
                                            )
                                    }
                                },
                                onDeleteSearch = { id ->
                                    scope.launch {
                                        favoritesRepository
                                            .deleteSavedSearch(id)
                                            .fold(
                                                onSuccess = {
                                                    savedSearches =
                                                        savedSearches.filter { it.id != id }
                                                },
                                                onFailure = { errorMessage = friendlyError(it) },
                                            )
                                    }
                                },
                            )
                        }
                    }
                }
            }
        }
    }
}

private enum class TransactionFilter {
    ALL,
    SALE,
    RENT,
}

private enum class SortMode {
    NEWEST,
    PRICE_ASC,
    PRICE_DESC,
}

private fun applyFilterAndSort(
    favorites: List<FavoriteEntry>,
    filter: TransactionFilter,
    sort: SortMode,
): List<FavoriteEntry> {
    // `transactionType` comes from the flat server field; fall back to the nested
    // listing type for any legacy/mock data that still uses the old shape.
    fun FavoriteEntry.effectiveTransactionType(): String? = transactionType ?: listing?.type?.name

    val filtered =
        when (filter) {
            TransactionFilter.ALL -> favorites
            TransactionFilter.SALE ->
                favorites.filter {
                    it.effectiveTransactionType()?.equals("sale", ignoreCase = true) == true
                }
            TransactionFilter.RENT ->
                favorites.filter {
                    it.effectiveTransactionType()?.equals("rent", ignoreCase = true) == true
                }
        }
    return when (sort) {
        SortMode.NEWEST -> filtered.sortedByDescending { it.createdAt }
        SortMode.PRICE_ASC ->
            filtered.sortedBy { it.currentPrice ?: it.listing?.price ?: Long.MAX_VALUE }
        SortMode.PRICE_DESC ->
            filtered.sortedByDescending { it.currentPrice ?: it.listing?.price ?: 0L }
    }
}

// ─── Header ─────────────────────────────────────────────────────────────────

@Composable
private fun FavoritesHeader(favoriteCount: Int, savedSearchCount: Int) {
    Row(
        modifier =
            Modifier.fillMaxWidth().padding(start = 16.dp, end = 8.dp, top = 14.dp, bottom = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = stringResource(R.string.tab_favorites),
                style = MaterialTheme.typography.headlineSmall,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.onSurface,
            )
            Spacer(modifier = Modifier.height(2.dp))
            Text(
                text = stringResource(R.string.favorites_summary, favoriteCount, savedSearchCount),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        IconButton(onClick = { /* share */ }) {
            Icon(
                imageVector = Icons.Default.Share,
                contentDescription = stringResource(R.string.cd_share),
                tint = MaterialTheme.colorScheme.onSurface,
            )
        }
        IconButton(onClick = { /* export */ }) {
            Icon(
                imageVector = Icons.Default.FileDownload,
                contentDescription = stringResource(R.string.cd_export),
                tint = MaterialTheme.colorScheme.onSurface,
            )
        }
    }
}

@Composable
private fun TabLabel(label: String, count: Int) {
    Row(verticalAlignment = Alignment.CenterVertically) {
        Text(text = label)
        if (count > 0) {
            Spacer(modifier = Modifier.width(6.dp))
            Badge { Text(count.toString()) }
        }
    }
}

// ─── Property tab content ───────────────────────────────────────────────────

@Composable
private fun PropertyTabContent(
    items: List<FavoriteEntry>,
    transactionFilter: TransactionFilter,
    onFilterChange: (TransactionFilter) -> Unit,
    sortMode: SortMode,
    onSortChange: (SortMode) -> Unit,
    onListingClick: (String) -> Unit,
    onRemoveFavorite: (String) -> Unit,
) {
    Column(modifier = Modifier.fillMaxSize()) {
        FilterChipStrip(selected = transactionFilter, onSelect = onFilterChange)
        Spacer(modifier = Modifier.height(8.dp))
        SortSegmentedControl(
            selected = sortMode,
            onSelect = onSortChange,
            modifier = Modifier.padding(horizontal = 16.dp),
        )
        Spacer(modifier = Modifier.height(12.dp))

        if (items.isEmpty()) {
            EmptyFavorites()
        } else {
            Box(modifier = Modifier.weight(1f)) {
                LazyVerticalGrid(
                    columns = GridCells.Fixed(2),
                    contentPadding = PaddingValues(start = 16.dp, end = 16.dp, bottom = 96.dp),
                    horizontalArrangement = Arrangement.spacedBy(10.dp),
                    verticalArrangement = Arrangement.spacedBy(10.dp),
                ) {
                    items(items, key = { it.id }) { fav ->
                        FavoriteEntryCard(
                            entry = fav,
                            onClick = { onListingClick(fav.listingId) },
                            onRemoveFavorite = { onRemoveFavorite(fav.listingId) },
                        )
                    }
                }

                // FAB — "Create list"
                ExtendedFloatingActionButton(
                    onClick = { /* create list */ },
                    modifier =
                        Modifier.align(Alignment.BottomEnd).padding(end = 16.dp, bottom = 16.dp),
                    containerColor = MaterialTheme.colorScheme.primaryContainer,
                    contentColor = MaterialTheme.colorScheme.onPrimaryContainer,
                    icon = { Icon(imageVector = Icons.Default.Add, contentDescription = null) },
                    text = { Text(text = stringResource(R.string.favorites_create_list)) },
                )
            }
        }
    }
}

@Composable
private fun FilterChipStrip(selected: TransactionFilter, onSelect: (TransactionFilter) -> Unit) {
    val chips =
        listOf(
            TransactionFilter.ALL to R.string.favorites_filter_all,
            TransactionFilter.SALE to R.string.filter_sale,
            TransactionFilter.RENT to R.string.filter_rent,
        )
    Row(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 4.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        chips.forEach { (filter, labelRes) ->
            val isOn = filter == selected
            Surface(
                onClick = { onSelect(filter) },
                shape = RoundedCornerShape(8.dp),
                color = if (isOn) MaterialTheme.colorScheme.primaryContainer else Color.Transparent,
                border =
                    if (isOn) null
                    else
                        androidx.compose.foundation.BorderStroke(
                            1.dp,
                            MaterialTheme.colorScheme.outline,
                        ),
            ) {
                Text(
                    text = stringResource(labelRes),
                    modifier = Modifier.padding(horizontal = 14.dp, vertical = 7.dp),
                    style = MaterialTheme.typography.labelMedium,
                    fontWeight = FontWeight.Medium,
                    color =
                        if (isOn) MaterialTheme.colorScheme.onPrimaryContainer
                        else MaterialTheme.colorScheme.onSurface,
                )
            }
        }
    }
}

@Composable
private fun SortSegmentedControl(
    selected: SortMode,
    onSelect: (SortMode) -> Unit,
    modifier: Modifier = Modifier,
) {
    val items =
        listOf(
            SortMode.NEWEST to R.string.sort_newest,
            SortMode.PRICE_ASC to R.string.sort_price_asc_short,
            SortMode.PRICE_DESC to R.string.sort_price_desc_short,
        )
    Surface(
        modifier = modifier,
        shape = RoundedCornerShape(999.dp),
        color = MaterialTheme.colorScheme.surfaceVariant,
    ) {
        Row(modifier = Modifier.padding(3.dp)) {
            items.forEach { (mode, labelRes) ->
                val isOn = mode == selected
                Surface(
                    onClick = { onSelect(mode) },
                    shape = RoundedCornerShape(999.dp),
                    color = if (isOn) MaterialTheme.colorScheme.surface else Color.Transparent,
                    shadowElevation = if (isOn) 1.dp else 0.dp,
                ) {
                    Text(
                        text = stringResource(labelRes),
                        modifier = Modifier.padding(horizontal = 14.dp, vertical = 6.dp),
                        style = MaterialTheme.typography.labelMedium,
                        fontWeight = FontWeight.SemiBold,
                        color =
                            if (isOn) MaterialTheme.colorScheme.onSurface
                            else MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
        }
    }
}

/**
 * Card that renders a [FavoriteEntry] using the flat server fields (`title`, `currentPrice`,
 * `photoUrl`, `city`, etc.) returned by `GET /api/v1/favorites`. Falls back to the nested
 * [FavoriteEntry.listing] for any legacy / mock data that still carries the old shape.
 */
@Composable
private fun FavoriteEntryCard(
    entry: FavoriteEntry,
    onClick: () -> Unit,
    onRemoveFavorite: () -> Unit,
) {
    // Prefer the flat fields from PortalFavoriteWithListing; fall back to nested listing.
    val title = entry.title ?: entry.listing?.title ?: ""
    val price = entry.currentPrice ?: entry.listing?.price ?: 0L
    val currency = entry.currency ?: entry.listing?.currency ?: "EUR"
    val photoUrl = entry.photoUrl ?: entry.listing?.primaryImage?.url ?: ""
    val metaText = buildString {
        entry.city?.let { append(it) } ?: entry.listing?.address?.city?.let { append(it) }
        if (entry.priceChanged) {
            // `price_change_percentage = (current - original) / original * 100`, so a positive
            // percentage means the price went up (↑) and a negative one means it went down (↓).
            // When the percentage is null but `priceChanged` is true, fall back to ↓ to preserve
            // the prior visual (the only case the server still emits today).
            val pct = entry.priceChangePercentage
            append(if (pct != null && pct > 0) " · ↑" else " · ↓")
        }
    }

    Card(
        modifier = Modifier.clickable(onClick = onClick),
        shape = RoundedCornerShape(16.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
    ) {
        Column {
            Box(modifier = Modifier.fillMaxWidth().aspectRatio(4f / 3f)) {
                AsyncImage(
                    model =
                        ImageRequest.Builder(LocalContext.current)
                            .data(photoUrl)
                            .crossfade(true)
                            .build(),
                    contentDescription = title,
                    contentScale = ContentScale.Crop,
                    modifier =
                        Modifier.fillMaxSize().background(MaterialTheme.colorScheme.surfaceVariant),
                )
                // Heart pill (top-right) — white pill, red filled heart
                Box(
                    modifier =
                        Modifier.align(Alignment.TopEnd)
                            .padding(8.dp)
                            .size(30.dp)
                            .clip(CircleShape)
                            .background(Color.White.copy(alpha = 0.92f))
                            .clickable(onClick = onRemoveFavorite),
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(
                        imageVector = Icons.Default.Favorite,
                        contentDescription = stringResource(R.string.cd_remove_favorite),
                        tint = MaterialTheme.colorScheme.error,
                        modifier = Modifier.size(14.dp),
                    )
                }
            }
            Column(
                modifier = Modifier.padding(start = 10.dp, end = 10.dp, top = 8.dp, bottom = 10.dp)
            ) {
                Text(
                    text = FormatUtils.formatPrice(price, currency),
                    style = MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.onSurface,
                )
                Spacer(modifier = Modifier.height(2.dp))
                Text(
                    text = title,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurface,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                if (metaText.isNotEmpty()) {
                    Spacer(modifier = Modifier.height(2.dp))
                    Text(
                        text = metaText,
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
        }
    }
}

// Retained for any callers outside this file that still pass a ListingSummary directly.
@Composable
private fun FavoritePropertyCard(
    listing: ListingSummary,
    onClick: () -> Unit,
    onRemoveFavorite: () -> Unit,
) {
    Card(
        modifier = Modifier.clickable(onClick = onClick),
        shape = RoundedCornerShape(16.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
    ) {
        Column {
            Box(modifier = Modifier.fillMaxWidth().aspectRatio(4f / 3f)) {
                AsyncImage(
                    model =
                        ImageRequest.Builder(LocalContext.current)
                            .data(listing.primaryImage?.url ?: "")
                            .crossfade(true)
                            .build(),
                    contentDescription = listing.title,
                    contentScale = ContentScale.Crop,
                    modifier =
                        Modifier.fillMaxSize().background(MaterialTheme.colorScheme.surfaceVariant),
                )
                Box(
                    modifier =
                        Modifier.align(Alignment.TopEnd)
                            .padding(8.dp)
                            .size(30.dp)
                            .clip(CircleShape)
                            .background(Color.White.copy(alpha = 0.92f))
                            .clickable(onClick = onRemoveFavorite),
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(
                        imageVector = Icons.Default.Favorite,
                        contentDescription = stringResource(R.string.cd_remove_favorite),
                        tint = MaterialTheme.colorScheme.error,
                        modifier = Modifier.size(14.dp),
                    )
                }
            }
            Column(
                modifier = Modifier.padding(start = 10.dp, end = 10.dp, top = 8.dp, bottom = 10.dp)
            ) {
                Text(
                    text = FormatUtils.formatPrice(listing.price, listing.currency),
                    style = MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.onSurface,
                )
                Spacer(modifier = Modifier.height(2.dp))
                Text(
                    text = listing.title,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurface,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Spacer(modifier = Modifier.height(2.dp))
                Text(
                    text = listingMeta(listing),
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

private fun listingMeta(listing: ListingSummary): String {
    val parts = buildList {
        listing.areaSqm?.toInt()?.let { add("$it m²") }
        listing.rooms?.let { add("$it+1") }
    }
    return parts.joinToString(" · ").ifEmpty { listing.address.city }
}

// ─── Saved searches tab (preserved from previous implementation) ──────────────

@Composable
private fun SavedSearchesTabContent(
    searches: List<SavedSearch>,
    onSearchClick: (SavedSearch) -> Unit,
    onToggleAlert: (String, Boolean) -> Unit,
    onDeleteSearch: (String) -> Unit,
) {
    if (searches.isEmpty()) {
        EmptySavedSearches()
        return
    }
    LazyColumn(
        contentPadding = PaddingValues(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        items(searches, key = { it.id }) { search ->
            SavedSearchCard(
                search = search,
                onClick = { onSearchClick(search) },
                onToggleAlert = { onToggleAlert(search.id, !search.alertEnabled) },
                onDelete = { onDeleteSearch(search.id) },
            )
        }
    }
}

@Composable
private fun SavedSearchCard(
    search: SavedSearch,
    onClick: () -> Unit,
    onToggleAlert: () -> Unit,
    onDelete: () -> Unit,
) {
    var showDeleteDialog by remember { mutableStateOf(false) }

    Card(
        modifier = Modifier.fillMaxWidth(),
        onClick = onClick,
        shape = RoundedCornerShape(16.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
    ) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(16.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column(modifier = Modifier.weight(1f)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        text = search.name,
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold,
                    )
                    if (search.newCount > 0) {
                        Spacer(modifier = Modifier.width(8.dp))
                        Badge(containerColor = MaterialTheme.colorScheme.primary) {
                            Text(stringResource(R.string.saved_search_new_count, search.newCount))
                        }
                    }
                }
                Spacer(modifier = Modifier.height(4.dp))
                val filterParts = mutableListOf<String>()
                search.filters?.let { filters ->
                    filters.type?.let { filterParts.add(it) }
                    filters.category?.let { filterParts.add(it) }
                    filters.city?.let { filterParts.add(it) }
                    if (filters.minPrice != null || filters.maxPrice != null) {
                        filterParts.add("€${filters.minPrice ?: 0} – €${filters.maxPrice ?: "∞"}")
                    }
                }
                if (filterParts.isNotEmpty()) {
                    Text(
                        text = filterParts.joinToString(" • "),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
            IconButton(onClick = onToggleAlert) {
                Icon(
                    if (search.alertEnabled) Icons.Default.Notifications
                    else Icons.Default.NotificationsOff,
                    contentDescription =
                        if (search.alertEnabled)
                            stringResource(R.string.saved_search_disable_alerts)
                        else stringResource(R.string.saved_search_enable_alerts),
                    tint =
                        if (search.alertEnabled) MaterialTheme.colorScheme.primary
                        else MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            IconButton(onClick = { showDeleteDialog = true }) {
                Icon(
                    Icons.Default.Delete,
                    contentDescription = stringResource(R.string.delete),
                    tint = MaterialTheme.colorScheme.error,
                )
            }
        }
    }

    if (showDeleteDialog) {
        AlertDialog(
            onDismissRequest = { showDeleteDialog = false },
            title = { Text(stringResource(R.string.saved_search_delete_title)) },
            text = { Text(stringResource(R.string.saved_search_delete_message, search.name)) },
            confirmButton = {
                TextButton(
                    onClick = {
                        onDelete()
                        showDeleteDialog = false
                    },
                    colors =
                        ButtonDefaults.textButtonColors(
                            contentColor = MaterialTheme.colorScheme.error
                        ),
                ) {
                    Text(stringResource(R.string.delete))
                }
            },
            dismissButton = {
                TextButton(onClick = { showDeleteDialog = false }) {
                    Text(stringResource(R.string.cancel))
                }
            },
        )
    }
}

// ─── Shared state views ─────────────────────────────────────────────────────

@Composable
private fun NotSignedInContent(onSignInClick: () -> Unit) {
    Column(
        modifier = Modifier.fillMaxSize().padding(32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Icon(
            Icons.Default.FavoriteBorder,
            contentDescription = null,
            modifier = Modifier.size(64.dp),
            tint = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(modifier = Modifier.height(16.dp))
        Text(
            text = stringResource(R.string.favorites_sign_in_title),
            style = MaterialTheme.typography.titleMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            text = stringResource(R.string.favorites_sign_in_description),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(modifier = Modifier.height(24.dp))
        Button(
            onClick = onSignInClick,
            shape = RoundedCornerShape(14.dp),
            modifier = Modifier.fillMaxWidth(),
        ) {
            Text(stringResource(R.string.sign_in))
        }
    }
}

@Composable
private fun EmptyFavorites() {
    Column(
        modifier = Modifier.fillMaxSize().padding(32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Icon(
            Icons.Default.FavoriteBorder,
            contentDescription = null,
            modifier = Modifier.size(64.dp),
            tint = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(modifier = Modifier.height(16.dp))
        Text(
            text = stringResource(R.string.empty_favorites),
            style = MaterialTheme.typography.titleMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            text = stringResource(R.string.favorites_empty_description),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
private fun EmptySavedSearches() {
    Column(
        modifier = Modifier.fillMaxSize().padding(32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Icon(
            Icons.Default.SearchOff,
            contentDescription = null,
            modifier = Modifier.size(64.dp),
            tint = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(modifier = Modifier.height(16.dp))
        Text(
            text = stringResource(R.string.saved_searches_empty_title),
            style = MaterialTheme.typography.titleMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            text = stringResource(R.string.saved_searches_empty_description),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
private fun LoadingBox() {
    Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
        CircularProgressIndicator()
    }
}

@Composable
private fun ErrorContent(message: String, onRetry: () -> Unit) {
    Column(
        modifier = Modifier.fillMaxSize().padding(32.dp),
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
