import SwiftUI
import shared

/// Saved-searches screen for Reality Portal iOS (UC-45.2).
///
/// Brings the iOS feature surface up to parity with the Android
/// `SavedSearchesScreen` composable. Loads the signed-in user's saved
/// searches via `FavoritesRepository.getSavedSearches()` and exposes
/// alert toggle + delete actions that hit the existing reality-server
/// endpoints.
struct SavedSearchesView: View {
    @Environment(NavigationCoordinator.self) private var coordinator
    @Environment(AuthManager.self) private var authManager

    @State private var searches: [SavedSearch] = []
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
            } else if let errorMessage {
                errorView(errorMessage)
            } else if searches.isEmpty {
                emptyView
            } else {
                searchesList
            }
        }
        .navigationTitle("Saved searches")
        .refreshable { await loadSearches() }
        .task {
            if authManager.isAuthenticated {
                await loadSearches()
            } else {
                isLoading = false
            }
        }
    }

    private var loadingView: some View {
        VStack { ProgressView() }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private func errorView(_ message: String) -> some View {
        VStack(spacing: 12) {
            Image(systemName: "exclamationmark.triangle")
                .font(.largeTitle)
                .foregroundStyle(.secondary)
            Text("Couldn't load saved searches")
                .font(.headline)
            Text(message)
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal)
            Button("Retry") {
                Task { await loadSearches() }
            }
            .buttonStyle(.borderedProminent)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var emptyView: some View {
        VStack(spacing: 12) {
            Image(systemName: "magnifyingglass")
                .font(.largeTitle)
                .foregroundStyle(.secondary)
            Text("No saved searches")
                .font(.headline)
            Text("Save a search from the Search tab to get notified when matching listings appear.")
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var notAuthenticatedView: some View {
        VStack(spacing: 12) {
            Image(systemName: "person.crop.circle.badge.questionmark")
                .font(.largeTitle)
                .foregroundStyle(.secondary)
            Text("Sign in to view saved searches")
                .font(.headline)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var searchesList: some View {
        List {
            ForEach(searches, id: \.id) { search in
                row(for: search)
            }
            .onDelete(perform: delete)
        }
        .listStyle(.insetGrouped)
    }

    @ViewBuilder
    private func row(for search: SavedSearch) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Text(search.name)
                    .font(.headline)
                Spacer()
                Toggle(
                    "",
                    isOn: Binding(
                        get: { search.alertEnabled },
                        set: { newValue in
                            Task { await toggleAlerts(id: search.id, enabled: newValue) }
                        }
                    )
                )
                .labelsHidden()
            }
            if let summary = summary(for: search) {
                Text(summary)
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }
        }
        .contentShape(Rectangle())
        .onTapGesture {
            // No saved-search → search-with-filters payload yet; drop the
            // user into the plain Search tab so they can tweak filters.
            coordinator.navigate(to: .search)
        }
    }

    private func summary(for search: SavedSearch) -> String? {
        var parts: [String] = []
        if let q = search.query, !q.isEmpty { parts.append(q) }
        if let city = search.filters?.city, !city.isEmpty { parts.append(city) }
        if let minRooms = search.filters?.minRooms { parts.append("\(minRooms)+ rooms") }
        return parts.isEmpty ? nil : parts.joined(separator: " · ")
    }

    private func loadSearches() async {
        isLoading = true
        errorMessage = nil
        let result = await favoritesRepository.getSavedSearches()
        if let payload = result.getOrNull() {
            searches = payload.searches
        } else if let error = result.exceptionOrNull() {
            errorMessage = error.message ?? "Failed to load saved searches"
        }
        isLoading = false
    }

    private func toggleAlerts(id: String, enabled: Bool) async {
        let result = await favoritesRepository.toggleSearchAlert(
            searchId: id,
            enabled: enabled
        )
        if let updated = result.getOrNull() {
            if let index = searches.firstIndex(where: { $0.id == id }) {
                searches[index] = updated
            }
        } else if let error = result.exceptionOrNull() {
            errorMessage = error.message ?? "Failed to toggle alert"
        }
    }

    private func delete(at offsets: IndexSet) {
        let toDelete = offsets.map { searches[$0].id }
        for id in toDelete {
            Task {
                let result = await favoritesRepository.deleteSavedSearch(searchId: id)
                if result.exceptionOrNull() == nil {
                    searches.removeAll { $0.id == id }
                } else if let error = result.exceptionOrNull() {
                    errorMessage = error.message ?? "Failed to delete search"
                }
            }
        }
    }
}
