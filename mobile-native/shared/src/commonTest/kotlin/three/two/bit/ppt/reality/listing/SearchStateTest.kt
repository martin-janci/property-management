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
