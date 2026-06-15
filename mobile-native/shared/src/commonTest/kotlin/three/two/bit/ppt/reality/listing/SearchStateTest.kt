package three.two.bit.ppt.reality.listing

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

/**
 * Unit tests for [SearchState] — the pure logic behind the Reality mobile home/search screen.
 *
 * Epic 82 - Story 82.3 (AC-2 debounced search, AC-4 infinite scroll + FilterSheet). These pin the
 * infinite-scroll trigger and request/merge semantics so the Compose screen stays a thin shell.
 */
class SearchStateTest {

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
        assertEquals(
            0,
            SearchState.activeFilterCount(null, null, null, "", "", radiusKm = 10.0),
        )
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
