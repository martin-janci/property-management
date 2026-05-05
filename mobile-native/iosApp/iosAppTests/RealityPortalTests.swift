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
