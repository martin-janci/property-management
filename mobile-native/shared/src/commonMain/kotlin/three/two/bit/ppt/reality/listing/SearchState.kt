package three.two.bit.ppt.reality.listing

import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.debounce
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.flow.filterNotNull
import kotlinx.coroutines.flow.map

/**
 * Pure search-screen state helpers shared across platforms.
 *
 * These functions hold the non-UI logic the Reality home/search screen needs so it can be unit
 * tested without a Compose (or SwiftUI) runtime and reused by the iOS target later:
 * - [debouncedQueryFlow] — the AC-2 search-as-you-type debounce pipeline applied to the raw
 *   free-text query stream (drop the initial emission, debounce, de-duplicate settled text).
 * - [buildSearchRequest] — assembles a [ListingSearchRequest] from the screen's filter inputs.
 * - [activeFilterCount] — the badge count shown on the "Filtre" chip.
 * - [shouldLoadNextPage] — the infinite-scroll trigger predicate (AC-4): given the currently loaded
 *   count, the reported total, and whether a load is in flight, decide whether to fetch the next
 *   page.
 * - [nextPageTriggerFlow] — the AC-4 infinite-scroll pipeline applied to the stream of
 *   last-visible-index values: de-duplicate the index, snapshot the pagination state, and emit the
 *   page number to load when [shouldLoadNextPage] says so.
 * - [mergePage] — append-or-replace semantics for paginated results (page 1 replaces, later pages
 *   append).
 * - [shouldApplyResponse] — stale-response guard: given the request generation a response belongs
 *   to and the generation currently in effect, decide whether the response may be applied. Protects
 *   the search-as-you-type path from out-of-order completions where a slower OLDER request would
 *   otherwise clobber the results of a NEWER one.
 * - [searchResultMeta] — the "rooms · m²" meta line shown on each result card.
 *
 * Epic 82 - Story 82.3: Reality mobile Home & Search (AC-2 debounced search, AC-4 infinite scroll +
 * FilterSheet).
 */
object SearchState {

    /** Page size used by the mobile search screen. */
    const val PAGE_SIZE: Int = 20

    /** Debounce window (ms) applied to free-text query changes — matches the iOS SearchView. */
    const val SEARCH_DEBOUNCE_MS: Long = 300L

    /**
     * How many items before the end of the loaded list the infinite-scroll prefetch should kick in.
     */
    const val PREFETCH_THRESHOLD: Int = 5

    /**
     * Default radius (km) applied when the user enables the FilterSheet "Near Me" toggle without
     * having picked an explicit radius yet. Mirrors the iOS `nearMeSection` default of 10 km.
     */
    const val DEFAULT_NEAR_ME_RADIUS_KM: Double = 10.0

    /**
     * Radius options (km) offered by the FilterSheet "Near Me" picker. Mirrors the iOS
     * `radiusOptions` list so both platforms present the same choices.
     */
    val RADIUS_OPTIONS_KM: List<Double> = listOf(1.0, 3.0, 5.0, 10.0, 20.0, 50.0)

    /**
     * Debounced search-as-you-type pipeline (AC-2).
     *
     * Given the raw stream of free-text query values emitted as the user types ([source]), returns
     * the stream of queries that should actually trigger a network search:
     * - `drop(1)` skips the initial/current value so the screen's own first load owns it (the
     *   `LaunchedEffect(Unit) { performSearch() }` on mount) and a search isn't double-fired.
     * - `debounce([SEARCH_DEBOUNCE_MS])` collapses rapid keystrokes into a single emission once the
     *   user pauses for the debounce window — mirroring the iOS `SearchView` 300 ms `Task.sleep`.
     * - `distinctUntilChanged()` avoids re-firing when the text settles back on the same value.
     *
     * Extracted from `SearchScreen.kt` so the AC-2 timing behaviour is unit-testable with
     * `kotlinx-coroutines-test` virtual time instead of only living in a Compose `LaunchedEffect`.
     */
    fun debouncedQueryFlow(source: Flow<String>): Flow<String> =
        source.drop(1).debounce(SEARCH_DEBOUNCE_MS).distinctUntilChanged()

    /** Build the search request payload from the raw screen inputs. */
    fun buildSearchRequest(
        query: String,
        type: ListingType? = null,
        category: PropertyCategory? = null,
        minPrice: String = "",
        maxPrice: String = "",
        minRooms: Int? = null,
        sort: ListingSortOption = ListingSortOption.NEWEST,
        page: Int = 1,
        nearLat: Double? = null,
        nearLng: Double? = null,
        radiusKm: Double? = null,
    ): ListingSearchRequest =
        ListingSearchRequest(
            query = query.takeIf { it.isNotBlank() },
            filters =
                ListingSearchFilters(
                    type = type,
                    category = category,
                    minPrice = minPrice.toLongOrNull(),
                    maxPrice = maxPrice.toLongOrNull(),
                    minRooms = minRooms,
                    // A radius is only meaningful with a centre point: drop the radius unless we
                    // actually have device coordinates, so the server never receives a dangling
                    // radius_km with no near_lat/near_lng (which it would otherwise ignore or 400).
                    nearLat = nearLat.takeIf { radiusKm != null },
                    nearLng = nearLng.takeIf { radiusKm != null },
                    radiusKm = radiusKm.takeIf { nearLat != null && nearLng != null },
                ),
            sort = sort,
            page = page,
            pageSize = PAGE_SIZE,
        )

    /**
     * Whether a "Near Me" location filter is active — i.e. a radius AND a centre coordinate are
     * both present. A radius with no coordinate (location not yet resolved) is not yet active.
     */
    fun isNearMeActive(nearLat: Double?, nearLng: Double?, radiusKm: Double?): Boolean =
        radiusKm != null && nearLat != null && nearLng != null

    /** Count of distinct active filters, for the filter-chip badge. */
    fun activeFilterCount(
        type: ListingType?,
        category: PropertyCategory?,
        minRooms: Int?,
        minPrice: String,
        maxPrice: String,
        nearLat: Double? = null,
        nearLng: Double? = null,
        radiusKm: Double? = null,
    ): Int =
        listOf(type, category, minRooms).count { it != null } +
            (if (minPrice.isNotBlank() || maxPrice.isNotBlank()) 1 else 0) +
            (if (isNearMeActive(nearLat, nearLng, radiusKm)) 1 else 0)

    /**
     * Infinite-scroll trigger (AC-4).
     *
     * Returns true when the list of currently-loaded items is within [PREFETCH_THRESHOLD] of the
     * last visible index, more results exist on the server, and no load is already in flight.
     *
     * @param lastVisibleIndex index of the last item currently visible (null when the list is
     *   empty)
     * @param loadedCount number of items already loaded
     * @param total total result count reported by the server
     * @param isLoading whether a page request is in flight
     */
    fun shouldLoadNextPage(
        lastVisibleIndex: Int?,
        loadedCount: Int,
        total: Int,
        isLoading: Boolean,
    ): Boolean {
        if (isLoading) return false
        if (loadedCount == 0) return false
        if (loadedCount >= total) return false
        val last = lastVisibleIndex ?: return false
        return last >= loadedCount - PREFETCH_THRESHOLD
    }

    /** Snapshot of the pagination state read when the scroll position changes. */
    data class PageSnapshot(
        val loadedCount: Int,
        val total: Int,
        val isLoading: Boolean,
        /** Page currently loaded; the next page to fetch is [currentPage] + 1. */
        val currentPage: Int,
    )

    /**
     * Infinite-scroll pipeline (AC-4).
     *
     * Given the stream of last-visible-index values emitted as the user scrolls
     * ([lastVisibleIndex], null when the list is empty), returns the stream of page numbers that
     * should actually be fetched:
     * - `distinctUntilChanged()` collapses repeated index reports for the same row so we only react
     *   when the bottom-most visible item actually moves.
     * - For each new index we take a fresh [PageSnapshot] (loaded count, total, in-flight flag,
     *   current page) via [snapshot] and run the [shouldLoadNextPage] predicate; when it fires we
     *   map to `currentPage + 1`, otherwise to null.
     * - `filterNotNull()` drops the indices that don't warrant a fetch.
     *
     * Extracted from `SearchScreen.kt` so the AC-4 trigger timing/threshold behaviour is
     * unit-testable with `kotlinx-coroutines-test` virtual time instead of only living in a Compose
     * `LaunchedEffect` over `snapshotFlow`.
     *
     * @param lastVisibleIndex stream of the index of the last currently-visible item
     * @param snapshot reads the live pagination state at the moment an index change is processed
     * @return stream of page numbers to load (one emission per page boundary crossed)
     */
    fun nextPageTriggerFlow(lastVisibleIndex: Flow<Int?>, snapshot: () -> PageSnapshot): Flow<Int> =
        lastVisibleIndex
            .distinctUntilChanged()
            .map { last ->
                val s = snapshot()
                if (
                    shouldLoadNextPage(
                        lastVisibleIndex = last,
                        loadedCount = s.loadedCount,
                        total = s.total,
                        isLoading = s.isLoading,
                    )
                ) {
                    s.currentPage + 1
                } else {
                    null
                }
            }
            .filterNotNull()

    /** Merge a freshly-loaded page into the existing results (page 1 replaces, others append). */
    fun mergePage(
        existing: List<ListingSummary>,
        incoming: List<ListingSummary>,
        page: Int,
    ): List<ListingSummary> = if (page <= 1) incoming else existing + incoming

    /**
     * Stale-response guard for search-as-you-type.
     *
     * Each search dispatched from the screen is tagged with a monotonically increasing generation
     * token (captured at launch time). When the network request completes, the caller passes the
     * generation the response belongs to ([responseGeneration]) together with the generation that
     * is currently in effect ([currentGeneration]) — i.e. the token of the most-recently-launched
     * search. The response may only be applied when it belongs to the latest generation.
     *
     * This makes the apply step order-independent: if a slower OLDER request finishes after a NEWER
     * one was launched, its [responseGeneration] is behind [currentGeneration] and the response is
     * discarded instead of clobbering the newer (correct) results. A response from a generation
     * ahead of the current one is impossible by construction and is treated as stale too.
     *
     * @param responseGeneration the generation the just-completed response was dispatched under
     * @param currentGeneration the generation of the most-recently-launched search
     * @return true when the response is the latest in flight and may be applied
     */
    fun shouldApplyResponse(responseGeneration: Long, currentGeneration: Long): Boolean =
        responseGeneration == currentGeneration

    /**
     * Meta line for a search-result [ListingCard][three.two.bit.ppt.reality.ui.search.ListingCard]:
     * room count and floor area joined with " · " (e.g. `3 izby · 84 m²`).
     *
     * Pure formatting extracted from the Compose card so it can be unit-tested and reused by the
     * iOS target. Behaviour is identical to the inline helper it replaced: rooms come first (when
     * present), then the integer-truncated area in m², and an absent dimension is simply omitted.
     * With neither dimension present the result is the empty string.
     *
     * @param rooms room count, or null when unknown
     * @param areaSqm floor area in m² (truncated to a whole number for display), or null
     */
    fun searchResultMeta(rooms: Int?, areaSqm: Double?): String {
        val parts = buildList {
            rooms?.let { add("$it izby") }
            areaSqm?.toInt()?.let { add("$it m²") }
        }
        return parts.joinToString(" · ")
    }
}
