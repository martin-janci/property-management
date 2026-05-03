package three.two.bit.ppt.reality.ui.search

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import coil.compose.AsyncImage
import coil.request.ImageRequest
import kotlinx.coroutines.launch
import three.two.bit.ppt.reality.R
import three.two.bit.ppt.reality.listing.*
import three.two.bit.ppt.reality.ui.theme.BadgeColors
import three.two.bit.ppt.reality.util.FormatUtils

/**
 * Search screen for Reality Portal mobile app.
 *
 * Epic 48 - Story 48.1: Portal Mobile Search
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SearchScreen(
    repository: ListingRepository,
    onListingClick: (String) -> Unit,
    onBackClick: () -> Unit,
) {
    val scope = rememberCoroutineScope()
    val keyboardController = LocalSoftwareKeyboardController.current

    var searchQuery by remember { mutableStateOf("") }
    var isLoading by remember { mutableStateOf(false) }
    var searchResults by remember { mutableStateOf<List<ListingSummary>>(emptyList()) }
    var totalResults by remember { mutableStateOf(0) }
    var currentPage by remember { mutableStateOf(1) }
    var errorMessage by remember { mutableStateOf<String?>(null) }
    var showFilters by remember { mutableStateOf(false) }

    // Filter state
    var selectedType by remember { mutableStateOf<ListingType?>(null) }
    var selectedCategory by remember { mutableStateOf<PropertyCategory?>(null) }
    var minPrice by remember { mutableStateOf("") }
    var maxPrice by remember { mutableStateOf("") }
    var minRooms by remember { mutableStateOf<Int?>(null) }
    var selectedSort by remember { mutableStateOf(ListingSortOption.NEWEST) }

    fun performSearch(page: Int = 1) {
        scope.launch {
            isLoading = true
            errorMessage = null

            val filters =
                ListingSearchFilters(
                    type = selectedType,
                    category = selectedCategory,
                    minPrice = minPrice.toLongOrNull(),
                    maxPrice = maxPrice.toLongOrNull(),
                    minRooms = minRooms,
                )

            val request =
                ListingSearchRequest(
                    query = searchQuery.takeIf { it.isNotBlank() },
                    filters = filters,
                    sort = selectedSort,
                    page = page,
                    pageSize = 20,
                )

            repository
                .searchListings(request)
                .fold(
                    onSuccess = { response ->
                        if (page == 1) {
                            searchResults = response.listings
                        } else {
                            searchResults = searchResults + response.listings
                        }
                        totalResults = response.total
                        currentPage = page
                    },
                    onFailure = { error -> errorMessage = error.message ?: "Search failed" },
                )

            isLoading = false
        }
    }

    // Initial search on mount
    LaunchedEffect(Unit) { performSearch() }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    OutlinedTextField(
                        value = searchQuery,
                        onValueChange = { searchQuery = it },
                        placeholder = { Text(stringResource(R.string.search_placeholder)) },
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                        keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
                        keyboardActions =
                            KeyboardActions(
                                onSearch = {
                                    keyboardController?.hide()
                                    performSearch()
                                },
                            ),
                        leadingIcon = {
                            Icon(
                                Icons.Default.Search,
                                contentDescription = stringResource(R.string.cd_search),
                            )
                        },
                        trailingIcon = {
                            if (searchQuery.isNotEmpty()) {
                                IconButton(
                                    onClick = {
                                        searchQuery = ""
                                        performSearch()
                                    },
                                ) {
                                    Icon(
                                        Icons.Default.Clear,
                                        contentDescription = stringResource(R.string.cd_clear),
                                    )
                                }
                            }
                        },
                        colors =
                            OutlinedTextFieldDefaults.colors(
                                focusedBorderColor = Color.Transparent,
                                unfocusedBorderColor = Color.Transparent,
                            ),
                    )
                },
                navigationIcon = {
                    IconButton(onClick = onBackClick) {
                        Icon(
                            Icons.Default.ArrowBack,
                            contentDescription = stringResource(R.string.cd_back),
                        )
                    }
                },
                actions = {
                    IconButton(onClick = { showFilters = !showFilters }) {
                        Icon(
                            Icons.Default.FilterList,
                            contentDescription = stringResource(R.string.cd_filters),
                            tint =
                                if (showFilters) MaterialTheme.colorScheme.primary
                                else MaterialTheme.colorScheme.onSurface
                        )
                    }
                },
            )
        },
    ) { paddingValues ->
        Column(modifier = Modifier.fillMaxSize().padding(paddingValues)) {
            // Filter chips row
            if (showFilters) {
                FilterSection(
                    selectedType = selectedType,
                    onTypeChange = {
                        selectedType = it
                        performSearch()
                    },
                    selectedCategory = selectedCategory,
                    onCategoryChange = {
                        selectedCategory = it
                        performSearch()
                    },
                    minPrice = minPrice,
                    onMinPriceChange = { minPrice = it },
                    maxPrice = maxPrice,
                    onMaxPriceChange = { maxPrice = it },
                    minRooms = minRooms,
                    onMinRoomsChange = {
                        minRooms = it
                        performSearch()
                    },
                    selectedSort = selectedSort,
                    onSortChange = {
                        selectedSort = it
                        performSearch()
                    },
                    onApplyPrice = { performSearch() },
                )
            }

            // Results count
            if (totalResults > 0) {
                Text(
                    text = stringResource(R.string.properties_found, totalResults),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
                )
            }

            // Error message
            errorMessage?.let { error ->
                Card(
                    modifier = Modifier.fillMaxWidth().padding(16.dp),
                    colors =
                        CardDefaults.cardColors(
                            containerColor = MaterialTheme.colorScheme.errorContainer,
                        ),
                ) {
                    Row(
                        modifier = Modifier.padding(16.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Icon(
                            Icons.Default.Error,
                            contentDescription = null,
                            tint = MaterialTheme.colorScheme.onErrorContainer,
                        )
                        Spacer(modifier = Modifier.width(8.dp))
                        Text(text = error, color = MaterialTheme.colorScheme.onErrorContainer)
                    }
                }
            }

            // Results list
            if (isLoading && searchResults.isEmpty()) {
                Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    CircularProgressIndicator()
                }
            } else if (searchResults.isEmpty()) {
                EmptySearchResults()
            } else {
                LazyColumn(
                    modifier = Modifier.fillMaxSize(),
                    contentPadding = PaddingValues(16.dp),
                    verticalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    items(searchResults) { listing ->
                        ListingCard(listing = listing, onClick = { onListingClick(listing.id) })
                    }

                    // Load more button
                    if (searchResults.size < totalResults) {
                        item {
                            Box(
                                modifier = Modifier.fillMaxWidth().padding(vertical = 16.dp),
                                contentAlignment = Alignment.Center,
                            ) {
                                if (isLoading) {
                                    CircularProgressIndicator()
                                } else {
                                    Button(onClick = { performSearch(currentPage + 1) }) {
                                        Text(stringResource(R.string.action_load_more))
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun FilterSection(
    selectedType: ListingType?,
    onTypeChange: (ListingType?) -> Unit,
    selectedCategory: PropertyCategory?,
    onCategoryChange: (PropertyCategory?) -> Unit,
    minPrice: String,
    onMinPriceChange: (String) -> Unit,
    maxPrice: String,
    onMaxPriceChange: (String) -> Unit,
    minRooms: Int?,
    onMinRoomsChange: (Int?) -> Unit,
    selectedSort: ListingSortOption,
    onSortChange: (ListingSortOption) -> Unit,
    onApplyPrice: () -> Unit,
) {
    val filterAllLabel = stringResource(R.string.filter_all)
    val filterAnyLabel = stringResource(R.string.filter_any)
    val sortNewestLabel = stringResource(R.string.sort_newest)
    val sortOldestLabel = stringResource(R.string.sort_oldest)
    val sortPriceAscLabel = stringResource(R.string.sort_price_asc)
    val sortPriceDescLabel = stringResource(R.string.sort_price_desc)
    val sortAreaAscLabel = stringResource(R.string.sort_area_asc)
    val sortAreaDescLabel = stringResource(R.string.sort_area_desc)
    val sortRelevanceLabel = stringResource(R.string.sort_relevance)

    Column(
        modifier =
            Modifier.fillMaxWidth()
                .background(MaterialTheme.colorScheme.surfaceVariant)
                .padding(16.dp),
    ) {
        // Type filter
        Text(
            text = stringResource(R.string.filter_type),
            style = MaterialTheme.typography.labelMedium,
            modifier = Modifier.padding(bottom = 8.dp),
        )
        LazyRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            item {
                FilterChip(
                    selected = selectedType == null,
                    onClick = { onTypeChange(null) },
                    label = { Text(filterAllLabel) },
                )
            }
            items(ListingType.entries) { type ->
                FilterChip(
                    selected = selectedType == type,
                    onClick = { onTypeChange(type) },
                    label = { Text(type.name.lowercase().replaceFirstChar { it.uppercase() }) },
                )
            }
        }

        Spacer(modifier = Modifier.height(12.dp))

        // Category filter
        Text(
            text = stringResource(R.string.filter_category),
            style = MaterialTheme.typography.labelMedium,
            modifier = Modifier.padding(bottom = 8.dp),
        )
        LazyRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            item {
                FilterChip(
                    selected = selectedCategory == null,
                    onClick = { onCategoryChange(null) },
                    label = { Text(filterAllLabel) },
                )
            }
            items(PropertyCategory.entries) { category ->
                FilterChip(
                    selected = selectedCategory == category,
                    onClick = { onCategoryChange(category) },
                    label = { Text(category.name.lowercase().replaceFirstChar { it.uppercase() }) },
                )
            }
        }

        Spacer(modifier = Modifier.height(12.dp))

        // Price range
        Text(
            text = stringResource(R.string.filter_price_range_eur),
            style = MaterialTheme.typography.labelMedium,
            modifier = Modifier.padding(bottom = 8.dp),
        )
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            OutlinedTextField(
                value = minPrice,
                onValueChange = onMinPriceChange,
                placeholder = { Text(stringResource(R.string.filter_min)) },
                modifier = Modifier.weight(1f),
                singleLine = true,
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Next),
            )
            Text("—")
            OutlinedTextField(
                value = maxPrice,
                onValueChange = onMaxPriceChange,
                placeholder = { Text(stringResource(R.string.filter_max)) },
                modifier = Modifier.weight(1f),
                singleLine = true,
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
                keyboardActions = KeyboardActions(onDone = { onApplyPrice() }),
            )
            IconButton(onClick = onApplyPrice) {
                Icon(
                    Icons.Default.Check,
                    contentDescription = stringResource(R.string.filter_apply),
                )
            }
        }

        Spacer(modifier = Modifier.height(12.dp))

        // Rooms filter
        Text(
            text = stringResource(R.string.filter_rooms),
            style = MaterialTheme.typography.labelMedium,
            modifier = Modifier.padding(bottom = 8.dp),
        )
        LazyRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            item {
                FilterChip(
                    selected = minRooms == null,
                    onClick = { onMinRoomsChange(null) },
                    label = { Text(filterAnyLabel) },
                )
            }
            items(listOf(1, 2, 3, 4, 5)) { rooms ->
                FilterChip(
                    selected = minRooms == rooms,
                    onClick = { onMinRoomsChange(rooms) },
                    label = { Text("$rooms+") },
                )
            }
        }

        Spacer(modifier = Modifier.height(12.dp))

        // Sort
        Text(
            text = stringResource(R.string.sort_by),
            style = MaterialTheme.typography.labelMedium,
            modifier = Modifier.padding(bottom = 8.dp),
        )
        LazyRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            items(ListingSortOption.entries) { sort ->
                val label =
                    when (sort) {
                        ListingSortOption.NEWEST -> sortNewestLabel
                        ListingSortOption.OLDEST -> sortOldestLabel
                        ListingSortOption.PRICE_ASC -> sortPriceAscLabel
                        ListingSortOption.PRICE_DESC -> sortPriceDescLabel
                        ListingSortOption.AREA_ASC -> sortAreaAscLabel
                        ListingSortOption.AREA_DESC -> sortAreaDescLabel
                        ListingSortOption.RELEVANCE -> sortRelevanceLabel
                    }
                FilterChip(
                    selected = selectedSort == sort,
                    onClick = { onSortChange(sort) },
                    label = { Text(label) },
                )
            }
        }
    }
}

@Composable
private fun EmptySearchResults() {
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
            text = stringResource(R.string.empty_search_results),
            style = MaterialTheme.typography.titleMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            text = stringResource(R.string.empty_search_tip),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
fun ListingCard(
    listing: ListingSummary,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    showFavoriteButton: Boolean = false,
    isFavorite: Boolean = false,
    onFavoriteClick: (() -> Unit)? = null,
) {
    Card(
        modifier = modifier.fillMaxWidth().clickable(onClick = onClick),
        elevation = CardDefaults.cardElevation(defaultElevation = 2.dp),
    ) {
        Column {
            // Image
            Box(modifier = Modifier.fillMaxWidth().height(180.dp)) {
                AsyncImage(
                    model =
                        ImageRequest.Builder(LocalContext.current)
                            .data(listing.primaryImage?.url ?: "")
                            .crossfade(true)
                            .build(),
                    contentDescription = listing.title,
                    contentScale = ContentScale.Crop,
                    modifier = Modifier.fillMaxSize(),
                )

                // Badges row
                Row(
                    modifier = Modifier.align(Alignment.TopStart).padding(8.dp),
                    horizontalArrangement = Arrangement.spacedBy(4.dp),
                ) {
                    if (listing.isFeatured) {
                        Badge(containerColor = BadgeColors.featuredBg) {
                            Text(
                                stringResource(R.string.label_featured),
                                modifier = Modifier.padding(horizontal = 4.dp),
                                color = BadgeColors.featuredInk,
                            )
                        }
                    }
                    if (listing.isNew) {
                        Badge(containerColor = MaterialTheme.colorScheme.tertiary) {
                            Text(
                                stringResource(R.string.label_new),
                                modifier = Modifier.padding(horizontal = 4.dp),
                            )
                        }
                    }
                    if (listing.isPriceReduced) {
                        Badge(containerColor = MaterialTheme.colorScheme.error) {
                            Text(
                                stringResource(R.string.label_reduced),
                                modifier = Modifier.padding(horizontal = 4.dp),
                            )
                        }
                    }
                }

                // Type badge
                val typeBadgeBg =
                    when (listing.type) {
                        ListingType.SALE -> BadgeColors.saleBg
                        ListingType.RENT -> BadgeColors.rentBg
                    }
                val typeBadgeInk =
                    when (listing.type) {
                        ListingType.SALE -> BadgeColors.saleInk
                        ListingType.RENT -> BadgeColors.rentInk
                    }
                Surface(
                    modifier = Modifier.align(Alignment.TopEnd).padding(8.dp),
                    shape = RoundedCornerShape(4.dp),
                    color = typeBadgeBg,
                ) {
                    Text(
                        text =
                            when (listing.type) {
                                ListingType.SALE -> stringResource(R.string.for_sale)
                                ListingType.RENT -> stringResource(R.string.for_rent)
                            },
                        modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
                        style = MaterialTheme.typography.labelSmall,
                        color = typeBadgeInk,
                    )
                }

                // Favorite button
                if (showFavoriteButton && onFavoriteClick != null) {
                    IconButton(
                        onClick = onFavoriteClick,
                        modifier = Modifier.align(Alignment.BottomEnd),
                    ) {
                        Icon(
                            imageVector =
                                if (isFavorite) Icons.Default.Favorite
                                else Icons.Default.FavoriteBorder,
                            contentDescription =
                                if (isFavorite) stringResource(R.string.remove_from_favorites)
                                else stringResource(R.string.add_to_favorites),
                            tint = if (isFavorite) MaterialTheme.colorScheme.error else Color.White,
                        )
                    }
                }

                // Image count
                if (listing.imageCount > 1) {
                    Surface(
                        modifier = Modifier.align(Alignment.BottomStart).padding(8.dp),
                        shape = RoundedCornerShape(4.dp),
                        color = Color.Black.copy(alpha = 0.6f),
                    ) {
                        Row(
                            modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Icon(
                                Icons.Default.PhotoLibrary,
                                contentDescription = null,
                                modifier = Modifier.size(14.dp),
                                tint = Color.White,
                            )
                            Spacer(modifier = Modifier.width(4.dp))
                            Text(
                                text = "${listing.imageCount}",
                                style = MaterialTheme.typography.labelSmall,
                                color = Color.White,
                            )
                        }
                    }
                }
            }

            // Content
            Column(modifier = Modifier.padding(12.dp)) {
                // Price
                Text(
                    text = formatPrice(listing.price, listing.currency),
                    style = MaterialTheme.typography.titleLarge,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.primary,
                )

                Spacer(modifier = Modifier.height(4.dp))

                // Title
                Text(
                    text = listing.title,
                    style = MaterialTheme.typography.titleMedium,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )

                Spacer(modifier = Modifier.height(4.dp))

                // Location
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(
                        Icons.Default.LocationOn,
                        contentDescription = null,
                        modifier = Modifier.size(16.dp),
                        tint = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Spacer(modifier = Modifier.width(4.dp))
                    Text(
                        text = buildLocationString(listing.address),
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }

                Spacer(modifier = Modifier.height(8.dp))

                // Property details
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(16.dp),
                ) {
                    listing.areaSqm?.let { area ->
                        PropertyDetail(
                            icon = Icons.Default.SquareFoot,
                            value = "${area.toInt()} ${stringResource(R.string.sqm)}",
                        )
                    }
                    listing.rooms?.let { rooms ->
                        PropertyDetail(
                            icon = Icons.Default.MeetingRoom,
                            value = stringResource(R.string.rooms_count, rooms),
                        )
                    }
                    listing.bedrooms?.let { bedrooms ->
                        PropertyDetail(
                            icon = Icons.Default.Bed,
                            value = stringResource(R.string.bed_count, bedrooms),
                        )
                    }
                    listing.bathrooms?.let { bathrooms ->
                        PropertyDetail(
                            icon = Icons.Default.Bathtub,
                            value = stringResource(R.string.bath_count, bathrooms),
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun PropertyDetail(icon: androidx.compose.ui.graphics.vector.ImageVector, value: String) {
    Row(verticalAlignment = Alignment.CenterVertically) {
        Icon(
            icon,
            contentDescription = null,
            modifier = Modifier.size(16.dp),
            tint = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(modifier = Modifier.width(4.dp))
        Text(
            text = value,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

private fun formatPrice(price: Long, currency: String): String {
    return FormatUtils.formatPrice(price, currency)
}

private fun buildLocationString(address: Address): String {
    return FormatUtils.buildSimpleLocationString(address)
}
