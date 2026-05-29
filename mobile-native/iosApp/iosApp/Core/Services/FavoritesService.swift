import Foundation
import Observation
import shared

/// App-wide favorites state manager for Reality Portal iOS app.
///
/// `FavoritesService` is the single source of truth for which listings the
/// authenticated user has saved. It is injected via `@Environment` so that
/// both `ListingDetailView` (heart button) and `FavoritesView` (list) stay
/// automatically in sync without requiring manual refresh calls.
///
/// ## Optimistic updates
///
/// `toggle(listingId:)` and `remove(listingId:)` apply changes immediately
/// to `favoriteIds` before the network call completes. If the server returns
/// an error the optimistic change is reverted.
///
/// ## Session lifecycle
///
/// Call `configure(sessionToken:)` once a session token is available (on
/// launch or after SSO login) to prime the in-memory set from the server.
/// The service is a no-op when called without an authenticated session.
///
/// Epic 82 - Story 82.4: Listing Detail and Favorites
@Observable
final class FavoritesService {

    // MARK: - Observable State

    /// Set of listing IDs the current user has favorited.
    ///
    /// Observers (e.g. `FavoritesView.onChange`) react to mutations here so
    /// the heart icon in `ListingDetailView` and the list in `FavoritesView`
    /// are always consistent.
    private(set) var favoriteIds: Set<String> = []

    // MARK: - Private

    private var sessionToken: String?
    private let configuration = Configuration.shared

    /// Creates a new `FavoritesRepository` scoped to the current session.
    private var repository: FavoritesRepository {
        DependencyContainer.shared.makeAuthenticatedFavoritesRepository(
            sessionToken: sessionToken
        )
    }

    // MARK: - Initialization

    init() {}

    // MARK: - Session Lifecycle

    /// Prime the service with a session token and fetch the user's current
    /// favorites from the server.
    ///
    /// Safe to call multiple times — subsequent calls re-fetch with the new
    /// token (e.g. after an SSO refresh). Calling with `nil` clears the
    /// in-memory set and disables all network operations.
    func configure(sessionToken: String?) async {
        self.sessionToken = sessionToken

        guard sessionToken != nil else {
            favoriteIds = []
            return
        }

        await fetchFavorites()
    }

    // MARK: - Read

    /// Returns `true` synchronously if `listingId` is in the local set.
    ///
    /// This is the hot path called from `ListingDetailView.body` on every
    /// render, so it must be O(1) and never block.
    func isFavorite(listingId: String) -> Bool {
        favoriteIds.contains(listingId)
    }

    // MARK: - Write

    /// Toggle the favorite state of a listing.
    ///
    /// Applies an optimistic update immediately, then calls the server.
    /// Reverts on failure.
    func toggle(listingId: String) async {
        guard sessionToken != nil else { return }

        let wasFavorite = favoriteIds.contains(listingId)

        // Optimistic update
        if wasFavorite {
            favoriteIds.remove(listingId)
        } else {
            favoriteIds.insert(listingId)
        }

        let succeeded: Bool
        if wasFavorite {
            let result = await repository.removeFavorite(listingId: listingId)
            succeeded = result.getOrNull() != nil
        } else {
            let result = await repository.addFavorite(listingId: listingId)
            succeeded = result.getOrNull() != nil
        }

        if !succeeded {
            // Revert the optimistic update
            if wasFavorite {
                favoriteIds.insert(listingId)
            } else {
                favoriteIds.remove(listingId)
            }

            #if DEBUG
            print("FavoritesService.toggle failed for \(listingId); optimistic update reverted")
            #endif
        }
    }

    /// Remove a listing from favorites.
    ///
    /// Applies an optimistic update immediately, then calls the server.
    /// Reverts on failure.
    func remove(listingId: String) async {
        guard sessionToken != nil else { return }
        guard favoriteIds.contains(listingId) else { return }

        // Optimistic update
        favoriteIds.remove(listingId)

        let result = await repository.removeFavorite(listingId: listingId)
        let succeeded = result.getOrNull() != nil

        if !succeeded {
            // Revert
            favoriteIds.insert(listingId)

            #if DEBUG
            print("FavoritesService.remove failed for \(listingId); optimistic update reverted")
            #endif
        }
    }

    // MARK: - Private helpers

    /// Fetches the full favorites list from the server and populates
    /// `favoriteIds`. Silently ignores errors — the UI will retry on
    /// explicit pull-to-refresh.
    private func fetchFavorites() async {
        let result = await repository.getFavorites()

        if let response = result.getOrNull() {
            favoriteIds = Set(response.favorites.map { $0.listingId })
        } else {
            #if DEBUG
            if let error = result.exceptionOrNull() {
                print("FavoritesService.fetchFavorites error: \(error.message ?? "unknown")")
            }
            #endif
        }
    }
}
