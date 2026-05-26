import SwiftUI
import shared

/// Favorites screen for Reality Portal iOS app.
///
/// Displays user's saved favorite listings.
///
/// Uses `FavoritesService` as the authoritative source so that a listing
/// favorited or un-favorited in `ListingDetailView` is immediately reflected
/// here without a pull-to-refresh. When the service's `favoriteIds` set
/// shrinks (e.g. the user un-hearts a listing in the detail view), the local
/// `favorites` array is pruned automatically via `.onChange`.
///
/// Epic 82 - Story 82.4: Listing Detail and Favorites
struct FavoritesView: View {
    @Environment(NavigationCoordinator.self) private var coordinator
    @Environment(AuthManager.self) private var authManager
    @Environment(FavoritesService.self) private var favoritesService

    @State private var favorites: [ListingPreview] = []
    @State private var isLoading = true
    @State private var errorMessage: String?

    private var favoritesRepository: FavoritesRepository {
        DependencyContainer.shared.makeAuthenticatedFavoritesRepository(
            sessionToken: authManager.getSessionToken()
        )
    }

    var body: some View {
        Group {
            if !authManager.isAuthenticated {
                notAuthenticatedView
            } else if isLoading {
                loadingView
            } else if favorites.isEmpty {
                emptyView
            } else {
                favoritesListView
            }
        }
        .navigationTitle("Favorites")
        .refreshable {
            await loadFavorites()
        }
        .task {
            if authManager.isAuthenticated {
                await loadFavorites()
            }
        }
        // Cross-view sync: when FavoritesService removes an ID (e.g. toggled off
        // from ListingDetailView), prune the local display list immediately.
        .onChange(of: favoritesService.favoriteIds) { _, newIds in
            let pruned = favorites.filter { newIds.contains($0.id) }
            if pruned.count != favorites.count {
                favorites = pruned
            }
        }
    }

    // MARK: - Subviews

    private var notAuthenticatedView: some View {
        VStack(spacing: 24) {
            Spacer()

            Image(systemName: "heart.fill")
                .font(.system(size: 64))
                .foregroundStyle(.secondary)

            Text("Sign in to see your favorites")
                .font(.headline)

            Text("Save properties you're interested in and access them from any device.")
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal, 32)

            Button {
                coordinator.navigate(to: .login)
            } label: {
                Text("Sign In")
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(Color.accentColor)
                    .foregroundStyle(.white)
                    .clipShape(RoundedRectangle(cornerRadius: 12))
            }
            .padding(.horizontal, 32)

            Spacer()
        }
    }

    private var loadingView: some View {
        VStack(spacing: 16) {
            Spacer()
            ProgressView()
            Text("Loading favorites...")
                .foregroundStyle(.secondary)
            Spacer()
        }
    }

    private var emptyView: some View {
        VStack(spacing: 24) {
            Spacer()

            Image(systemName: "heart")
                .font(.system(size: 64))
                .foregroundStyle(.secondary)

            Text("No favorites yet")
                .font(.headline)

            Text("Tap the heart icon on any listing to save it to your favorites.")
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal, 32)

            Button {
                coordinator.selectedTab = .search
            } label: {
                Text("Browse Listings")
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(Color.accentColor)
                    .foregroundStyle(.white)
                    .clipShape(RoundedRectangle(cornerRadius: 12))
            }
            .padding(.horizontal, 32)

            Spacer()
        }
    }

    private var favoritesListView: some View {
        ScrollView {
            LazyVStack(spacing: 12) {
                ForEach(favorites) { listing in
                    FavoriteListingCard(listing: listing) {
                        coordinator.navigate(to: .listingDetail(id: listing.id))
                    } onRemove: {
                        Task { await removeFavorite(listing.id) }
                    }
                }
            }
            .padding()
        }
    }

    // MARK: - Data Loading

    private func loadFavorites() async {
        isLoading = true
        errorMessage = nil

        // Load favorites from KMP shared module
        let result = await favoritesRepository.getFavorites()

        if let response = result.getOrNull() {
            favorites = response.favorites.compactMap { favoriteEntry in
                // Convert FavoriteEntry to ListingPreview
                guard let listing = favoriteEntry.listing else { return nil }
                return KMPBridge.toListingPreview(listing)
            }
        } else if let error = result.exceptionOrNull() {
            errorMessage = error.message ?? "Failed to load favorites"
            #if DEBUG
            print("Favorites error: \(errorMessage ?? "Unknown")")
            #endif
        }

        isLoading = false
    }

    private func removeFavorite(_ id: String) async {
        // Optimistic update on the display list.
        favorites.removeAll { $0.id == id }

        // Delegate to FavoritesService so that the heart icon in
        // ListingDetailView (and any other observer) updates in sync.
        await favoritesService.remove(listingId: id)

        // If the service reverted its optimistic update due to a server error,
        // the .onChange above will NOT add the item back to the display list
        // (we lost the full ListingPreview model). Reload to restore it.
        if favoritesService.isFavorite(listingId: id) {
            await loadFavorites()
        }
    }
}

// MARK: - Supporting Views

private struct FavoriteListingCard: View {
    let listing: ListingPreview
    let onTap: () -> Void
    let onRemove: () -> Void

    var body: some View {
        Button(action: onTap) {
            HStack(spacing: 12) {
                // Image placeholder or async image
                ZStack {
                    RoundedRectangle(cornerRadius: 8)
                        .fill(Color(.systemGray5))
                        .frame(width: 100, height: 80)

                    if let thumbnailUrl = listing.thumbnailUrl,
                       let url = URL(string: thumbnailUrl) {
                        AsyncImage(url: url) { phase in
                            switch phase {
                            case .success(let image):
                                image
                                    .resizable()
                                    .aspectRatio(contentMode: .fill)
                                    .frame(width: 100, height: 80)
                                    .clipShape(RoundedRectangle(cornerRadius: 8))
                            case .failure, .empty:
                                Image(systemName: "photo")
                                    .foregroundStyle(.secondary)
                            @unknown default:
                                Image(systemName: "photo")
                                    .foregroundStyle(.secondary)
                            }
                        }
                    } else {
                        Image(systemName: "photo")
                            .foregroundStyle(.secondary)
                    }
                }

                VStack(alignment: .leading, spacing: 4) {
                    Text(listing.formattedPrice)
                        .font(.headline)
                        .foregroundStyle(Color.accentColor)

                    Text(listing.title)
                        .font(.subheadline)
                        .lineLimit(1)
                        .foregroundStyle(.primary)

                    HStack(spacing: 4) {
                        Image(systemName: "location.fill")
                            .font(.caption)
                        Text(listing.location)
                            .font(.caption)
                    }
                    .foregroundStyle(.secondary)
                }

                Spacer()

                Button(action: onRemove) {
                    Image(systemName: "heart.fill")
                        .foregroundStyle(Color.pptDanger)
                }
            }
            .padding()
            .background(Color(.systemBackground))
            .clipShape(RoundedRectangle(cornerRadius: 12))
            .shadow(color: .black.opacity(0.05), radius: 4, y: 2)
        }
        .buttonStyle(.plain)
        .swipeActions(edge: .trailing, allowsFullSwipe: true) {
            Button(role: .destructive, action: onRemove) {
                Label("Remove", systemImage: "heart.slash")
            }
        }
    }
}

// MARK: - Preview

#Preview {
    NavigationStack {
        FavoritesView()
    }
    .environment(NavigationCoordinator())
    .environment(AuthManager())
    .environment(FavoritesService())
}
