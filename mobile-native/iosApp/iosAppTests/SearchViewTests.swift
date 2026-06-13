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
