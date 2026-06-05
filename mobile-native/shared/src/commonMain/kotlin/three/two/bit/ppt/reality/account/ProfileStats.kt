package three.two.bit.ppt.reality.account

import three.two.bit.ppt.reality.favorites.FavoritesRepository
import three.two.bit.ppt.reality.inquiry.InquiryRepository

/**
 * Aggregated counts shown in the Account / Profile screen stat strip.
 *
 * Each field is nullable: a `null` means "count unknown" (the underlying request failed) and the UI
 * renders an em-dash rather than a misleading `0`. The reality-server has no single `/me/stats`
 * endpoint, so the counts are derived from the `total` field returned by the three list endpoints
 * the app already calls:
 *
 * - favorites — `GET /api/v1/favorites` → `FavoritesResponse.total`
 * - searches  — `GET /api/v1/saved-searches` → `SavedSearchesResponse.total`
 * - inquiries — `GET /api/v1/inquiries` → `InquiriesResponse.total`
 *
 * Epic 48 - Story 48.5: Portal Mobile Account.
 */
data class ProfileStats(val favorites: Int?, val searches: Int?, val inquiries: Int?) {
    companion object {
        /** All-unknown stats — the initial state before the network calls resolve. */
        val UNKNOWN = ProfileStats(favorites = null, searches = null, inquiries = null)
    }
}

/**
 * Loads the profile stat counts for the signed-in user.
 *
 * Lives in `commonMain` (and is unit-tested in `commonTest`) so the count-derivation logic is shared
 * with iOS and verifiable without an Android build. The three calls are independent: a failure in
 * one leaves the others intact and only that field falls back to `null`.
 *
 * The class takes an [InquiryRepository] and a [FavoritesRepository] (which also owns saved
 * searches) rather than constructing its own, so callers can inject the session-scoped instances and
 * tests can supply mock-engine-backed repositories.
 */
class ProfileStatsLoader(
    private val favoritesRepository: FavoritesRepository,
    private val inquiryRepository: InquiryRepository,
) {
    suspend fun load(): ProfileStats {
        val favorites =
            favoritesRepository.getFavorites().getOrNull()?.total?.toIntClampedOrNull()
        val searches = favoritesRepository.getSavedSearches().getOrNull()?.total
        val inquiries = inquiryRepository.getInquiries().getOrNull()?.total
        return ProfileStats(
            favorites = favorites,
            searches = searches,
            inquiries = inquiries,
        )
    }
}

/**
 * `FavoritesResponse.total` is a `Long`; the stat strip renders an `Int`. Clamp to [Int.MAX_VALUE]
 * defensively rather than overflowing on an implausibly large count.
 */
private fun Long.toIntClampedOrNull(): Int = if (this > Int.MAX_VALUE) Int.MAX_VALUE else toInt()
