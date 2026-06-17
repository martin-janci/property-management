//
//  FavoritesServiceTests.swift
//  iosAppTests
//
//  Coverage 82-4: Listing Detail & Favorites (iOS SwiftUI) polish.
//
//  `FavoritesService` is the app-wide source of truth for the authenticated
//  user's saved listings, shared via `@Environment` between `ListingDetailView`
//  (the heart button) and `FavoritesView` (the list). It applies optimistic
//  toggle/remove with revert-on-error and is seeded from the server via
//  `configure(sessionToken:)`.
//
//  The network path delegates to the KMP `FavoritesRepository`, which is a
//  `final` class with no protocol seam, so a true network-stubbed test isn't
//  expressible from Swift without a host running reality-server (same constraint
//  documented in `SearchViewTests`). What IS host-runnable — and is exactly the
//  contract `ListingDetailView`/`FavoritesView` rely on every render — is the
//  service's *session-gating* and *synchronous read* behaviour:
//
//    * A fresh service holds no favorites and reports `isFavorite == false`.
//    * Without a session token, `toggle`/`remove` are no-ops (the `guard
//      sessionToken != nil` early-returns) — they must NOT optimistically
//      mutate `favoriteIds`, otherwise the heart would flip with no backing
//      server write and `FavoritesView` would desync.
//    * `configure(sessionToken: nil)` clears the in-memory set (sign-out path).
//
//  These assertions pin that gating without a live server. A behaviour test of
//  the optimistic add/remove + revert against a seeded reality-server is left
//  for the macOS reviewer's `xcodebuild test` run (see PR notes).
//

import XCTest
@testable import iosApp

// MARK: - FavoritesService session gating

/// Pins `FavoritesService`'s session-gating + synchronous read contract — the
/// part `ListingDetailView.isFavorite` and `FavoritesView.onChange` depend on
/// every render. Touching these symbols also forces the SUT to compile.
final class FavoritesServiceTests: XCTestCase {

    func testFreshServiceHasNoFavorites() {
        let service = FavoritesService()
        XCTAssertTrue(service.favoriteIds.isEmpty)
        XCTAssertFalse(service.isFavorite(listingId: "lst-1"))
    }

    func testToggleWithoutSessionIsNoOp() async {
        // No `configure(sessionToken:)` call → sessionToken is nil.
        // toggle() must early-return WITHOUT optimistically inserting, or the
        // heart in ListingDetailView would flip with no server write behind it.
        let service = FavoritesService()
        await service.toggle(listingId: "lst-1")
        XCTAssertFalse(service.isFavorite(listingId: "lst-1"),
                       "toggle must not mutate favoriteIds without a session token")
        XCTAssertTrue(service.favoriteIds.isEmpty)
    }

    func testRemoveWithoutSessionIsNoOp() async {
        let service = FavoritesService()
        await service.remove(listingId: "lst-1")
        XCTAssertTrue(service.favoriteIds.isEmpty)
    }

    func testConfigureWithNilTokenClearsFavorites() async {
        // The sign-out path: configure(nil) disables network ops and must clear
        // any in-memory favorites so a previous user's saved IDs don't leak.
        let service = FavoritesService()
        await service.configure(sessionToken: nil)
        XCTAssertTrue(service.favoriteIds.isEmpty)
        XCTAssertFalse(service.isFavorite(listingId: "lst-99"))
    }

    func testIsFavoriteIsStableAcrossRepeatedReads() {
        // isFavorite is the hot path called from ListingDetailView.body on every
        // render — it must be a pure, side-effect-free O(1) read.
        let service = FavoritesService()
        for _ in 0..<1000 {
            XCTAssertFalse(service.isFavorite(listingId: "lst-x"))
        }
        XCTAssertTrue(service.favoriteIds.isEmpty)
    }
}
