package three.two.bit.ppt.reality.ui.search

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil.compose.AsyncImage
import coil.request.ImageRequest
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import three.two.bit.ppt.reality.R
import three.two.bit.ppt.reality.listing.*
import three.two.bit.ppt.reality.ui.theme.BadgeColors
import three.two.bit.ppt.reality.util.FormatUtils
import three.two.bit.ppt.reality.util.isNetworkError

/**
 * Search / Listings screen — KMP / Compose M3 redesign matching the design bundle
 * (`guest-registration-v2-design-system/project/ui_kits/mobile-native/ screens.jsx` →
 * `KmpListingsScreen`).
 *
 * Layout (top → bottom):
 * 1. Top bar — back arrow, Filtre chip (with active-filter count), View switcher (List / Mapa) as a
 *    pill-style segmented control.
 * 2. Search field row — single-line outlined input with leading search icon, "x" trailing clear
 *    (kept from previous implementation, hidden when bottom-sheet filter is open per design hint).
 * 3. Filter chip strip — horizontally scrollable: Všetky · count / 1+1 / 2+1 / 3+1 / 4+1 / Viac…
 *    The active chip uses primaryContainer fill and onPrimaryContainer ink (M3 chip token).
 * 4. Result list — 16:9 hero card with Featured badge (top-left) + heart pill (top-right); under
 *    image: price + city row, 1-line title, meta line (rooms · m² · floor).
 * 5. FAB — primaryContainer "Save search" (bottom-right, above bottom nav).
 * 6. Optional map view — placeholder (real Mapbox/MapLibre integration tracked in screens-extension
 *    D2).
 *
 * Bottom nav lives in `Navigation.kt`; this composable reserves 96dp of bottom padding for it.
 *
 * The public `ListingCard` composable remains in this file (used by the rest of the app via `import
 * three.two.bit.ppt.reality.ui.search.ListingCard`). It now uses the redesigned card body so
 * callers automatically inherit the new visual.
 *
 * Epic 48 - Story 48.1: Portal Mobile Search.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SearchScreen(
    repository: ListingRepository,
    onListingClick: (String) -> Unit,
    onBackClick: () -> Unit,
    initialType: ListingType? = null,
    initialCategory: PropertyCategory? = null,
) {
    val scope = rememberCoroutineScope()
    val keyboardController = LocalSoftwareKeyboardController.current
    val listState = rememberLazyListState()

    var searchQuery by remember { mutableStateOf("") }
    var isLoading by remember { mutableStateOf(false) }
    var searchResults by remember { mutableStateOf<List<ListingSummary>>(emptyList()) }
    var totalResults by remember { mutableStateOf(0) }
    var currentPage by remember { mutableStateOf(1) }
    var errorMessage by remember { mutableStateOf<String?>(null) }
    var showFilters by remember { mutableStateOf(false) }
    // ViewMode kept as state since list/map switch is wired in many call-sites,
    // but the segmented toggle is hidden from the UI until a real map view ships.
    val viewMode = ViewMode.LIST

    // Filter state. Initial filter values may come from Home (tab/category tap)
    // — those are read from the route args and seed `selectedType` / `selectedCategory`.
    var selectedType by remember { mutableStateOf<ListingType?>(initialType) }
    var selectedCategory by remember { mutableStateOf<PropertyCategory?>(initialCategory) }
    var minPrice by remember { mutableStateOf("") }
    var maxPrice by remember { mutableStateOf("") }
    var minRooms by remember { mutableStateOf<Int?>(null) }
    var selectedSort by remember { mutableStateOf(ListingSortOption.NEWEST) }
    // "Near Me" location filter (epic-82 story 82.3 FilterSheet). radiusKm drives the toggle;
    // nearLat/nearLng are filled once the device coordinate resolves. The shared SearchState
    // only sends a radius once a centre point is known (see buildSearchRequest).
    var nearLat by remember { mutableStateOf<Double?>(null) }
    var nearLng by remember { mutableStateOf<Double?>(null) }
    var radiusKm by remember { mutableStateOf<Double?>(null) }
    val nearMeRequester = rememberNearMeLocationRequester()

    val networkErrorMsg = stringResource(R.string.error_network)
    val searchFailedMsg = stringResource(R.string.error_generic)

    // Stale-response race guard. Each new (page-1) search bumps a monotonic generation and
    // cancels the in-flight job; the apply step is gated on SearchState.shouldApplyResponse so a
    // slower OLDER request that completes after a NEWER one can't clobber the newer results.
    // Page > 1 (infinite-scroll append) reuses the current generation so legitimate appends still
    // land. searchGeneration/searchJob are plain holders (no recomposition needed).
    val searchGeneration = remember { intArrayOf(0) }
    val searchJob = remember { arrayOfNulls<Job>(1) }

    // Shared search logic lives in SearchState (commonMain) so it's unit-tested and
    // reusable by the iOS target; this composable is a thin shell around it.
    fun performSearch(page: Int = 1) {
        // A fresh search (page 1) supersedes anything in flight: cancel it and bump the
        // generation so its late response is discarded by the guard below.
        if (page <= 1) {
            searchJob[0]?.cancel()
            searchGeneration[0] += 1
        }
        val generation = searchGeneration[0].toLong()

        searchJob[0] = scope.launch {
            isLoading = true
            errorMessage = null

            val request =
                SearchState.buildSearchRequest(
                    query = searchQuery,
                    type = selectedType,
                    category = selectedCategory,
                    minPrice = minPrice,
                    maxPrice = maxPrice,
                    minRooms = minRooms,
                    sort = selectedSort,
                    page = page,
                    nearLat = nearLat,
                    nearLng = nearLng,
                    radiusKm = radiusKm,
                )
            val result = repository.searchListings(request)

            // Discard responses that no longer belong to the latest search generation —
            // this is what prevents an out-of-order older response from winning.
            if (
                !SearchState.shouldApplyResponse(
                    responseGeneration = generation,
                    currentGeneration = searchGeneration[0].toLong(),
                )
            ) {
                return@launch
            }

            result.fold(
                onSuccess = { response ->
                    searchResults = SearchState.mergePage(searchResults, response.listings, page)
                    totalResults = response.total
                    currentPage = page
                },
                onFailure = {
                    errorMessage = if (it.isNetworkError()) networkErrorMsg else searchFailedMsg
                },
            )
            isLoading = false
        }
    }

    LaunchedEffect(Unit) { performSearch() }

    // AC-2: debounced search. The pipeline (drop initial emission, 300ms debounce,
    // distinctUntilChanged) lives in SearchState.debouncedQueryFlow so the timing behaviour is
    // unit-tested with virtual time; here we just feed it the query snapshot and search on each
    // settled value.
    LaunchedEffect(Unit) {
        SearchState.debouncedQueryFlow(snapshotFlow { searchQuery }).collect { performSearch() }
    }

    // AC-4: infinite scroll. The trigger pipeline (de-duplicate the last-visible index, snapshot
    // the pagination state, apply the shouldLoadNextPage threshold) lives in
    // SearchState.nextPageTriggerFlow so the trigger behaviour is unit-tested with virtual time;
    // here we feed it the scroll-position snapshot and load each page it emits. The snapshot lambda
    // reads the live pagination state at the moment an index change is processed, so this effect
    // does not need to re-key on searchResults.size / totalResults / isLoading.
    LaunchedEffect(listState) {
        SearchState.nextPageTriggerFlow(
                lastVisibleIndex =
                    snapshotFlow { listState.layoutInfo.visibleItemsInfo.lastOrNull()?.index },
                snapshot = {
                    SearchState.PageSnapshot(
                        loadedCount = searchResults.size,
                        total = totalResults,
                        isLoading = isLoading,
                        currentPage = currentPage,
                    )
                },
            )
            .collect { page -> performSearch(page) }
    }

    val activeFilterCount =
        remember(
            selectedType,
            selectedCategory,
            minRooms,
            minPrice,
            maxPrice,
            nearLat,
            nearLng,
            radiusKm,
        ) {
            SearchState.activeFilterCount(
                type = selectedType,
                category = selectedCategory,
                minRooms = minRooms,
                minPrice = minPrice,
                maxPrice = maxPrice,
                nearLat = nearLat,
                nearLng = nearLng,
                radiusKm = radiusKm,
            )
        }

    // AC-4: FilterSheet — advanced filters presented as a modal bottom sheet (matching the
    // iOS shared `FilterSheet` component) instead of an inline collapsible panel.
    if (showFilters) {
        FilterSheet(
            selectedType = selectedType,
            selectedCategory = selectedCategory,
            minPrice = minPrice,
            maxPrice = maxPrice,
            selectedSort = selectedSort,
            radiusKm = radiusKm,
            hasLocation = nearLat != null && nearLng != null,
            locationRequester = nearMeRequester,
            onDismiss = { showFilters = false },
            onApply = { type, category, minP, maxP, sort, draftRadiusKm, coordinate ->
                selectedType = type
                selectedCategory = category
                minPrice = minP
                maxPrice = maxP
                selectedSort = sort
                radiusKm = draftRadiusKm
                // A resolved coordinate overwrites the previous one; clearing the radius drops it.
                if (draftRadiusKm == null) {
                    nearLat = null
                    nearLng = null
                } else if (coordinate != null) {
                    nearLat = coordinate.latitude
                    nearLng = coordinate.longitude
                }
                showFilters = false
                performSearch()
            },
            onReset = {
                selectedType = null
                selectedCategory = null
                minPrice = ""
                maxPrice = ""
                selectedSort = ListingSortOption.NEWEST
                radiusKm = null
                nearLat = null
                nearLng = null
                showFilters = false
                performSearch()
            },
        )
    }

    Surface(modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
        Box(modifier = Modifier.fillMaxSize()) {
            Column(modifier = Modifier.fillMaxSize()) {
                ListingsTopBar(
                    activeFilterCount = activeFilterCount,
                    onBackClick = onBackClick,
                    onFiltersClick = { showFilters = true },
                )

                // Search input row — single line, inline with chips per design hint.
                // Text changes drive the debounced search above; the IME "search"
                // action just dismisses the keyboard.
                SearchInputRow(
                    value = searchQuery,
                    onChange = { searchQuery = it },
                    onSearch = { keyboardController?.hide() },
                    onClear = { searchQuery = "" },
                )

                // Filter chip strip
                RoomsChipStrip(
                    selected = minRooms,
                    onSelect = {
                        minRooms = it
                        performSearch()
                    },
                    totalResults = totalResults,
                )

                errorMessage?.let { ErrorBanner(it) }

                when {
                    isLoading && searchResults.isEmpty() -> {
                        Box(
                            modifier = Modifier.fillMaxSize(),
                            contentAlignment = Alignment.Center,
                        ) {
                            CircularProgressIndicator()
                        }
                    }
                    // Suppress the "No results found / try adjusting filters" empty
                    // state when the search failed — the error banner above already
                    // explains why; the empty-state copy is misleading in that case.
                    errorMessage != null && searchResults.isEmpty() -> Unit
                    searchResults.isEmpty() -> EmptySearchResults()
                    viewMode == ViewMode.LIST -> {
                        LazyColumn(
                            state = listState,
                            modifier = Modifier.fillMaxSize(),
                            contentPadding =
                                PaddingValues(
                                    start = 16.dp,
                                    end = 16.dp,
                                    top = 4.dp,
                                    bottom = 120.dp,
                                ),
                            verticalArrangement = Arrangement.spacedBy(12.dp),
                        ) {
                            items(searchResults) { listing ->
                                ListingCard(
                                    listing = listing,
                                    onClick = { onListingClick(listing.id) },
                                )
                            }
                            // Infinite-scroll footer: a spinner while the next page loads.
                            // The fetch itself is triggered by the scroll LaunchedEffect above.
                            if (searchResults.size < totalResults) {
                                item {
                                    Box(
                                        modifier =
                                            Modifier.fillMaxWidth().padding(vertical = 16.dp),
                                        contentAlignment = Alignment.Center,
                                    ) {
                                        CircularProgressIndicator()
                                    }
                                }
                            }
                        }
                    }
                    viewMode == ViewMode.MAP -> MapPlaceholder()
                }
            }

            // FAB — Save search
            if (searchResults.isNotEmpty()) {
                ExtendedFloatingActionButton(
                    onClick = { /* save search — opens KmpSaveSearchModal (deferred) */ },
                    modifier =
                        Modifier.align(Alignment.BottomEnd)
                            .padding(
                                end = 16.dp,
                                bottom = if (viewMode == ViewMode.MAP) 100.dp else 96.dp,
                            ),
                    containerColor = MaterialTheme.colorScheme.primaryContainer,
                    contentColor = MaterialTheme.colorScheme.onPrimaryContainer,
                    icon = { Icon(Icons.Default.BookmarkBorder, contentDescription = null) },
                    text = { Text(stringResource(R.string.action_save_search)) },
                )
            }
        }
    }
}

private enum class ViewMode {
    LIST,
    MAP,
}

// ─── Top bar ────────────────────────────────────────────────────────

@Composable
private fun ListingsTopBar(
    activeFilterCount: Int,
    onBackClick: () -> Unit,
    onFiltersClick: () -> Unit,
) {
    Row(
        modifier =
            Modifier.fillMaxWidth()
                .background(MaterialTheme.colorScheme.surface)
                .padding(start = 4.dp, end = 12.dp, top = 8.dp, bottom = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        IconButton(onClick = onBackClick) {
            Icon(
                Icons.AutoMirrored.Filled.ArrowBack,
                contentDescription = stringResource(R.string.cd_back),
                tint = MaterialTheme.colorScheme.onSurface,
            )
        }
        // Filtre chip — primaryContainer when filters active
        Surface(
            onClick = onFiltersClick,
            shape = RoundedCornerShape(8.dp),
            color =
                if (activeFilterCount > 0) MaterialTheme.colorScheme.primaryContainer
                else MaterialTheme.colorScheme.surface,
            border =
                if (activeFilterCount > 0) null
                else BorderStroke(1.dp, MaterialTheme.colorScheme.outline),
        ) {
            Row(
                modifier = Modifier.padding(horizontal = 12.dp, vertical = 7.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Icon(
                    Icons.Default.FilterList,
                    contentDescription = null,
                    tint =
                        if (activeFilterCount > 0) MaterialTheme.colorScheme.onPrimaryContainer
                        else MaterialTheme.colorScheme.onSurface,
                    modifier = Modifier.size(16.dp),
                )
                Spacer(modifier = Modifier.width(6.dp))
                Text(
                    text =
                        if (activeFilterCount > 0)
                            stringResource(R.string.filters_with_count, activeFilterCount)
                        else stringResource(R.string.filters),
                    style = MaterialTheme.typography.labelMedium,
                    fontWeight = FontWeight.SemiBold,
                    color =
                        if (activeFilterCount > 0) MaterialTheme.colorScheme.onPrimaryContainer
                        else MaterialTheme.colorScheme.onSurface,
                )
            }
        }
        Spacer(modifier = Modifier.weight(1f))
        // View switcher hidden until a real map view ships — the toggle was a no-op
        // and read as a broken control. ViewMode state stays so the underlying
        // branch keeps working once map integration lands.
    }
}

@Composable
private fun SegmentChip(icon: ImageVector, label: String, selected: Boolean, onClick: () -> Unit) {
    Surface(
        onClick = onClick,
        shape = RoundedCornerShape(999.dp),
        color = if (selected) MaterialTheme.colorScheme.surface else Color.Transparent,
        shadowElevation = if (selected) 1.dp else 0.dp,
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 14.dp, vertical = 6.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Icon(
                imageVector = icon,
                contentDescription = null,
                modifier = Modifier.size(14.dp),
                tint =
                    if (selected) MaterialTheme.colorScheme.onSurface
                    else MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(modifier = Modifier.width(4.dp))
            Text(
                text = label,
                style = MaterialTheme.typography.labelSmall,
                fontWeight = FontWeight.SemiBold,
                color =
                    if (selected) MaterialTheme.colorScheme.onSurface
                    else MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

// ─── Search input ────────────────────────────────────────────────────

@Composable
private fun SearchInputRow(
    value: String,
    onChange: (String) -> Unit,
    onSearch: () -> Unit,
    onClear: () -> Unit,
) {
    OutlinedTextField(
        value = value,
        onValueChange = onChange,
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
        placeholder = { Text(stringResource(R.string.search_placeholder)) },
        singleLine = true,
        leadingIcon = {
            Icon(Icons.Default.Search, contentDescription = stringResource(R.string.cd_search))
        },
        trailingIcon = {
            if (value.isNotEmpty()) {
                IconButton(onClick = onClear) {
                    Icon(
                        Icons.Default.Clear,
                        contentDescription = stringResource(R.string.cd_clear),
                    )
                }
            }
        },
        keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
        keyboardActions = KeyboardActions(onSearch = { onSearch() }),
        shape = RoundedCornerShape(12.dp),
    )
}

// ─── Rooms chip strip ─────────────────────────────────────────

@Composable
private fun RoomsChipStrip(selected: Int?, onSelect: (Int?) -> Unit, totalResults: Int) {
    val chips =
        listOf<Pair<Int?, String>>(
            null to stringResource(R.string.filter_all),
            1 to "1+1",
            2 to "2+1",
            3 to "3+1",
            4 to "4+1",
        )
    LazyRow(
        contentPadding = PaddingValues(horizontal = 16.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        items(chips.size) { idx ->
            val (rooms, label) = chips[idx]
            val isOn = selected == rooms
            Surface(
                onClick = { onSelect(rooms) },
                shape = RoundedCornerShape(8.dp),
                color = if (isOn) MaterialTheme.colorScheme.primaryContainer else Color.Transparent,
                border = if (isOn) null else BorderStroke(1.dp, MaterialTheme.colorScheme.outline),
            ) {
                Row(
                    modifier = Modifier.padding(horizontal = 14.dp, vertical = 7.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        text = label,
                        style = MaterialTheme.typography.labelMedium,
                        fontWeight = FontWeight.Medium,
                        color =
                            if (isOn) MaterialTheme.colorScheme.onPrimaryContainer
                            else MaterialTheme.colorScheme.onSurface,
                    )
                    if (isOn && totalResults > 0) {
                        Text(
                            text = " · $totalResults",
                            style = MaterialTheme.typography.labelMedium,
                            color = MaterialTheme.colorScheme.onPrimaryContainer.copy(alpha = 0.7f),
                        )
                    }
                }
            }
        }
    }
}

// ─── Filter sheet (modal bottom sheet) ──────────────────────
//
// AC-4: advanced filters are presented as a Material 3 ModalBottomSheet, named
// `FilterSheet` to match the iOS shared component (docs/screens/reality-mobile/search.md
// → sharedComponents: FilterSheet). The sheet holds its own draft copy of every
// filter and only commits to the caller on "Apply", so partial edits don't fire a
// search mid-typing. "Reset" clears the draft and applies the cleared state.

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun FilterSheet(
    selectedType: ListingType?,
    selectedCategory: PropertyCategory?,
    minPrice: String,
    maxPrice: String,
    selectedSort: ListingSortOption,
    radiusKm: Double?,
    hasLocation: Boolean,
    locationRequester: NearMeLocationRequester,
    onDismiss: () -> Unit,
    onApply:
        (
            ListingType?,
            PropertyCategory?,
            String,
            String,
            ListingSortOption,
            Double?,
            NearMeCoordinate?,
        ) -> Unit,
    onReset: () -> Unit,
) {
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)

    // Draft state — seeded from the committed filters, edited locally, committed on Apply.
    var draftType by remember { mutableStateOf(selectedType) }
    var draftCategory by remember { mutableStateOf(selectedCategory) }
    var draftMinPrice by remember { mutableStateOf(minPrice) }
    var draftMaxPrice by remember { mutableStateOf(maxPrice) }
    var draftSort by remember { mutableStateOf(selectedSort) }
    // Near Me draft. draftRadiusKm null = toggle off. draftCoordinate is filled once the device
    // location resolves; isLocating tracks the in-flight permission/fetch so the UI can show a
    // spinner. If the filter was already active (hasLocation), the previously-committed coordinate
    // is reused unless a fresh one is fetched.
    var draftRadiusKm by remember { mutableStateOf(radiusKm) }
    var draftCoordinate by remember { mutableStateOf<NearMeCoordinate?>(null) }
    var isLocating by remember { mutableStateOf(false) }
    // The toggle is "on" while a radius is selected; it stays on optimistically while locating.
    val nearMeEnabled = draftRadiusKm != null
    // Whether we have (or had) a usable centre point for the search.
    val hasCoordinate = draftCoordinate != null || (hasLocation && radiusKm != null)

    ModalBottomSheet(onDismissRequest = onDismiss, sheetState = sheetState) {
        Column(
            modifier = Modifier.fillMaxWidth().padding(start = 16.dp, end = 16.dp, bottom = 24.dp)
        ) {
            Text(
                text = stringResource(R.string.filters),
                style = MaterialTheme.typography.titleLarge,
                fontWeight = FontWeight.Bold,
                modifier = Modifier.padding(bottom = 16.dp),
            )

            // Type
            Text(
                text = stringResource(R.string.filter_type),
                style = MaterialTheme.typography.labelMedium,
                fontWeight = FontWeight.SemiBold,
                modifier = Modifier.padding(bottom = 8.dp),
            )
            LazyRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                item {
                    FilterChip(
                        selected = draftType == null,
                        onClick = { draftType = null },
                        label = { Text(stringResource(R.string.filter_all)) },
                    )
                }
                items(ListingType.entries) { type ->
                    FilterChip(
                        selected = draftType == type,
                        onClick = { draftType = type },
                        label = { Text(type.name.lowercase().replaceFirstChar { it.uppercase() }) },
                    )
                }
            }
            Spacer(modifier = Modifier.height(12.dp))
            // Category
            Text(
                text = stringResource(R.string.filter_category),
                style = MaterialTheme.typography.labelMedium,
                fontWeight = FontWeight.SemiBold,
                modifier = Modifier.padding(bottom = 8.dp),
            )
            LazyRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                item {
                    FilterChip(
                        selected = draftCategory == null,
                        onClick = { draftCategory = null },
                        label = { Text(stringResource(R.string.filter_all)) },
                    )
                }
                items(PropertyCategory.entries) { category ->
                    FilterChip(
                        selected = draftCategory == category,
                        onClick = { draftCategory = category },
                        label = {
                            Text(category.name.lowercase().replaceFirstChar { it.uppercase() })
                        },
                    )
                }
            }
            Spacer(modifier = Modifier.height(12.dp))
            // Price
            Text(
                text = stringResource(R.string.filter_price_range_eur),
                style = MaterialTheme.typography.labelMedium,
                fontWeight = FontWeight.SemiBold,
                modifier = Modifier.padding(bottom = 8.dp),
            )
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                OutlinedTextField(
                    value = draftMinPrice,
                    onValueChange = { draftMinPrice = it },
                    placeholder = { Text(stringResource(R.string.filter_min)) },
                    modifier = Modifier.weight(1f),
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(imeAction = ImeAction.Next),
                )
                Text("—")
                OutlinedTextField(
                    value = draftMaxPrice,
                    onValueChange = { draftMaxPrice = it },
                    placeholder = { Text(stringResource(R.string.filter_max)) },
                    modifier = Modifier.weight(1f),
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
                )
            }
            Spacer(modifier = Modifier.height(12.dp))
            // Sort
            Text(
                text = stringResource(R.string.sort_by),
                style = MaterialTheme.typography.labelMedium,
                fontWeight = FontWeight.SemiBold,
                modifier = Modifier.padding(bottom = 8.dp),
            )
            LazyRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                items(ListingSortOption.entries) { sort ->
                    val label =
                        when (sort) {
                            ListingSortOption.NEWEST -> stringResource(R.string.sort_newest)
                            ListingSortOption.OLDEST -> stringResource(R.string.sort_oldest)
                            ListingSortOption.PRICE_ASC -> stringResource(R.string.sort_price_asc)
                            ListingSortOption.PRICE_DESC -> stringResource(R.string.sort_price_desc)
                            ListingSortOption.AREA_ASC -> stringResource(R.string.sort_area_asc)
                            ListingSortOption.AREA_DESC -> stringResource(R.string.sort_area_desc)
                            ListingSortOption.RELEVANCE -> stringResource(R.string.sort_relevance)
                        }
                    FilterChip(
                        selected = draftSort == sort,
                        onClick = { draftSort = sort },
                        label = { Text(label) },
                    )
                }
            }
            Spacer(modifier = Modifier.height(12.dp))
            // Near Me — location-radius filter (mirrors iOS FilterSheet.nearMeSection).
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(
                        Icons.Default.LocationOn,
                        contentDescription = null,
                        tint = MaterialTheme.colorScheme.primary,
                    )
                    Spacer(modifier = Modifier.width(8.dp))
                    Text(
                        text = stringResource(R.string.filter_near_me),
                        style = MaterialTheme.typography.labelLarge,
                        fontWeight = FontWeight.SemiBold,
                    )
                }
                Switch(
                    checked = nearMeEnabled,
                    onCheckedChange = { enabled ->
                        if (enabled) {
                            draftRadiusKm = SearchState.DEFAULT_NEAR_ME_RADIUS_KM
                            if (!hasCoordinate) {
                                isLocating = true
                                locationRequester.acquire { coord ->
                                    isLocating = false
                                    if (coord != null) {
                                        draftCoordinate = coord
                                    } else {
                                        // Permission denied / unavailable — revert the toggle.
                                        draftRadiusKm = null
                                    }
                                }
                            }
                        } else {
                            draftRadiusKm = null
                            draftCoordinate = null
                        }
                    },
                )
            }
            if (nearMeEnabled) {
                when {
                    isLocating ->
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            CircularProgressIndicator(
                                modifier = Modifier.size(16.dp),
                                strokeWidth = 2.dp,
                            )
                            Spacer(modifier = Modifier.width(8.dp))
                            Text(
                                text = stringResource(R.string.locating),
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    hasCoordinate -> {
                        Spacer(modifier = Modifier.height(8.dp))
                        Text(
                            text = stringResource(R.string.filter_radius),
                            style = MaterialTheme.typography.labelMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.padding(bottom = 8.dp),
                        )
                        LazyRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            items(SearchState.RADIUS_OPTIONS_KM) { km ->
                                FilterChip(
                                    selected = draftRadiusKm == km,
                                    onClick = { draftRadiusKm = km },
                                    label = {
                                        Text(stringResource(R.string.radius_km_format, km.toInt()))
                                    },
                                )
                            }
                        }
                    }
                }
            }
            Spacer(modifier = Modifier.height(20.dp))
            // Action row — Reset (text) + Apply (filled).
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                OutlinedButton(onClick = onReset, modifier = Modifier.weight(1f)) {
                    Text(stringResource(R.string.clear_filters))
                }
                Button(
                    onClick = {
                        onApply(
                            draftType,
                            draftCategory,
                            draftMinPrice,
                            draftMaxPrice,
                            draftSort,
                            draftRadiusKm,
                            draftCoordinate,
                        )
                    },
                    modifier = Modifier.weight(1f),
                ) {
                    Text(stringResource(R.string.apply_filters))
                }
            }
        }
    }
}

// ─── Empty / error / map placeholder ──────────────────────

@Composable
private fun ErrorBanner(error: String) {
    Card(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.errorContainer),
        shape = RoundedCornerShape(12.dp),
    ) {
        Row(modifier = Modifier.padding(16.dp), verticalAlignment = Alignment.CenterVertically) {
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
private fun MapPlaceholder() {
    Box(
        modifier = Modifier.fillMaxSize().background(MaterialTheme.colorScheme.surfaceVariant),
        contentAlignment = Alignment.Center,
    ) {
        Column(horizontalAlignment = Alignment.CenterHorizontally) {
            Icon(
                Icons.Default.Map,
                contentDescription = null,
                modifier = Modifier.size(64.dp),
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(modifier = Modifier.height(12.dp))
            Text(
                text = stringResource(R.string.map_placeholder),
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

// ─── Listing card (public — also used by Favorites + other screens) ─────────

@Composable
fun ListingCard(
    listing: ListingSummary,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    showFavoriteButton: Boolean = true,
    isFavorite: Boolean = false,
    onFavoriteClick: (() -> Unit)? = null,
) {
    Card(
        modifier = modifier.fillMaxWidth().clickable(onClick = onClick),
        shape = RoundedCornerShape(16.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 1.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
    ) {
        Column {
            // Hero — 16:9 photo with badges and heart pill
            Box(modifier = Modifier.fillMaxWidth().aspectRatio(16f / 9f)) {
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

                // Featured / Sale / Rent badges (top-left)
                Row(
                    modifier = Modifier.align(Alignment.TopStart).padding(12.dp),
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    if (listing.isFeatured) {
                        UppercaseBadge(
                            text = stringResource(R.string.label_featured),
                            bg = BadgeColors.featuredBg,
                            ink = BadgeColors.featuredInk,
                        )
                    }
                    val transactionBg =
                        when (listing.type) {
                            ListingType.SALE -> BadgeColors.saleBg
                            ListingType.RENT -> BadgeColors.rentBg
                        }
                    val transactionInk =
                        when (listing.type) {
                            ListingType.SALE -> BadgeColors.saleInk
                            ListingType.RENT -> BadgeColors.rentInk
                        }
                    UppercaseBadge(
                        text =
                            when (listing.type) {
                                ListingType.SALE -> stringResource(R.string.for_sale)
                                ListingType.RENT -> stringResource(R.string.for_rent)
                            },
                        bg = transactionBg,
                        ink = transactionInk,
                    )
                }

                // Heart pill (top-right)
                if (showFavoriteButton) {
                    Box(
                        modifier =
                            Modifier.align(Alignment.TopEnd)
                                .padding(12.dp)
                                .size(36.dp)
                                .clip(CircleShape)
                                .background(Color.White.copy(alpha = 0.92f))
                                .clickable(enabled = onFavoriteClick != null) {
                                    onFavoriteClick?.invoke()
                                },
                        contentAlignment = Alignment.Center,
                    ) {
                        Icon(
                            imageVector =
                                if (isFavorite) Icons.Default.Favorite
                                else Icons.Default.FavoriteBorder,
                            contentDescription =
                                if (isFavorite) stringResource(R.string.remove_from_favorites)
                                else stringResource(R.string.add_to_favorites),
                            tint =
                                if (isFavorite) MaterialTheme.colorScheme.error
                                else MaterialTheme.colorScheme.onSurface,
                            modifier = Modifier.size(18.dp),
                        )
                    }
                }

                // Image count (bottom-left) — kept from previous impl
                if (listing.imageCount > 1) {
                    Surface(
                        modifier = Modifier.align(Alignment.BottomStart).padding(12.dp),
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

            // Body — price + location row, title, meta
            Column(
                modifier = Modifier.padding(start = 14.dp, end = 14.dp, top = 12.dp, bottom = 14.dp)
            ) {
                Row(verticalAlignment = Alignment.Bottom, modifier = Modifier.fillMaxWidth()) {
                    Text(
                        text = FormatUtils.formatPrice(listing.price, listing.currency),
                        style = MaterialTheme.typography.titleLarge,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.onSurface,
                        modifier = Modifier.weight(1f),
                    )
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Icon(
                            Icons.Default.LocationOn,
                            contentDescription = null,
                            modifier = Modifier.size(11.dp),
                            tint = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        Spacer(modifier = Modifier.width(3.dp))
                        Text(
                            text = listing.address.city,
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
                Spacer(modifier = Modifier.height(6.dp))
                Text(
                    text = listing.title,
                    style = MaterialTheme.typography.bodyMedium,
                    fontWeight = FontWeight.Medium,
                    color = MaterialTheme.colorScheme.onSurface,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Spacer(modifier = Modifier.height(6.dp))
                Text(
                    text = SearchState.searchResultMeta(listing.rooms, listing.areaSqm),
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
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
