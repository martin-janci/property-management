//
//  RealityPortalTests.swift
//  iosAppTests
//
//  Epic 80 - Story 80.6: Mobile Unit Tests (iOS)
//

import XCTest
@testable import iosApp

// MARK: - Configuration Tests

/// Verifies the `Configuration` singleton + `Environment` enum that drives
/// per-build API URLs and deep-link schemes.
final class ConfigurationTests: XCTestCase {

    func testSharedConfigurationIsAccessible() throws {
        // The singleton must be reachable so DI containers can pull URLs
        // off it during app launch. (No identity check because Swift
        // singletons aren't reference-equal across module boundaries in
        // some CI matrices.)
        XCTAssertFalse(Configuration.shared.bundleIdentifier.isEmpty)
        XCTAssertFalse(Configuration.shared.appName.isEmpty)
    }

    func testBundleIdentifierMatchesNamespace() throws {
        // Pinned by `CLAUDE.md`'s namespace table.
        XCTAssertEqual(
            Configuration.shared.bundleIdentifier,
            "three.two.bit.ppt.reality"
        )
    }

    func testEnvironmentApiBaseUrls() throws {
        // Each environment must produce a non-empty URL with the right
        // scheme so DI doesn't accidentally point at `localhost` in
        // staging/production builds.
        XCTAssertTrue(Environment.development.apiBaseUrl.hasPrefix("http://"))
        XCTAssertTrue(Environment.staging.apiBaseUrl.hasPrefix("https://"))
        XCTAssertTrue(Environment.production.apiBaseUrl.hasPrefix("https://"))
        XCTAssertNotEqual(Environment.development.apiBaseUrl, Environment.production.apiBaseUrl)
    }

    func testEnvironmentDeepLinkSchemeIsStable() throws {
        // The Android `MainActivity.handleDeepLink` checks `scheme == "reality"`;
        // iOS must agree or universal deep-links break. NOTE: Android uses
        // "reality"; iOS uses "realityportal" — this test pins the iOS value
        // so any rename surfaces here, then a follow-up can reconcile both.
        XCTAssertEqual(Environment.development.urlScheme, "realityportal")
        XCTAssertEqual(Environment.staging.urlScheme, "realityportal")
        XCTAssertEqual(Environment.production.urlScheme, "realityportal")
    }

    func testWebBaseUrlsAreEnvironmentSpecific() throws {
        XCTAssertNotEqual(Environment.development.webBaseUrl, Environment.staging.webBaseUrl)
        XCTAssertNotEqual(Environment.staging.webBaseUrl, Environment.production.webBaseUrl)
    }

    func testApiBaseUrlIsForwardedFromEnvironment() throws {
        // Configuration.apiBaseUrl is a thin forwarder; verify the layer
        // doesn't accidentally hard-code a literal.
        XCTAssertEqual(
            Configuration.shared.apiBaseUrl,
            Configuration.shared.environment.apiBaseUrl
        )
    }

    func testKeychainServiceMatchesBundleId() throws {
        // The Keychain service name has to be stable across launches —
        // pinning it to the bundle id keeps it that way and prevents
        // accidental drift that would orphan stored credentials.
        XCTAssertEqual(
            Configuration.shared.keychainService,
            Configuration.shared.bundleIdentifier
        )
    }
}

// MARK: - Route Tests

/// Verifies the `Route` enum's identity behaviour. Compose-Navigation /
/// SwiftUI NavigationStack rely on `Hashable` equality to deduplicate
/// route stacks, so `searchResults` cases with identical payloads must
/// compare equal even when constructed independently.
final class RouteTests: XCTestCase {

    func testSimpleRoutesAreEqual() throws {
        XCTAssertEqual(Route.home, Route.home)
        XCTAssertEqual(Route.search, Route.search)
        XCTAssertEqual(Route.favorites, Route.favorites)
    }

    func testListingDetailEqualityRespectsId() throws {
        XCTAssertEqual(Route.listingDetail(id: "lst-1"), Route.listingDetail(id: "lst-1"))
        XCTAssertNotEqual(Route.listingDetail(id: "lst-1"), Route.listingDetail(id: "lst-2"))
    }

    func testHashabilityAcrossEquivalentValues() throws {
        // Two Routes that are == must share a hash so a Set/Dictionary
        // backed nav stack treats them as the same entry.
        var bucket = Set<Route>()
        bucket.insert(.listingDetail(id: "lst-1"))
        bucket.insert(.listingDetail(id: "lst-1"))
        bucket.insert(.listingDetail(id: "lst-2"))
        XCTAssertEqual(bucket.count, 2)
    }
}

// MARK: - Authentication Tests

/// Smoke tests for the `AuthManager` lifecycle. They operate on a fresh
/// instance and avoid touching the real Keychain by relying on the
/// public API — the underlying storage is exercised by integration
/// tests on a device.
final class AuthenticationTests: XCTestCase {

    func testInitialAuthStateIsUnauthenticated() throws {
        let auth = AuthManager()
        XCTAssertFalse(auth.isAuthenticated)
    }
}

// MARK: - Deep Link / URL Parsing Tests

/// The iOS app accepts `realityportal://...` URLs. These tests pin the
/// parsing behavior the SceneDelegate relies on so a typo in the
/// scheme/host wiring surfaces immediately rather than at runtime.
final class DeepLinkTests: XCTestCase {

    func testCustomSchemeUrlParses() throws {
        let url = try XCTUnwrap(URL(string: "realityportal://listing/lst-42"))
        XCTAssertEqual(url.scheme, Environment.development.urlScheme)
        XCTAssertEqual(url.host, "listing")
        XCTAssertEqual(url.lastPathComponent, "lst-42")
    }

    func testSsoCallbackQueryParametersAreParseable() throws {
        let url = try XCTUnwrap(
            URL(string: "realityportal://sso?token=abc.def.ghi&state=42")
        )
        let components = try XCTUnwrap(URLComponents(url: url, resolvingAgainstBaseURL: false))
        let token = components.queryItems?.first { $0.name == "token" }?.value
        let state = components.queryItems?.first { $0.name == "state" }?.value
        XCTAssertEqual(token, "abc.def.ghi")
        XCTAssertEqual(state, "42")
    }

    func testRejectsForeignScheme() throws {
        // `https://` URLs are universal links and should not be parsed by
        // the same code path as the custom scheme — the SceneDelegate
        // routes them differently.
        let url = try XCTUnwrap(URL(string: "https://example.com/listing/lst-1"))
        XCTAssertNotEqual(url.scheme, Environment.development.urlScheme)
    }
}

// MARK: - DeepLinkHandler Tests

/// Unit tests for `DeepLinkHandler` covering every URL pattern in the table
/// documented on the type. These run without SwiftUI state so they are fast
/// and deterministic.
final class DeepLinkHandlerTests: XCTestCase {
    private let handler = DeepLinkHandler()

    // MARK: SSO

    func testSsoCallbackWithToken() throws {
        let url = try XCTUnwrap(URL(string: "realityportal://sso?token=jwt.header.payload&state=xyz"))
        guard case .ssoCallback(let token, let state) = handler.parse(url) else {
            return XCTFail("Expected ssoCallback result")
        }
        XCTAssertEqual(token, "jwt.header.payload")
        XCTAssertEqual(state, "xyz")
    }

    func testSsoCallbackMissingTokenIsUnrecognised() throws {
        let url = try XCTUnwrap(URL(string: "realityportal://sso?state=42"))
        guard case .unrecognized = handler.parse(url) else {
            return XCTFail("Expected unrecognized result for SSO without token")
        }
    }

    // MARK: Listing routes

    func testListingDetailRoute() throws {
        let url = try XCTUnwrap(URL(string: "realityportal://listing/lst-99"))
        guard case .route(let route) = handler.parse(url), case .listingDetail(let id) = route else {
            return XCTFail("Expected listingDetail route")
        }
        XCTAssertEqual(id, "lst-99")
    }

    func testListingGalleryRoute() throws {
        let url = try XCTUnwrap(URL(string: "realityportal://listing/lst-99/gallery"))
        guard case .route(let route) = handler.parse(url), case .listingGallery(let id) = route else {
            return XCTFail("Expected listingGallery route")
        }
        XCTAssertEqual(id, "lst-99")
    }

    func testListingMapRoute() throws {
        let url = try XCTUnwrap(URL(string: "realityportal://listing/lst-99/map"))
        guard case .route(let route) = handler.parse(url), case .listingMap(let id) = route else {
            return XCTFail("Expected listingMap route")
        }
        XCTAssertEqual(id, "lst-99")
    }

    func testListingMissingIdIsUnrecognised() throws {
        let url = try XCTUnwrap(URL(string: "realityportal://listing"))
        guard case .unrecognized = handler.parse(url) else {
            return XCTFail("Expected unrecognized — no listing id")
        }
    }

    // MARK: Search

    func testSearchRouteWithQuery() throws {
        let url = try XCTUnwrap(URL(string: "realityportal://search?q=Bratislava"))
        guard case .route(let route) = handler.parse(url),
              case .searchResults(let query, let filters) = route else {
            return XCTFail("Expected searchResults route")
        }
        XCTAssertEqual(query, "Bratislava")
        XCTAssertNil(filters)
    }

    func testSearchRouteWithFilters() throws {
        let url = try XCTUnwrap(URL(string: "realityportal://search?q=Praha&priceMin=100000&priceMax=500000&types=apartment,house"))
        guard case .route(let route) = handler.parse(url),
              case .searchResults(let query, let filters) = route else {
            return XCTFail("Expected searchResults route")
        }
        XCTAssertEqual(query, "Praha")
        let f = try XCTUnwrap(filters)
        XCTAssertEqual(f.priceMin, 100_000)
        XCTAssertEqual(f.priceMax, 500_000)
        XCTAssertTrue(f.propertyTypes.contains(.apartment))
        XCTAssertTrue(f.propertyTypes.contains(.house))
    }

    func testSearchRouteEmptyQuery() throws {
        let url = try XCTUnwrap(URL(string: "realityportal://search"))
        guard case .route(let route) = handler.parse(url),
              case .searchResults(let query, _) = route else {
            return XCTFail("Expected searchResults route")
        }
        XCTAssertEqual(query, "")
    }

    // MARK: Favorites / Inquiries

    func testFavoritesRoute() throws {
        let url = try XCTUnwrap(URL(string: "realityportal://favorites"))
        guard case .route(let route) = handler.parse(url), case .favorites = route else {
            return XCTFail("Expected favorites route")
        }
    }

    func testInquiriesListRoute() throws {
        let url = try XCTUnwrap(URL(string: "realityportal://inquiries"))
        guard case .route(let route) = handler.parse(url), case .inquiries = route else {
            return XCTFail("Expected inquiries route")
        }
    }

    func testInquiryDetailRoute() throws {
        let url = try XCTUnwrap(URL(string: "realityportal://inquiries/inq-7"))
        guard case .route(let route) = handler.parse(url), case .inquiryDetail(let id) = route else {
            return XCTFail("Expected inquiryDetail route")
        }
        XCTAssertEqual(id, "inq-7")
    }

    func testNewInquiryRoute() throws {
        let url = try XCTUnwrap(URL(string: "realityportal://inquiries/new?listingId=lst-5"))
        guard case .route(let route) = handler.parse(url), case .newInquiry(let listingId) = route else {
            return XCTFail("Expected newInquiry route")
        }
        XCTAssertEqual(listingId, "lst-5")
    }

    func testNewInquiryMissingListingIdIsUnrecognised() throws {
        let url = try XCTUnwrap(URL(string: "realityportal://inquiries/new"))
        guard case .unrecognized = handler.parse(url) else {
            return XCTFail("Expected unrecognized — newInquiry without listingId")
        }
    }

    // MARK: Account sub-routes

    func testAccountRootRoute() throws {
        let url = try XCTUnwrap(URL(string: "realityportal://account"))
        guard case .route(let route) = handler.parse(url), case .account = route else {
            return XCTFail("Expected account route")
        }
    }

    func testProfileRoute() throws {
        let url = try XCTUnwrap(URL(string: "realityportal://account/profile"))
        guard case .route(let route) = handler.parse(url), case .profile = route else {
            return XCTFail("Expected profile route")
        }
    }

    func testSettingsRoute() throws {
        let url = try XCTUnwrap(URL(string: "realityportal://account/settings"))
        guard case .route(let route) = handler.parse(url), case .settings = route else {
            return XCTFail("Expected settings route")
        }
    }

    // MARK: Directory routes

    func testRealtorsRoute() throws {
        let url = try XCTUnwrap(URL(string: "realityportal://realtors"))
        guard case .route(let route) = handler.parse(url), case .realtors = route else {
            return XCTFail("Expected realtors route")
        }
    }

    func testAgenciesRoute() throws {
        let url = try XCTUnwrap(URL(string: "realityportal://agencies"))
        guard case .route(let route) = handler.parse(url), case .agencies = route else {
            return XCTFail("Expected agencies route")
        }
    }

    func testSavedSearchesRoute() throws {
        let url = try XCTUnwrap(URL(string: "realityportal://saved-searches"))
        guard case .route(let route) = handler.parse(url), case .savedSearches = route else {
            return XCTFail("Expected savedSearches route")
        }
    }

    // MARK: Compare

    func testCompareListingsRoute() throws {
        let url = try XCTUnwrap(URL(string: "realityportal://compare?ids=lst-1,lst-2,lst-3"))
        guard case .route(let route) = handler.parse(url), case .compareListings(let ids) = route else {
            return XCTFail("Expected compareListings route")
        }
        XCTAssertEqual(ids, ["lst-1", "lst-2", "lst-3"])
    }

    func testCompareRequiresAtLeastTwoIds() throws {
        let url = try XCTUnwrap(URL(string: "realityportal://compare?ids=lst-1"))
        guard case .unrecognized = handler.parse(url) else {
            return XCTFail("Expected unrecognized — compare with single id")
        }
    }

    func testCompareWithoutIdsParamIsUnrecognised() throws {
        let url = try XCTUnwrap(URL(string: "realityportal://compare"))
        guard case .unrecognized = handler.parse(url) else {
            return XCTFail("Expected unrecognized — compare without ids param")
        }
    }

    // MARK: Unknown

    func testUnknownHostIsUnrecognised() throws {
        let url = try XCTUnwrap(URL(string: "realityportal://unknown/path"))
        guard case .unrecognized = handler.parse(url) else {
            return XCTFail("Expected unrecognized for unknown host")
        }
    }
}

// MARK: - NavigationStateRestorationService Tests

/// Tests for ``NavigationStateRestorationService`` — encode/decode round-trips
/// and auth-gating of protected stacks.
final class NavigationStateRestorationServiceTests: XCTestCase {
    private var service: NavigationStateRestorationService!
    private var testDefaults: UserDefaults!

    override func setUp() {
        super.setUp()
        // Use an isolated UserDefaults suite so tests never pollute each other
        // or the host app's standard defaults.
        let suiteName = "test.NavigationStateRestorationServiceTests.\(UUID().uuidString)"
        testDefaults = UserDefaults(suiteName: suiteName)!
        service = NavigationStateRestorationService(defaults: testDefaults)
    }

    override func tearDown() {
        // Each suite has the same name as the suiteName passed to init.
        // We clear by removing the object from standard defaults to avoid leaks.
        testDefaults.dictionaryRepresentation().keys.forEach { testDefaults.removeObject(forKey: $0) }
        super.tearDown()
    }

    // MARK: Encode / decode

    func testEncodeDecodeRoundTrip() throws {
        let routes: [Route] = [
            .listingDetail(id: "lst-10"),
            .listingGallery(id: "lst-10"),
            .searchResults(query: "Brno", filters: nil),
            .inquiryDetail(id: "inq-3"),
            .profile,
            .settings,
            .realtors,
            .agencies,
            .savedSearches,
        ]

        for route in routes {
            guard let encoded = service.encodeRoute(route) else {
                XCTFail("encodeRoute returned nil for \(route)")
                continue
            }
            let decoded = service.decodeRoute(encoded)
            XCTAssertEqual(decoded, route, "Round-trip failed for \(route) → '\(encoded)' → \(String(describing: decoded))")
        }
    }

    func testCompareListingsIsNotPersisted() throws {
        // compareListings must return nil from encodeRoute (ephemeral).
        let encoded = service.encodeRoute(.compareListings(ids: ["lst-1", "lst-2"]))
        XCTAssertNil(encoded, "compareListings should not be encoded for persistence")
    }

    func testEncodedRoutesAreCorrectLength() throws {
        let routes: [Route] = [.home, .search, .listingDetail(id: "x"), .compareListings(ids: ["a","b"])]
        let encoded = service.encode(routes: routes)
        // compareListings is excluded → 3 items
        XCTAssertEqual(encoded.count, 3)
    }

    // MARK: Save / restore

    func testSaveAndRestorePreservesSelectedTab() throws {
        let coordinator = NavigationCoordinator()
        coordinator.selectedTab = .search
        service.save(coordinator: coordinator)

        let restored = NavigationCoordinator()
        service.restore(into: restored, isAuthenticated: true)
        XCTAssertEqual(restored.selectedTab, .search)
    }

    func testRestoreDropsProtectedTabWhenUnauthenticated() throws {
        let coordinator = NavigationCoordinator()
        coordinator.selectedTab = .favorites   // protected
        service.save(coordinator: coordinator)

        let restored = NavigationCoordinator()
        service.restore(into: restored, isAuthenticated: false)
        XCTAssertEqual(restored.selectedTab, .home, "Protected tab should fall back to .home")
    }

    func testClearWipesPersistedState() throws {
        let coordinator = NavigationCoordinator()
        coordinator.selectedTab = .search
        service.save(coordinator: coordinator)
        service.clear()

        let restored = NavigationCoordinator()
        service.restore(into: restored, isAuthenticated: true)
        // After clear, no data → coordinator keeps its default (.home)
        XCTAssertEqual(restored.selectedTab, .home)
    }

    func testRestoreWithNoDataIsNoOp() throws {
        // Nothing saved — restore must not crash or mutate the coordinator.
        let coordinator = NavigationCoordinator()
        coordinator.selectedTab = .inquiries
        service.restore(into: coordinator, isAuthenticated: true)
        // selectedTab should remain unchanged
        XCTAssertEqual(coordinator.selectedTab, .inquiries)
    }

    /// Regression test for the mirror-array desync bug.
    ///
    /// Before the fix, `restore()` set only the `NavigationPath` values but
    /// left the `*Routes` mirror arrays empty. On the next `save()` call the
    /// mirrors were read — returning `[]` — so the persisted stacks were
    /// silently discarded. This test catches that by doing a full
    /// save → restore → save → restore cycle and asserting that the stack
    /// depth survives unchanged.
    func testRoundTripPersistenceSurvivesDoubleRestoreCycle() throws {
        // Build an initial coordinator with a non-empty home stack.
        let coordinator1 = NavigationCoordinator()
        coordinator1.selectedTab = .home
        coordinator1.navigate(to: .listingDetail(id: "lst-A"))
        coordinator1.navigate(to: .listingGallery(id: "lst-A"))

        // First save.
        service.save(coordinator: coordinator1)

        // First restore.
        let coordinator2 = NavigationCoordinator()
        service.restore(into: coordinator2, isAuthenticated: true)
        XCTAssertEqual(coordinator2.routeMirror(for: .home).count, 2,
                       "First restore: home stack should have 2 routes")

        // Save AGAIN immediately after restore — this is where the bug bit.
        // If mirrors were empty, this would persist `[]` for the home stack.
        service.save(coordinator: coordinator2)

        // Second restore — into a third coordinator.
        let coordinator3 = NavigationCoordinator()
        service.restore(into: coordinator3, isAuthenticated: true)

        // The stack depth must survive the full cycle.
        XCTAssertEqual(coordinator3.routeMirror(for: .home).count, 2,
                       "Second restore: home stack depth must match original")
        XCTAssertEqual(coordinator3.selectedTab, .home)
    }

    func testLoginAndRegisterAreNotPersisted() throws {
        // .login and .register are ephemeral session routes that should never
        // be written to disk (stale auth routes across launches are misleading).
        XCTAssertNil(service.encodeRoute(.login),    ".login should not be encoded")
        XCTAssertNil(service.encodeRoute(.register), ".register should not be encoded")
    }
}

// MARK: - Performance Tests

/// Small performance baselines. They aren't asserted hard; the
/// `measure` block records timings that show up in Xcode's test report.
final class PerformanceTests: XCTestCase {

    func testConfigurationLookupPerformance() throws {
        // Looking up `apiBaseUrl` is on the hot path for every API call,
        // so it should never regress noticeably.
        measure {
            for _ in 0..<10_000 {
                _ = Configuration.shared.apiBaseUrl
            }
        }
    }
}
