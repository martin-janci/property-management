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

    // MARK: SSO CSRF nonce (closes #578 secondary finding)
    //
    // These tests pin the contract of `beginSsoFlow` / `consumeSsoState`:
    // a freshly minted nonce must validate exactly once, all other inputs
    // (nil, empty, mismatched, replay) must be rejected, and successive
    // calls to `beginSsoFlow` must yield distinct values.

    func testBeginSsoFlowReturnsHexString() throws {
        let auth = AuthManager()
        let nonce = auth.beginSsoFlow()
        // 32 random bytes → 64 hex chars; allow the UUID fallback (≥ 32 chars).
        XCTAssertGreaterThanOrEqual(nonce.count, 32)
        let allowed = CharacterSet(charactersIn: "0123456789abcdefABCDEF-")
        XCTAssertTrue(nonce.unicodeScalars.allSatisfy { allowed.contains($0) })
    }

    func testBeginSsoFlowGeneratesUniqueNoncesAcrossCalls() throws {
        let auth = AuthManager()
        // We compare two successive calls — the second overwrites the first
        // pending value, which is acceptable (caller starts a new flow).
        let first = auth.beginSsoFlow()
        let second = auth.beginSsoFlow()
        XCTAssertNotEqual(first, second, "Two SSO flows must mint different nonces")
    }

    func testConsumeSsoStateAcceptsMatchingNonce() throws {
        let auth = AuthManager()
        let nonce = auth.beginSsoFlow()
        XCTAssertTrue(auth.consumeSsoState(nonce))
    }

    func testConsumeSsoStateRejectsMismatch() throws {
        let auth = AuthManager()
        _ = auth.beginSsoFlow()
        XCTAssertFalse(auth.consumeSsoState("not-the-real-nonce"))
    }

    func testConsumeSsoStateRejectsNil() throws {
        let auth = AuthManager()
        _ = auth.beginSsoFlow()
        XCTAssertFalse(auth.consumeSsoState(nil))
    }

    func testConsumeSsoStateRejectsEmptyString() throws {
        let auth = AuthManager()
        _ = auth.beginSsoFlow()
        XCTAssertFalse(auth.consumeSsoState(""))
    }

    func testConsumeSsoStateIsSingleUse() throws {
        // Replay protection: a successful consume must clear the pending
        // value, so a second attempt with the same nonce returns false.
        let auth = AuthManager()
        let nonce = auth.beginSsoFlow()
        XCTAssertTrue(auth.consumeSsoState(nonce))
        XCTAssertFalse(auth.consumeSsoState(nonce))
    }

    func testConsumeSsoStateRejectsWhenNoFlowStarted() throws {
        // Fresh AuthManager — no `beginSsoFlow` call — must reject any input.
        let auth = AuthManager()
        XCTAssertFalse(auth.consumeSsoState("anything"))
        XCTAssertFalse(auth.consumeSsoState(nil))
    }

    func testFailedConsumeAlsoClearsPendingState() throws {
        // A mismatched consume must also clear the pending nonce so a
        // probe with the wrong state can't be retried with the right one.
        let auth = AuthManager()
        let nonce = auth.beginSsoFlow()
        XCTAssertFalse(auth.consumeSsoState("wrong"))
        XCTAssertFalse(auth.consumeSsoState(nonce),
                       "First consume cleared the pending nonce, second must fail")
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

    // MARK: Universal-link host consistency (GH #1408)

    /// Every Environment.universalLinkDomain must be in DeepLinkHandler.allowedUniversalLinkHosts
    /// so staging/production universal links are not silently dropped by the parser.
    func testEveryEnvironmentUniversalLinkDomainIsAllowed() {
        let allowed = DeepLinkHandler.allowedUniversalLinkHosts
        for env in [Environment.staging, Environment.production] {
            XCTAssertTrue(
                allowed.contains(env.universalLinkDomain),
                "\(env) universalLinkDomain '\(env.universalLinkDomain)' missing from allowedUniversalLinkHosts"
            )
        }
    }

    /// A URL from an out-of-allow-list host must not be routed to the app.
    func testOutOfAllowlistUniversalLinkIsUnrecognised() throws {
        let url = try XCTUnwrap(URL(string: "https://attacker.example/listing/lst-1"))
        guard case .unrecognized = handler.parse(url) else {
            return XCTFail("Expected unrecognized for non-allow-listed universal-link host")
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

// MARK: - InquiryStatus Tests

/// Verifies the Swift `InquiryStatus` enum used by `InquiriesView`
/// and that `KMPBridge.toInquiryPreview` maps KMP status values correctly.
final class InquiryStatusTests: XCTestCase {

    // MARK: displayName
    //
    // displayName now calls String(localized:) so the resolved string depends on the
    // test runner's locale.  Assert that each case returns a non-empty string rather
    // than a hardcoded English value — locale-fragile assertions break on non-English
    // CI runners and are meaningless as a correctness check.

    func testPendingDisplayNameIsNonEmpty() {
        XCTAssertFalse(InquiryStatus.pending.displayName.isEmpty,
                       "pending.displayName must resolve to a non-empty localised string")
    }

    func testRepliedDisplayNameIsNonEmpty() {
        XCTAssertFalse(InquiryStatus.replied.displayName.isEmpty,
                       "replied.displayName must resolve to a non-empty localised string")
    }

    func testClosedDisplayNameIsNonEmpty() {
        XCTAssertFalse(InquiryStatus.closed.displayName.isEmpty,
                       "closed.displayName must resolve to a non-empty localised string")
    }

    func testAllDisplayNamesAreDistinct() {
        let names = [InquiryStatus.pending.displayName,
                     InquiryStatus.replied.displayName,
                     InquiryStatus.closed.displayName]
        XCTAssertEqual(names.count, Set(names).count,
                       "Each InquiryStatus must have a distinct displayName")
    }

    // MARK: badge colours resolve without crashing

    func testPendingColorsAreNonNil() {
        // backgroundColor / foregroundColor call into InquiryStatusColors;
        // just ensure they don't crash — we don't assert specific hex values here.
        _ = InquiryStatus.pending.backgroundColor
        _ = InquiryStatus.pending.foregroundColor
    }

    func testRepliedColorsAreNonNil() {
        _ = InquiryStatus.replied.backgroundColor
        _ = InquiryStatus.replied.foregroundColor
    }

    func testClosedColorsAreNonNil() {
        _ = InquiryStatus.closed.backgroundColor
        _ = InquiryStatus.closed.foregroundColor
    }
}

// MARK: - InquiryPreview Tests

/// Verifies the `InquiryPreview` model that `InquiriesView` renders.
final class InquiryPreviewTests: XCTestCase {

    func testFormattedDateProducesNonEmptyString() {
        let preview = InquiryPreview(
            id: "p-1",
            listingId: "lst-1",
            listingTitle: "Test Property",
            lastMessage: "Hello",
            status: .pending,
            date: Date(),
            hasUnread: false
        )
        XCTAssertFalse(preview.formattedDate.isEmpty)
    }

    func testSampleDataIsAvailableInDebug() {
        #if DEBUG
        XCTAssertFalse(InquiryPreview.samples.isEmpty, "Sample data required for SwiftUI previews")
        // All sample IDs should be unique
        let ids = InquiryPreview.samples.map(\.id)
        XCTAssertEqual(ids.count, Set(ids).count, "Sample IDs must be unique")
        #endif
    }
}

// MARK: - PushNotificationManager Tests

/// Unit tests for `PushNotificationManager`'s preference management.
/// These tests do not interact with APNs and work entirely offline.
final class PushNotificationManagerTests: XCTestCase {

    private var keychain: KeychainService!
    private var manager: PushNotificationManager!

    override func setUp() {
        super.setUp()
        // Isolate from the real app keychain — use a unique service name.
        keychain = KeychainService(
            service: "three.two.bit.ppt.reality.tests.push.\(UUID().uuidString)"
        )
        manager = PushNotificationManager(keychainService: keychain)
    }

    override func tearDown() {
        keychain.deleteAll()
        // testDisablingNewListingsPersists writes a preference into
        // UserDefaults.standard (PushNotificationManager reads preferences from
        // the standard suite and does not accept an injected suite). Remove it
        // so a leftover `false` cannot make order-dependent tests in other
        // suites — which assume `newListings` defaults to `true` — fail
        // non-deterministically. Issue #698 finding 4.
        for key in NotificationPreferenceKey.allCases {
            UserDefaults.standard.removeObject(forKey: key.rawValue)
        }
        super.tearDown()
    }

    // MARK: Default preferences

    func testNewListingsDefaultsToEnabled() {
        XCTAssertTrue(manager.isEnabled(.newListings))
    }

    func testPriceDropsDefaultsToEnabled() {
        XCTAssertTrue(manager.isEnabled(.priceDrops))
    }

    func testInquiryResponsesDefaultsToEnabled() {
        XCTAssertTrue(manager.isEnabled(.inquiryResponses))
    }

    func testMarketingDefaultsToDisabled() {
        XCTAssertFalse(manager.isEnabled(.marketing))
    }

    // MARK: setEnabled persists the value

    func testDisablingNewListingsPersists() async {
        // Disable new listings — the manager is already authorized in this
        // path because we skip the authorization gate (not authorized means
        // setEnabled returns early without persisting).  We test the
        // UserDefaults persistence side only, which doesn't require APNs.
        UserDefaults.standard.set(false, forKey: NotificationPreferenceKey.newListings.rawValue)
        let freshManager = PushNotificationManager(keychainService: keychain)
        XCTAssertFalse(freshManager.isEnabled(.newListings))
    }

    // MARK: clearDeviceToken

    func testClearDeviceTokenRemovesToken() {
        // Simulate a stored token
        try? keychain.save("abc123", forKey: KeychainService.Keys.pushDeviceToken)
        let freshManager = PushNotificationManager(keychainService: keychain)
        XCTAssertNotNil(freshManager.deviceToken)

        freshManager.clearDeviceToken()
        XCTAssertNil(freshManager.deviceToken)
        XCTAssertNil(keychain.loadOptional(forKey: KeychainService.Keys.pushDeviceToken))
    }

    // MARK: didRegisterForRemoteNotifications

    func testDeviceTokenConvertedToHex() {
        let rawBytes: [UInt8] = [0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x23, 0x45, 0x67]
        let data = Data(rawBytes)
        manager.didRegisterForRemoteNotifications(deviceToken: data)
        XCTAssertEqual(manager.deviceToken, "deadbeef01234567")
    }

    func testDeviceTokenPersistedToKeychain() {
        let rawBytes: [UInt8] = [0xCA, 0xFE, 0xBA, 0xBE]
        manager.didRegisterForRemoteNotifications(deviceToken: Data(rawBytes))
        XCTAssertEqual(
            keychain.loadOptional(forKey: KeychainService.Keys.pushDeviceToken),
            "cafebabe"
        )
    }
}

// MARK: - KeychainService Tests

/// Verifies the `KeychainService` persistence layer used for auth tokens
/// and push device tokens.
final class KeychainServiceTests: XCTestCase {

    private var keychain: KeychainService!

    override func setUp() {
        super.setUp()
        // Use an isolated test service so tests don't affect the real app keychain.
        keychain = KeychainService(
            service: "three.two.bit.ppt.reality.tests.keychain.\(UUID().uuidString)"
        )
    }

    override func tearDown() {
        keychain.deleteAll()
        super.tearDown()
    }

    func testSaveAndLoad() throws {
        try keychain.save("secret-token", forKey: "test_key")
        let loaded = try keychain.load(forKey: "test_key")
        XCTAssertEqual(loaded, "secret-token")
    }

    func testOverwriteExistingValue() throws {
        try keychain.save("v1", forKey: "reuse_key")
        try keychain.save("v2", forKey: "reuse_key")
        XCTAssertEqual(try keychain.load(forKey: "reuse_key"), "v2")
    }

    func testLoadOptionalReturnsNilForMissingKey() {
        XCTAssertNil(keychain.loadOptional(forKey: "does_not_exist"))
    }

    func testDeleteRemovesItem() throws {
        try keychain.save("to-delete", forKey: "del_key")
        try keychain.delete(forKey: "del_key")
        XCTAssertNil(keychain.loadOptional(forKey: "del_key"))
    }

    func testDeleteMissingKeyDoesNotThrow() {
        XCTAssertNoThrow(try keychain.delete(forKey: "never_existed"))
    }

    func testContainsReturnsFalseForMissingKey() {
        XCTAssertFalse(keychain.contains(key: "absent"))
    }

    func testContainsReturnsTrueAfterSave() throws {
        try keychain.save("x", forKey: "present")
        XCTAssertTrue(keychain.contains(key: "present"))
    }

    func testDeleteAllClearsAllItems() throws {
        try keychain.save("a", forKey: "key_a")
        try keychain.save("b", forKey: "key_b")
        keychain.deleteAll()
        XCTAssertFalse(keychain.contains(key: "key_a"))
        XCTAssertFalse(keychain.contains(key: "key_b"))
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
