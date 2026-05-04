package three.two.bit.ppt.reality.ui.home

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import coil.compose.AsyncImage
import coil.request.ImageRequest
import three.two.bit.ppt.reality.R
import three.two.bit.ppt.reality.auth.AuthState
import three.two.bit.ppt.reality.auth.SsoService
import three.two.bit.ppt.reality.listing.*
import three.two.bit.ppt.reality.ui.search.ListingCard
import three.two.bit.ppt.reality.ui.theme.BadgeColors
import three.two.bit.ppt.reality.util.FormatUtils

/**
 * Home screen for Reality Portal mobile app.
 *
 * Epic 48 - Story 48.1: Portal Mobile Search
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HomeScreen(
    repository: ListingRepository,
    ssoService: SsoService,
    onSearchClick: () -> Unit,
    onListingClick: (String) -> Unit,
    onFavoritesClick: () -> Unit,
    onAccountClick: () -> Unit,
    onInquiriesClick: () -> Unit
) {
    val authState by ssoService.authState.collectAsState()

    var featuredListings by remember { mutableStateOf<List<ListingSummary>>(emptyList()) }
    var recentListings by remember { mutableStateOf<List<ListingSummary>>(emptyList()) }
    var isLoading by remember { mutableStateOf(true) }
    var errorMessage by remember { mutableStateOf<String?>(null) }

    // Load data on mount
    LaunchedEffect(Unit) {
        isLoading = true

        // Load featured listings
        repository
            .getFeaturedListings()
            .fold(
                onSuccess = { response -> featuredListings = response.listings },
                onFailure = { error -> errorMessage = error.message }
            )

        // Load recent listings
        repository
            .getRecentListings(limit = 10)
            .fold(
                onSuccess = { response -> recentListings = response.listings },
                onFailure = { error -> errorMessage = error.message }
            )

        isLoading = false
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        text = stringResource(R.string.app_name),
                        style = MaterialTheme.typography.titleLarge,
                        fontWeight = FontWeight.Bold
                    )
                },
                actions = {
                    when (authState) {
                        is AuthState.Authenticated -> {
                            IconButton(onClick = onInquiriesClick) {
                                Icon(
                                    Icons.Default.Email,
                                    contentDescription = stringResource(R.string.cd_inquiries)
                                )
                            }
                            IconButton(onClick = onFavoritesClick) {
                                Icon(
                                    Icons.Default.Favorite,
                                    contentDescription = stringResource(R.string.cd_favorites)
                                )
                            }
                            IconButton(onClick = onAccountClick) {
                                Icon(
                                    Icons.Default.AccountCircle,
                                    contentDescription = stringResource(R.string.cd_account)
                                )
                            }
                        }
                        else -> {
                            TextButton(onClick = onAccountClick) {
                                Text(stringResource(R.string.sign_in))
                            }
                        }
                    }
                }
            )
        }
    ) { paddingValues ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(paddingValues),
            contentPadding = PaddingValues(bottom = 16.dp)
        ) {
            // Search bar
            item { SearchBar(onClick = onSearchClick) }

            // Quick filters
            item { QuickFilters(onFilterClick = { /* Navigate to search with filter */}) }

            // Loading or Error state
            if (isLoading) {
                item {
                    Box(
                        modifier = Modifier.fillMaxWidth().height(200.dp),
                        contentAlignment = Alignment.Center
                    ) {
                        CircularProgressIndicator()
                    }
                }
            }

            errorMessage?.let { error ->
                item {
                    Card(
                        modifier = Modifier.fillMaxWidth().padding(16.dp),
                        colors =
                            CardDefaults.cardColors(
                                containerColor = MaterialTheme.colorScheme.errorContainer
                            )
                    ) {
                        Row(
                            modifier = Modifier.padding(16.dp),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Icon(
                                Icons.Default.Error,
                                contentDescription = null,
                                tint = MaterialTheme.colorScheme.onErrorContainer
                            )
                            Spacer(modifier = Modifier.width(8.dp))
                            Text(text = error, color = MaterialTheme.colorScheme.onErrorContainer)
                        }
                    }
                }
            }

            // Featured listings section
            if (featuredListings.isNotEmpty()) {
                item {
                    SectionHeader(
                        title = stringResource(R.string.featured_properties),
                        onSeeAllClick = onSearchClick
                    )
                }

                item {
                    FeaturedListingsRow(
                        listings = featuredListings,
                        onListingClick = onListingClick
                    )
                }
            }

            // Recent listings section
            if (recentListings.isNotEmpty()) {
                item {
                    SectionHeader(
                        title = stringResource(R.string.recently_added),
                        onSeeAllClick = onSearchClick
                    )
                }

                items(recentListings.take(5)) { listing ->
                    ListingCard(
                        listing = listing,
                        onClick = { onListingClick(listing.id) },
                        modifier = Modifier.padding(horizontal = 16.dp, vertical = 6.dp)
                    )
                }
            }

            // View all button
            item {
                Box(
                    modifier = Modifier.fillMaxWidth().padding(16.dp),
                    contentAlignment = Alignment.Center
                ) {
                    Button(onClick = onSearchClick) {
                        Icon(Icons.Default.Search, contentDescription = null)
                        Spacer(modifier = Modifier.width(8.dp))
                        Text(stringResource(R.string.view_all_properties))
                    }
                }
            }
        }
    }
}

@Composable
private fun SearchBar(onClick: () -> Unit) {
    Card(
        modifier = Modifier.fillMaxWidth().padding(16.dp).clickable(onClick = onClick),
        elevation = CardDefaults.cardElevation(defaultElevation = 4.dp)
    ) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(16.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(
                Icons.Default.Search,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.onSurfaceVariant
            )
            Spacer(modifier = Modifier.width(12.dp))
            Text(
                text = stringResource(R.string.search_properties),
                style = MaterialTheme.typography.bodyLarge,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
    }
}

@Composable
private fun QuickFilters(onFilterClick: (String) -> Unit) {
    LazyRow(
        contentPadding = PaddingValues(horizontal = 16.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        item {
            QuickFilterChip(
                label = stringResource(R.string.filter_for_sale),
                icon = Icons.Default.Sell,
                onClick = { onFilterClick("sale") }
            )
        }
        item {
            QuickFilterChip(
                label = stringResource(R.string.filter_for_rent),
                icon = Icons.Default.Home,
                onClick = { onFilterClick("rent") }
            )
        }
        item {
            QuickFilterChip(
                label = stringResource(R.string.filter_apartments),
                icon = Icons.Default.Apartment,
                onClick = { onFilterClick("apartment") }
            )
        }
        item {
            QuickFilterChip(
                label = stringResource(R.string.filter_houses),
                icon = Icons.Default.House,
                onClick = { onFilterClick("house") }
            )
        }
        item {
            QuickFilterChip(
                label = stringResource(R.string.filter_land),
                icon = Icons.Default.Landscape,
                onClick = { onFilterClick("land") }
            )
        }
    }
}

@Composable
private fun QuickFilterChip(
    label: String,
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    onClick: () -> Unit
) {
    ElevatedFilterChip(
        selected = false,
        onClick = onClick,
        label = { Text(label) },
        leadingIcon = { Icon(icon, contentDescription = null, modifier = Modifier.size(18.dp)) }
    )
}

@Composable
private fun SectionHeader(title: String, onSeeAllClick: () -> Unit) {
    Row(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 12.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Text(
            text = title,
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.Bold
        )
        TextButton(onClick = onSeeAllClick) {
            Text(stringResource(R.string.see_all))
            Icon(
                Icons.Default.ChevronRight,
                contentDescription = null,
                modifier = Modifier.size(18.dp)
            )
        }
    }
}

@Composable
private fun FeaturedListingsRow(listings: List<ListingSummary>, onListingClick: (String) -> Unit) {
    LazyRow(
        contentPadding = PaddingValues(horizontal = 16.dp),
        horizontalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        items(listings) { listing ->
            FeaturedListingCard(listing = listing, onClick = { onListingClick(listing.id) })
        }
    }
}

@Composable
private fun FeaturedListingCard(listing: ListingSummary, onClick: () -> Unit) {
    Card(
        modifier = Modifier.width(280.dp).clickable(onClick = onClick),
        elevation = CardDefaults.cardElevation(defaultElevation = 4.dp)
    ) {
        Column {
            // Image
            Box(modifier = Modifier.fillMaxWidth().height(160.dp)) {
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
                            .clip(RoundedCornerShape(topStart = 12.dp, topEnd = 12.dp))
                )

                // Featured badge
                Surface(
                    modifier = Modifier.align(Alignment.TopStart).padding(8.dp),
                    shape = RoundedCornerShape(4.dp),
                    color = BadgeColors.featuredBg
                ) {
                    Text(
                        text = stringResource(R.string.label_featured),
                        modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
                        style = MaterialTheme.typography.labelSmall,
                        color = BadgeColors.featuredInk
                    )
                }
            }

            // Content
            Column(modifier = Modifier.padding(12.dp)) {
                Text(
                    text = formatPrice(listing.price, listing.currency),
                    style = MaterialTheme.typography.titleLarge,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.primary
                )

                Spacer(modifier = Modifier.height(4.dp))

                Text(
                    text = listing.title,
                    style = MaterialTheme.typography.bodyLarge,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis
                )

                Spacer(modifier = Modifier.height(4.dp))

                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(
                        Icons.Default.LocationOn,
                        contentDescription = null,
                        modifier = Modifier.size(14.dp),
                        tint = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                    Spacer(modifier = Modifier.width(4.dp))
                    Text(
                        text = "${listing.address.city}",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }

                Spacer(modifier = Modifier.height(8.dp))

                // Property details
                Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    listing.areaSqm?.let { area ->
                        Text(
                            text = "${area.toInt()} ${stringResource(R.string.sqm)}",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                    listing.rooms?.let { rooms ->
                        Text(
                            text = stringResource(R.string.rooms_count, rooms),
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                }
            }
        }
    }
}

private fun formatPrice(price: Long, currency: String): String {
    return FormatUtils.formatPrice(price, currency)
}
