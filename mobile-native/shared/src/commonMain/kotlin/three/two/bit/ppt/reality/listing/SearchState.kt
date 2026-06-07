package three.two.bit.ppt.reality.listing

/**
 * Pure search-screen state helpers shared across platforms.
 *
 * These functions hold the non-UI logic the Reality home/search screen needs so it can be unit
 * tested without a Compose (or SwiftUI) runtime and reused by the iOS target later:
 * - [buildSearchRequest] — assembles a [ListingSearchRequest] from the screen's filter inputs.
 * - [activeFilterCount] — the badge count shown on the "Filtre" chip.
 * - [shouldLoadNextPage] — the infinite-scroll trigger (AC-4): given the currently loaded count,
 *   the reported total, and whether a load is in flight, decide whether to fetch the next page.
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
                ),
            sort = sort,
            page = page,
            pageSize = PAGE_SIZE,
        )

    /** Count of distinct active filters, for the filter-chip badge. */
    fun activeFilterCount(
        type: ListingType?,
        category: PropertyCategory?,
        minRooms: Int?,
        minPrice: String,
        maxPrice: String,
    ): Int =
        listOf(type, category, minRooms).count { it != null } +
            (if (minPrice.isNotBlank() || maxPrice.isNotBlank()) 1 else 0)

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
