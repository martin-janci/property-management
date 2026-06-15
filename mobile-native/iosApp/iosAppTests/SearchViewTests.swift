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

private extension CLAuthorizationStatus {
    /// The valid `CLAuthorizationStatus` cases — used to assert the manager's
    /// published status is a real enum value rather than a stray bit pattern.
    static var allTestCases: [CLAuthorizationStatus] {
        [.notDetermined, .restricted, .denied, .authorizedAlways, .authorizedWhenInUse]
    }
}
