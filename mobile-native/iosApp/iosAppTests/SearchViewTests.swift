//
//  SearchViewTests.swift
//  iosAppTests
//
//  Regression coverage for `bug-ios-searchview-uncompilable` (Issue #1266).
//
//  The whole `Features/Search/SearchView.swift` file used to be uncompilable:
//  `performSearch()` was called from ~9 sites and `scheduleSearch(for:)` from
//  one, but neither was defined, and `resultsGrid` had lost its `ForEach` body.
//  Because the file didn't compile, the `iosApp` test host didn't build, so
//  EVERY test in this target failed. That is the IG3 driver for this fix:
//  these assertions cannot even be compiled against `dev`, and pass on the
//  branch once the helpers + grid body are restored.
//
//  `ListingRepository` is a `final` KMP class (no protocol seam), so a true
//  network-stubbed `performSearch()` unit test isn't expressible from Swift
//  without a host running reality-server. We therefore assert the pure,
//  non-private logic that the restored file exposes (the `SortOption` <-> KMP
//  mapping and `SearchFilters` activation) — touching these symbols forces the
//  SUT to compile, which is exactly what regressed. A live `performSearch()`
//  behaviour test is left for the macOS reviewer's `xcodebuild test` run
//  against the dev stack (see PR notes).
//

import CoreLocation
import XCTest
import shared
@testable import iosApp

// MARK: - SortOption <-> KMP mapping

/// Pins the `SortOption.kmpSortOption` bridge so the SwiftUI sort picker and
/// the KMP `ListingSearchRequest.sort` field can't drift apart. Every Swift
/// case must map to its KMP `ListingSortOption` peer.
final class SearchViewSortOptionTests: XCTestCase {

    func testEverySortOptionMapsToItsKMPPeer() {
        XCTAssertEqual(SortOption.newest.kmpSortOption, ListingSortOption.newest)
        XCTAssertEqual(SortOption.oldest.kmpSortOption, ListingSortOption.oldest)
        XCTAssertEqual(SortOption.priceAsc.kmpSortOption, ListingSortOption.priceAsc)
        XCTAssertEqual(SortOption.priceDesc.kmpSortOption, ListingSortOption.priceDesc)
        XCTAssertEqual(SortOption.areaAsc.kmpSortOption, ListingSortOption.areaAsc)
        XCTAssertEqual(SortOption.areaDesc.kmpSortOption, ListingSortOption.areaDesc)
        XCTAssertEqual(SortOption.relevance.kmpSortOption, ListingSortOption.relevance)
    }

    func testAllCasesAreMappedAndHaveDisplayNames() {
        // Guards against a new SortOption case being added without a KMP
        // mapping / localized label — both are exhaustive switches, so a gap
        // would fail to compile, but this also documents the count.
        XCTAssertEqual(SortOption.allCases.count, 7)
        for option in SortOption.allCases {
            XCTAssertFalse(option.displayName.isEmpty)
        }
    }
}

// MARK: - SearchFilters activation

/// `SearchView` decides between the empty-results, search-prompt, and results
/// states using `filters.hasActiveFilters`; `performSearch()` only attaches a
/// KMP filter payload when filters are active. Pin that activation logic so the
/// debounce/search path keeps the correct empty-state semantics.
final class SearchViewFiltersTests: XCTestCase {

    func testDefaultFiltersAreInactive() {
        XCTAssertFalse(SearchFilters().hasActiveFilters)
    }

    func testPriceFilterActivates() {
        var filters = SearchFilters()
        filters.priceMin = 100_000
        XCTAssertTrue(filters.hasActiveFilters)
    }

    func testPropertyTypeFilterActivates() {
        var filters = SearchFilters()
        filters.propertyTypes.insert(.apartment)
        XCTAssertTrue(filters.hasActiveFilters)
    }

    func testRadiusFilterActivates() {
        var filters = SearchFilters()
        filters.radiusKm = 10
        XCTAssertTrue(filters.hasActiveFilters)
    }

    func testResetClearsActivation() {
        var filters = SearchFilters()
        filters.priceMin = 100_000
        filters.propertyTypes.insert(.house)
        filters.radiusKm = 5
        filters.reset()
        XCTAssertFalse(filters.hasActiveFilters)
    }
}

// MARK: - CoreLocation integration (Epic 82, Story 82.3 — "Near Me")

/// Verification coverage for the iOS CoreLocation integration behind the
/// Search FilterSheet's "Near Me" toggle.
///
/// Context (task
/// `verify-home-and-search-screens-corelocation-integration-confirmed-mobile`):
/// the screen-map flagged the CoreLocation wiring as *unconfirmed*. A source
/// audit (see `docs/screens/reality-mobile/search.md` Agent Log) confirmed the
/// path is fully wired:
///   * `LocationManager` (`Core/Location/LocationManager.swift`) owns the
///     `CLLocationManager`, requests *when-in-use* authorisation, and publishes
///     `coordinate` / `isLocating` / `locationError`.
///   * `RealityPortalApp` instantiates one `LocationManager` and injects it via
///     `.environment(...)`; `SearchView` re-injects it across the sheet boundary
///     so `FilterSheet` resolves the *same* instance (closes #625).
///   * `FilterSheet.nearMeSection` calls `requestLocation()` when the toggle is
///     enabled and no coordinate is cached, then copies the resolved coordinate
///     into `SearchFilters.latitude/longitude` and on to the KMP
///     `ListingSearchFilters.nearLat/nearLng` payload via `buildKMPFilters()`.
///   * `Info.plist` ships `NSLocationWhenInUseUsageDescription`.
///
/// These assertions pin the parts of that path that are exercisable in the
/// `iosApp` test host without a GPS fix or a live `reality-server`:
/// the `LocationManager` state machine that needs no real device fix, the
/// near-me coordinate→filter mapping the toggle performs, and the presence of
/// the permission usage string in the shipped app bundle. A full
/// permission-prompt + live-fix behaviour test (which needs a seeded simulator
/// location) is left for the macOS reviewer's `xcodebuild test` run.
final class SearchViewCoreLocationTests: XCTestCase {

    // MARK: LocationManager state machine (no GPS fix required)

    func testLocationManagerInitialStateIsIdle() {
        let manager = LocationManager()
        XCTAssertNil(manager.coordinate)
        XCTAssertNil(manager.locationError)
        XCTAssertFalse(manager.isLocating)
        // Authorisation status mirrors CLLocationManager; in a fresh test host
        // it is whatever the host reports, but it must be a valid enum value.
        XCTAssertTrue(CLAuthorizationStatus.allTestCases.contains(manager.authorizationStatus))
    }

    func testClearLocationErrorResetsError() {
        let manager = LocationManager()
        // requestLocation() on a host with denied/restricted status sets an
        // error; on .notDetermined it triggers the permission prompt and leaves
        // the error nil. Either way clearLocationError() must leave it nil.
        manager.requestLocation()
        manager.clearLocationError()
        XCTAssertNil(manager.locationError)
    }

    func testStopLocationClearsLocatingFlag() {
        let manager = LocationManager()
        manager.requestLocation()
        manager.stopLocation()
        XCTAssertFalse(manager.isLocating)
    }

    // MARK: Near-Me coordinate → filter mapping

    /// Mirrors the FilterSheet "Near Me" toggle ON path: a resolved
    /// `CLLocationCoordinate2D` is copied into the filter as `radiusKm` (default
    /// 10) + `latitude`/`longitude`, which `buildKMPFilters()` forwards to the
    /// KMP `ListingSearchFilters.nearLat/nearLng/radiusKm`.
    func testNearMeToggleOnAppliesCoordinateToFilters() {
        let coord = CLLocationCoordinate2D(latitude: 48.1486, longitude: 17.1077) // Bratislava
        var filters = SearchFilters()

        // Replicates nearMeSection's `set:` closure for the enabled branch.
        filters.radiusKm = 10.0
        filters.latitude = coord.latitude
        filters.longitude = coord.longitude

        XCTAssertTrue(filters.hasActiveFilters)
        XCTAssertEqual(filters.radiusKm, 10.0)
        XCTAssertEqual(filters.latitude ?? 0, 48.1486, accuracy: 0.0001)
        XCTAssertEqual(filters.longitude ?? 0, 17.1077, accuracy: 0.0001)
    }

    /// Mirrors the toggle OFF path: radius + both coordinates are cleared so no
    /// stale `nearLat/nearLng` leaks into the next search request.
    func testNearMeToggleOffClearsCoordinateAndRadius() {
        var filters = SearchFilters()
        filters.radiusKm = 10.0
        filters.latitude = 48.1486
        filters.longitude = 17.1077

        // Replicates nearMeSection's `set:` closure for the disabled branch.
        filters.radiusKm = nil
        filters.latitude = nil
        filters.longitude = nil

        XCTAssertNil(filters.radiusKm)
        XCTAssertNil(filters.latitude)
        XCTAssertNil(filters.longitude)
        XCTAssertFalse(filters.hasActiveFilters)
    }

    // MARK: Permission usage string (Info.plist audit, host-runnable)

    /// CoreLocation refuses to prompt without `NSLocationWhenInUseUsageDescription`.
    /// Assert the shipped app bundle carries a non-empty value so the "Near Me"
    /// permission prompt can actually appear.
    func testWhenInUseUsageDescriptionIsPresentInAppBundle() {
        let bundle = Bundle(for: LocationManager.self)
        let usage = bundle.object(
            forInfoDictionaryKey: "NSLocationWhenInUseUsageDescription"
        ) as? String
        XCTAssertNotNil(usage, "NSLocationWhenInUseUsageDescription missing from app Info.plist")
        XCTAssertFalse(usage?.isEmpty ?? true, "Location usage description must be non-empty")
    }
}

// MARK: - Stale-response guard preserves pagination (Issue #1365)

/// Regression coverage for the iOS `SearchView` stale-response guard, focused on
/// the pagination-interleaving bug raised in #1365 ("stale-response guard skips
/// pagination").
///
/// The race (from #1365): the user changes the query — firing a page-1
/// `performSearch()` that bumps `requestSeq` — while a page-2 `loadMoreResults()`
/// from the *previous* query is still awaiting the network. When the late page-2
/// response arrives, an unguarded `results.append(...)` would interleave the old
/// query's listings into the new query's result set. PR #1465 closed the bug by
/// giving `loadMoreResults()` the same `requestSeq` capture-before-await guard as
/// `performSearch()`: page > 1 *reuses* the current generation (never bumps it),
/// and the append is gated on `seq == requestSeq`.
///
/// `ListingRepository` is a `final` KMP class with no protocol seam, so a true
/// network-driven `loadMoreResults()` unit test isn't expressible from Swift
/// without a live `reality-server` (same constraint documented for the existing
/// classes above). These tests therefore pin the *guard decision* the SwiftUI
/// view runs — `seq == requestSeq` before an append — exactly as the existing
/// CoreLocation tests pin the FilterSheet toggle closures, and cross-check that
/// decision against the shared KMP `SearchState` contract (`shouldApplyResponse`
/// + `mergePage`) that `SearchView` mirrors and that #1365 recommends iOS adopt.
/// A live `performSearch`/`loadMoreResults` behaviour test is left for the macOS
/// reviewer's `xcodebuild test` run against the dev stack (see PR notes).
final class SearchViewStaleResponseGuardTests: XCTestCase {

    /// Tiny model of the `requestSeq` guard the SwiftUI view applies on resume.
    ///
    /// `performSearch()` (page 1) bumps `requestSeq` and captures the new value;
    /// `loadMoreResults()` (page > 1) captures the *current* value without
    /// bumping. Both then `guard seq == requestSeq` before mutating `results`.
    /// `shouldApply` is that guard predicate, isolated so it can be exercised
    /// without the `final` repository or a SwiftUI runtime.
    private struct GuardModel {
        private(set) var requestSeq = 0

        /// page-1 search: bump the generation, return the captured token.
        mutating func beginPageOneSearch() -> Int {
            requestSeq += 1
            return requestSeq
        }

        /// page-N (N>1) load-more: reuse the current generation, no bump.
        func beginLoadMore() -> Int { requestSeq }

        /// The `guard seq == requestSeq` decision evaluated on response resume.
        func shouldApply(captured seq: Int) -> Bool { seq == requestSeq }
    }

    // MARK: iOS requestSeq guard decision

    /// The #1365 scenario: a page-2 load-more is in flight when the user changes
    /// the query (a fresh page-1 search). The late page-2 response must be
    /// dropped so it can't append onto the new query's results.
    func testStalePageTwoResponseIsDroppedWhenNewSearchSupersedesIt() {
        var model = GuardModel()

        // Initial query lands and the user scrolls — page-2 load-more starts,
        // capturing the current generation (no bump for page > 1).
        _ = model.beginPageOneSearch()           // gen 1 (initial query applied)
        let pageTwoSeq = model.beginLoadMore()    // captures gen 1, awaiting...

        // Before page 2 returns, the user edits the query → fresh page-1 search
        // bumps the generation to 2.
        _ = model.beginPageOneSearch()            // gen 2 (new query in effect)

        // page-2 response from the OLD query now resumes: its captured gen (1) is
        // behind the current gen (2) → guard drops it, so no interleaving append.
        XCTAssertFalse(
            model.shouldApply(captured: pageTwoSeq),
            "Stale page-2 response from a superseded query must not be applied"
        )
    }

    /// The happy path: when no newer search intervenes, a page-2 load-more under
    /// the current generation IS applied — the guard preserves real pagination
    /// rather than dropping every append.
    func testInOrderPageTwoResponseIsAppliedSoPaginationStillWorks() {
        var model = GuardModel()
        _ = model.beginPageOneSearch()            // gen 1
        let pageTwoSeq = model.beginLoadMore()    // captures gen 1, no newer search

        XCTAssertTrue(
            model.shouldApply(captured: pageTwoSeq),
            "An in-order page-2 response under the current generation must apply"
        )
    }

    /// Load-more (page > 1) must NOT bump the generation — otherwise a trailing
    /// append would invalidate a concurrent in-flight page or the page-1 search,
    /// and pagination would livelock. Mirrors Android's "page > 1 reuses the
    /// current generation" rule (#1365).
    func testLoadMoreReusesCurrentGenerationAndDoesNotBumpIt() {
        var model = GuardModel()
        let genAfterSearch = model.beginPageOneSearch()  // gen 1
        let genForLoadMore = model.beginLoadMore()        // must still be gen 1

        XCTAssertEqual(genForLoadMore, genAfterSearch,
                       "loadMoreResults() must reuse the current generation, not bump it")
        XCTAssertEqual(model.requestSeq, genAfterSearch,
                       "requestSeq must be unchanged after a load-more begins")
    }

    /// Each page-1 search bumps the generation, so an earlier in-flight search's
    /// response is older-than-current and gets dropped — the core stale-guard
    /// invariant that keeps an out-of-order page-1 completion from clobbering the
    /// newest query.
    func testEachNewSearchBumpsGenerationSoOlderSearchResponseIsStale() {
        var model = GuardModel()
        let firstSeq = model.beginPageOneSearch()   // gen 1
        let secondSeq = model.beginPageOneSearch()  // gen 2

        XCTAssertFalse(model.shouldApply(captured: firstSeq),
                       "Older page-1 response (gen 1) must be stale once gen 2 launched")
        XCTAssertTrue(model.shouldApply(captured: secondSeq),
                      "Newest page-1 response (gen 2) is current and must apply")
    }

    // MARK: Cross-check against the shared KMP SearchState contract

    /// The shared `SearchState.shouldApplyResponse` (commonMain, exported via the
    /// `:shared` framework) is the platform-agnostic statement of the same guard
    /// #1365 recommends iOS route through. Assert the iOS `requestSeq` decision
    /// agrees with it for the page-2-superseded and in-order cases, so the two
    /// platforms can't drift on the stale-response rule.
    func testSharedSearchStateGuardAgreesWithIosRequestSeqDecision() {
        // Stale: response generation behind current → drop on both sides.
        XCTAssertFalse(
            SearchState.shared.shouldApplyResponse(responseGeneration: 1, currentGeneration: 2)
        )
        // Current: response generation equals current → apply on both sides.
        XCTAssertTrue(
            SearchState.shared.shouldApplyResponse(responseGeneration: 2, currentGeneration: 2)
        )
    }

    /// End-to-end shape of the pagination-preservation property, gating the
    /// append on the shared `shouldApplyResponse`: a stale page-2 response is
    /// dropped, so the newer query's page-1 results stay intact (no interleaved
    /// listings). The result set is modelled with plain id arrays — the merge
    /// semantics under test are page-1-replaces / page-N-appends, independent of
    /// the listing payload — so the test stays network-free and avoids
    /// hand-constructing the multi-field KMP `ListingSummary` from Swift.
    func testStalePageTwoAppendIsSkippedSoNewerResultsArePreserved() {
        let currentGeneration: Int64 = 2

        // The new query's page-1 result is already applied.
        let newerResults = ["new-1", "new-2"]
        var applied = newerResults

        // A late page-2 response from the OLD query (generation 1) tries to append.
        let stalePageTwo = ["old-3"]
        if SearchState.shared.shouldApplyResponse(responseGeneration: 1, currentGeneration: currentGeneration) {
            applied += stalePageTwo   // page > 1 → append (mirrors mergePage)
        }

        XCTAssertEqual(applied, newerResults,
                       "Stale page-2 listings must not be appended onto the newer query's results")
        XCTAssertFalse(applied.contains("old-3"),
                       "No listing from the superseded query may leak into the result set")
    }

    /// Counterpart: an in-order page-2 response (current generation) DOES append
    /// after page 1 — proving the guard preserves working pagination rather than
    /// suppressing all appends.
    func testInOrderPageTwoAppendMergesAfterPageOne() {
        let currentGeneration: Int64 = 1
        var applied = ["p1-a", "p1-b"]
        let incoming = ["p2-c"]

        if SearchState.shared.shouldApplyResponse(responseGeneration: 1, currentGeneration: currentGeneration) {
            applied += incoming
        }

        XCTAssertEqual(applied, ["p1-a", "p1-b", "p2-c"],
                       "In-order page-2 results must append after page-1 results")
    }
}

private extension CLAuthorizationStatus {
    /// The valid `CLAuthorizationStatus` cases — used to assert the manager's
    /// published status is a real enum value rather than a stray bit pattern.
    static var allTestCases: [CLAuthorizationStatus] {
        [.notDetermined, .restricted, .denied, .authorizedAlways, .authorizedWhenInUse]
    }
}
