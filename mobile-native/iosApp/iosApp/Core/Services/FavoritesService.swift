import Foundation
import Observation
import shared

/// App-wide favorites service for Reality Portal iOS app.
///
/// Owns the authoritative set of favorited listing IDs and surfaces it as
/// `favoriteIds` — an `@Observable` property that SwiftUI views can observe
/// directly. This means:
///
/// - `ListingDetailView` can derive `isFavorite` from `favoriteIds` without
///   a separate API call.
/// - `FavoritesView` can prune its display list immediately when a listing is
///   un-hearted from the detail view (via `.onChange(of: favoriteIds)`).
///
/// The service applies **optimistic updates**: the in-memory `favoriteIds` set
/// is mutated before the network call returns. If the server rejects the
/// request the set is rolled back, keeping the UI consistent.
///
/// Auth: the service is configured (once) with a session token via
/// `configure(sessionToken:)`. If no token is available,  toggle/remove
/// calls silently no-op so unauthenticated users see a clean empty state.
///
/// Epic 82 - Story 82.4: Listing Detail and Favorites
@Observable
final class FavoritesService {
    // MARK: - Public State

    /// The set of listing IDs that the current user has favorited.
    ///
    /// Observed by `ListingDetailView` (to derive `isFavorite`) and
    /// `FavoritesView` (to prune its display list on external changes).
    private(set) var favoriteIds: Set<String> = []

    // MARK: - Private

    private var sessionToken: String?
    private let configuration = Configuration.shared

    private var repository: FavoritesRepository? {
        guard let token = sessionToken else { return nil }
        return FavoritesRepository(
            baseUrl: configuration.apiBaseUrl,
            sessionToken: token,
            client: HttpClientProvider.shared.client
        )
    }

    // MARK: - Init

    init() {}

    // MARK: - Configuration

    /// Configure the service with the current session token and seed
    /// `favoriteIds` from the server.
    ///
    /// Call this after a successful login or SSO callback, and also on app
    /// launch (after `AuthManager.restoreSession()`) so favorites are
    /// available without an extra pull-to-refresh.
    func configure(sessionToken: String?) async {
        self.sessionToken = sessionToken
        guard sessionToken != nil else {
            favoriteIds = []
            return
        }
        await seedFromServer()
    }

    // MARK: - Query

    /// Whether `listingId` is in the current user's favorites.
    ///
    /// O(1) set lookup. This is called on every render of `ListingDetailView`'s
    /// favorite button so it must be cheap.
    func isFavorite(listingId: String) -> Bool {
        favoriteIds.contains(listingId)
    }

    // MARK: - Mutations

    /// Toggle the favorite state for `listingId`.
    ///
    /// Applies an optimistic update immediately, then confirms with the server.
    /// Rolls back on failure.
    func toggle(listingId: String) async {
        if isFavorite(listingId: listingId) {
            await remove(listingId: listingId)
        } else {
            await add(listingId: listingId)
        }
    }

    /// Add `listingId` to favorites.
    ///
    /// Optimistic: inserts into `favoriteIds` before the network call.
    func add(listingId: String) async {
        guard let repo = repository else { return }

        // Optimistic insert
        favoriteIds.insert(listingId)

        let result = await repo.addFavorite(listingId: listingId)
        if result.getOrNull() == nil {
            // Roll back on failure
            favoriteIds.remove(listingId)
            #if DEBUG
            let err = result.exceptionOrNull()
            print("FavoritesService: add(\(listingId)) failed — \(err?.message ?? "unknown")")
            #endif
        }
    }

    /// Remove `listingId` from favorites.
    ///
    /// Optimistic: removes from `favoriteIds` before the network call.
    func remove(listingId: String) async {
        guard let repo = repository else { return }

        // Optimistic removal
        favoriteIds.remove(listingId)

        let result = await repo.removeFavorite(listingId: listingId)
        if result.getOrNull() == nil {
            // Roll back on failure
            favoriteIds.insert(listingId)
            #if DEBUG
            let err = result.exceptionOrNull()
            print("FavoritesService: remove(\(listingId)) failed — \(err?.message ?? "unknown")")
            #endif
        }
    }

    // MARK: - Private Helpers

    /// Fetch the full favorites list from the server and populate `favoriteIds`.
    private func seedFromServer() async {
        guard let repo = repository else { return }

        let result = await repo.getFavorites()
        if let response = result.getOrNull() {
            favoriteIds = Set(response.favorites.map { $0.listingId })
        } else {
            #if DEBUG
            let err = result.exceptionOrNull()
            print("FavoritesService: seedFromServer failed — \(err?.message ?? "unknown")")
            #endif
        }
    }
}
