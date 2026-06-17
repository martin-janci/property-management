package three.two.bit.ppt.reality.listing

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.flow
import kotlinx.coroutines.flow.flowOf
import kotlinx.coroutines.flow.toList
import kotlinx.coroutines.test.runTest

/**
 * Unit tests for [SearchState] — the pure logic behind the Reality mobile home/search screen.
 *
 * Epic 82 - Story 82.3 (AC-2 debounced search, AC-4 infinite scroll + FilterSheet). These pin the
 * infinite-scroll trigger and request/merge semantics so the Compose screen stays a thin shell.
 */
class SearchStateTest {

    // ── debouncedQueryFlow (AC-2 search-as-you-type debounce) ────────────
    //
    // runTest drives kotlinx-coroutines debounce on virtual time, so these assert the *timing*
    // contract (collapse rapid keystrokes, emit once the user pauses, drop the initial value,
    // de-duplicate settled text) deterministically and instantly.

    @Test
    fun debouncedQueryFlow_collapsesRapidKeystrokesIntoFinalValue() = runTest {
        // Type "b","by","byt" in quick succession (each well within the debounce window), then
        // pause. Only the final settled value should survive the debounce.
        val typed = flow {
            emit("") // initial/current value — dropped by drop(1)
            emit("b")
            delay(50)
            emit("by")
            delay(50)
            emit("byt")
            delay(SearchState.SEARCH_DEBOUNCE_MS + 50) // pause longer than the debounce window
        }

        val emitted = SearchState.debouncedQueryFlow(typed).toList()

        assertEquals(listOf("byt"), emitted)
    }

    @Test
    fun debouncedQueryFlow_emitsEachValueThatSettlesPastTheWindow() = runTest {
        // Two distinct values, each followed by a pause exceeding the debounce window → both fire.
        val typed = flow {
            emit("")
            emit("praha")
            delay(SearchState.SEARCH_DEBOUNCE_MS + 50)
            emit("brno")
            delay(SearchState.SEARCH_DEBOUNCE_MS + 50)
        }

        assertEquals(listOf("praha", "brno"), SearchState.debouncedQueryFlow(typed).toList())
    }

    @Test
    fun debouncedQueryFlow_dropsInitialEmissionSoFirstLoadIsNotDoubleFired() = runTest {
        // Only the initial value is emitted (no user typing). drop(1) means nothing should fire —
        // the screen's own mount-time first load owns that case.
        val typed = flow {
            emit("seed")
            delay(SearchState.SEARCH_DEBOUNCE_MS + 50)
        }

        assertEquals(emptyList(), SearchState.debouncedQueryFlow(typed).toList())
    }

    @Test
    fun debouncedQueryFlow_deduplicatesWhenTextSettlesBackToSameValue() = runTest {
        // User types "byt", pauses (fires), edits to "byty" then deletes back to "byt", pauses.
        // distinctUntilChanged means the second settle on the same "byt" must NOT re-fire.
        val typed = flow {
            emit("")
            emit("byt")
            delay(SearchState.SEARCH_DEBOUNCE_MS + 50)
            emit("byty")
            emit("byt")
            delay(SearchState.SEARCH_DEBOUNCE_MS + 50)
        }

        assertEquals(listOf("byt"), SearchState.debouncedQueryFlow(typed).toList())
    }

    // ── buildSearchRequest ───────────────────────────────────────────────

    @Test
    fun buildSearchRequest_blankQuery_becomesNull() {
        val req = SearchState.buildSearchRequest(query = "   ")
        assertNull(req.query)
    }

    @Test
    fun buildSearchRequest_mapsFiltersAndPaging() {
        val req =
            SearchState.buildSearchRequest(
                query = "byt",
                type = ListingType.RENT,
                category = PropertyCategory.APARTMENT,
                minPrice = "500",
                maxPrice = "1200",
                minRooms = 2,
                sort = ListingSortOption.PRICE_ASC,
                page = 3,
            )
        assertEquals("byt", req.query)
        assertEquals(ListingType.RENT, req.filters?.type)
        assertEquals(PropertyCategory.APARTMENT, req.filters?.category)
        assertEquals(500L, req.filters?.minPrice)
        assertEquals(1200L, req.filters?.maxPrice)
        assertEquals(2, req.filters?.minRooms)
        assertEquals(ListingSortOption.PRICE_ASC, req.sort)
        assertEquals(3, req.page)
        assertEquals(SearchState.PAGE_SIZE, req.pageSize)
    }

    @Test
    fun buildSearchRequest_nonNumericPrice_isIgnored() {
        val req = SearchState.buildSearchRequest(query = "x", minPrice = "abc", maxPrice = "")
        assertNull(req.filters?.minPrice)
        assertNull(req.filters?.maxPrice)
    }

    // ── buildSearchRequest: Near Me / radius (FilterSheet location) ──────

    @Test
    fun buildSearchRequest_nearMe_mapsCoordinateAndRadius() {
        val req =
            SearchState.buildSearchRequest(
                query = "byt",
                nearLat = 48.1486,
                nearLng = 17.1077,
                radiusKm = 5.0,
            )
        assertEquals(48.1486, req.filters?.nearLat)
        assertEquals(17.1077, req.filters?.nearLng)
        assertEquals(5.0, req.filters?.radiusKm)
    }

    @Test
    fun buildSearchRequest_radiusWithoutCoordinate_isDropped() {
        // Radius enabled but location not yet resolved → no centre point, so the radius must not
        // be sent on its own (server can't apply a radius without near_lat/near_lng).
        val req = SearchState.buildSearchRequest(query = "x", radiusKm = 10.0)
        assertNull(req.filters?.radiusKm)
        assertNull(req.filters?.nearLat)
        assertNull(req.filters?.nearLng)
    }

    @Test
    fun buildSearchRequest_coordinateWithoutRadius_isDropped() {
        // Coordinate present but Near Me toggle off (no radius) → don't send a location centre.
        val req = SearchState.buildSearchRequest(query = "x", nearLat = 48.0, nearLng = 17.0)
        assertNull(req.filters?.nearLat)
        assertNull(req.filters?.nearLng)
        assertNull(req.filters?.radiusKm)
    }

    // ── isNearMeActive ───────────────────────────────────────────────────

    @Test
    fun isNearMeActive_trueOnlyWithCoordinateAndRadius() {
        assertTrue(SearchState.isNearMeActive(48.0, 17.0, 10.0))
    }

    @Test
    fun isNearMeActive_falseWithoutCoordinate() {
        assertFalse(SearchState.isNearMeActive(null, null, 10.0))
    }

    @Test
    fun isNearMeActive_falseWithoutRadius() {
        assertFalse(SearchState.isNearMeActive(48.0, 17.0, null))
    }

    @Test
    fun radiusOptions_matchIosNearMePicker() {
        assertEquals(listOf(1.0, 3.0, 5.0, 10.0, 20.0, 50.0), SearchState.RADIUS_OPTIONS_KM)
        assertTrue(SearchState.DEFAULT_NEAR_ME_RADIUS_KM in SearchState.RADIUS_OPTIONS_KM)
    }

    // ── activeFilterCount ────────────────────────────────────────────────

    @Test
    fun activeFilterCount_countsEachDimensionOnce() {
        val count =
            SearchState.activeFilterCount(
                type = ListingType.SALE,
                category = null,
                minRooms = 3,
                minPrice = "100",
                maxPrice = "",
            )
        // type (1) + minRooms (1) + price-range present (1) = 3
        assertEquals(3, count)
    }

    @Test
    fun activeFilterCount_noFilters_isZero() {
        assertEquals(0, SearchState.activeFilterCount(null, null, null, "", ""))
    }

    @Test
    fun activeFilterCount_priceRangeCountsOnceEvenWithBothBounds() {
        assertEquals(1, SearchState.activeFilterCount(null, null, null, "100", "900"))
    }

    @Test
    fun activeFilterCount_nearMeCountsWhenLocationResolved() {
        // type (1) + active Near-Me location (1) = 2
        val count =
            SearchState.activeFilterCount(
                type = ListingType.SALE,
                category = null,
                minRooms = null,
                minPrice = "",
                maxPrice = "",
                nearLat = 48.0,
                nearLng = 17.0,
                radiusKm = 10.0,
            )
        assertEquals(2, count)
    }

    @Test
    fun activeFilterCount_nearMeNotCountedUntilLocationResolved() {
        // Radius chosen but no coordinate yet → not an active filter, badge stays at 0.
        assertEquals(0, SearchState.activeFilterCount(null, null, null, "", "", radiusKm = 10.0))
    }

    // ── shouldLoadNextPage (AC-4 infinite scroll) ────────────────────────

    @Test
    fun shouldLoadNextPage_triggersNearEnd() {
        // 20 loaded of 100, last visible at index 16 → within threshold (20-5=15)
        assertTrue(SearchState.shouldLoadNextPage(16, 20, 100, isLoading = false))
    }

    @Test
    fun shouldLoadNextPage_notTriggeredWhenFarFromEnd() {
        assertFalse(SearchState.shouldLoadNextPage(5, 20, 100, isLoading = false))
    }

    @Test
    fun shouldLoadNextPage_notTriggeredWhileLoading() {
        assertFalse(SearchState.shouldLoadNextPage(19, 20, 100, isLoading = true))
    }

    @Test
    fun shouldLoadNextPage_notTriggeredWhenAllLoaded() {
        assertFalse(SearchState.shouldLoadNextPage(19, 20, 20, isLoading = false))
    }

    @Test
    fun shouldLoadNextPage_notTriggeredWhenEmpty() {
        assertFalse(SearchState.shouldLoadNextPage(null, 0, 0, isLoading = false))
    }

    // ── nextPageTriggerFlow (AC-4 infinite-scroll pipeline) ──────────────
    //
    // These pin the Flow pipeline that drives pagination — the piece that previously lived only
    // inline in SearchScreen.kt's `snapshotFlow { ... }.distinctUntilChanged().collect { ... }`
    // LaunchedEffect and so had no unit coverage (the `shouldLoadNextPage_*` cases above only pin
    // the threshold predicate, not the de-dup / snapshot / emit-page wiring). runTest drives the
    // flow on virtual time so the contract is deterministic and instant.

    @Test
    fun nextPageTriggerFlow_emitsNextPageWhenScrolledNearEnd() = runTest {
        // Page 1 of 100 loaded; the last-visible index crosses the prefetch threshold (20-5=15).
        val snap = pageSnapshot(loadedCount = 20, total = 100, currentPage = 1)
        // Only the threshold-crossing index (16) fires, and it maps to currentPage + 1.
        assertEquals(listOf(2), triggeredPages(snap, 0, 8, 16))
    }

    @Test
    fun nextPageTriggerFlow_doesNotEmitWhileFarFromEnd() = runTest {
        val snap = pageSnapshot(loadedCount = 20, total = 100, currentPage = 1)
        assertEquals(emptyList(), triggeredPages(snap, 0, 3, 5))
    }

    @Test
    fun nextPageTriggerFlow_doesNotEmitWhileLoading() = runTest {
        // A page request is already in flight → no duplicate fetch even at the threshold.
        val snap = pageSnapshot(loadedCount = 20, total = 100, currentPage = 1, isLoading = true)
        assertEquals(emptyList(), triggeredPages(snap, 16, 18, 19))
    }

    @Test
    fun nextPageTriggerFlow_doesNotEmitWhenAllResultsLoaded() = runTest {
        // Everything is already loaded (loadedCount >= total) → end of pagination, no fetch.
        val snap = pageSnapshot(loadedCount = 20, total = 20, currentPage = 1)
        assertEquals(emptyList(), triggeredPages(snap, 15, 18, 19))
    }

    @Test
    fun nextPageTriggerFlow_dedupesRepeatedIndexSoEachBoundaryFiresOnce() = runTest {
        // The bottom row is reported repeatedly while the user dwells at the end;
        // distinctUntilChanged means only the first crossing of a given index produces a fetch —
        // not one per recomposition.
        val snap = pageSnapshot(loadedCount = 20, total = 100, currentPage = 1)
        assertEquals(listOf(2), triggeredPages(snap, 16, 16, 16))
    }

    @Test
    fun nextPageTriggerFlow_ignoresNullIndexFromEmptyList() = runTest {
        // An empty list reports a null last-visible index → nothing to paginate.
        val snap = pageSnapshot(loadedCount = 0, total = 0, currentPage = 1)
        assertEquals(emptyList(), triggeredPages(snap, null))
    }

    @Test
    fun nextPageTriggerFlow_advancesAcrossPagesAsSnapshotMoves() = runTest {
        // The snapshot lambda reads live state: after page 2 lands (40 loaded, currentPage=2) a
        // later index crossing fetches page 3 — proving the helper uses the *current* page, not a
        // captured one.
        var snap = pageSnapshot(loadedCount = 20, total = 100, currentPage = 1)
        val firstFlow = SearchState.nextPageTriggerFlow(flowOf<Int?>(16), snapshot = { snap })
        assertEquals(listOf(2), firstFlow.toList())

        // Simulate page 2 having been applied.
        snap = snap.copy(loadedCount = 40, currentPage = 2)
        val secondFlow = SearchState.nextPageTriggerFlow(flowOf<Int?>(36), snapshot = { snap })
        assertEquals(listOf(3), secondFlow.toList())
    }

    private fun pageSnapshot(
        loadedCount: Int,
        total: Int,
        currentPage: Int,
        isLoading: Boolean = false,
    ) =
        SearchState.PageSnapshot(
            loadedCount = loadedCount,
            total = total,
            isLoading = isLoading,
            currentPage = currentPage,
        )

    private suspend fun triggeredPages(snapshot: SearchState.PageSnapshot, vararg indices: Int?) =
        SearchState.nextPageTriggerFlow(flowOf(*indices), snapshot = { snapshot }).toList()

    // ── mergePage ────────────────────────────────────────────────────────

    @Test
    fun mergePage_firstPageReplaces() {
        val existing = listOf(summary("a"))
        val incoming = listOf(summary("b"), summary("c"))
        assertEquals(incoming, SearchState.mergePage(existing, incoming, page = 1))
    }

    @Test
    fun mergePage_laterPageAppends() {
        val existing = listOf(summary("a"))
        val incoming = listOf(summary("b"))
        val merged = SearchState.mergePage(existing, incoming, page = 2)
        assertEquals(listOf("a", "b"), merged.map { it.id })
    }

    // ── shouldApplyResponse (stale-response race guard) ──────────────────

    @Test
    fun shouldApplyResponse_appliesLatestGeneration() {
        // Response belongs to the generation currently in effect → apply it.
        assertTrue(SearchState.shouldApplyResponse(responseGeneration = 3, currentGeneration = 3))
    }

    @Test
    fun shouldApplyResponse_discardsStaleOlderResponse() {
        // A slower OLDER request (gen 1) completes after a NEWER one (gen 2) was launched.
        // Its response must NOT be applied — this is the bug being fixed.
        assertFalse(SearchState.shouldApplyResponse(responseGeneration = 1, currentGeneration = 2))
    }

    @Test
    fun shouldApplyResponse_staleResponseDoesNotOverwriteNewerResult() {
        // End-to-end shape of the race: a NEWER query's result is already applied; the OLDER
        // query's response then arrives out of order and must be dropped, leaving the newer
        // result intact.
        val currentGeneration = 5L
        val newerResult = listOf(summary("new-1"), summary("new-2"))

        var applied = newerResult

        // The stale (older) response tries to clobber the state.
        val staleResponse = listOf(summary("old-1"))
        if (SearchState.shouldApplyResponse(responseGeneration = 4L, currentGeneration)) {
            applied = SearchState.mergePage(applied, staleResponse, page = 1)
        }

        assertEquals(newerResult, applied)
        assertEquals(listOf("new-1", "new-2"), applied.map { it.id })
    }

    @Test
    fun shouldApplyResponse_futureGenerationIsTreatedAsStale() {
        // A generation ahead of current is impossible by construction; guard rejects it too.
        assertFalse(SearchState.shouldApplyResponse(responseGeneration = 9, currentGeneration = 7))
    }

    // ── searchResultMeta ─────────────────────────────────────────────────

    @Test
    fun searchResultMeta_roomsAndArea_joinedWithSeparator() {
        assertEquals("3 izby · 84 m²", SearchState.searchResultMeta(rooms = 3, areaSqm = 84.0))
    }

    @Test
    fun searchResultMeta_areaTruncatedToWholeNumber() {
        assertEquals("2 izby · 64 m²", SearchState.searchResultMeta(rooms = 2, areaSqm = 64.9))
    }

    @Test
    fun searchResultMeta_onlyRooms() {
        assertEquals("1 izby", SearchState.searchResultMeta(rooms = 1, areaSqm = null))
    }

    @Test
    fun searchResultMeta_onlyArea() {
        assertEquals("50 m²", SearchState.searchResultMeta(rooms = null, areaSqm = 50.0))
    }

    @Test
    fun searchResultMeta_neither_isEmpty() {
        assertEquals("", SearchState.searchResultMeta(rooms = null, areaSqm = null))
    }

    private fun summary(id: String) =
        ListingSummary(
            id = id,
            title = "t",
            type = ListingType.SALE,
            category = PropertyCategory.APARTMENT,
            status = ListingStatus.ACTIVE,
            price = 1,
            address = Address(city = "c", country = "SK"),
            createdAt = "2026-01-01T00:00:00Z",
            updatedAt = "2026-01-01T00:00:00Z",
        )
}
