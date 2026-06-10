import SwiftUI

/// Main tab view container for Reality Portal iOS app.
///
/// Epic 82 - Story 82.2: Navigation and Routing
struct MainTabView: View {
    @Environment(NavigationCoordinator.self) private var coordinator
    @Environment(AuthManager.self) private var authManager

    @State private var showLoginSheet = false

    var body: some View {
        @Bindable var coordinator = coordinator

        TabView(selection: $coordinator.selectedTab) {
            // Home Tab
            NavigationStack(path: $coordinator.homePath) {
                HomeView()
                    .navigationDestination(for: Route.self) { route in
                        destinationView(for: route)
                    }
            }
            .tabItem {
                Label(Tab.home.title, systemImage: Tab.home.icon)
            }
            .tag(Tab.home)

            // Search Tab
            NavigationStack(path: $coordinator.searchPath) {
                SearchView()
                    .navigationDestination(for: Route.self) { route in
                        destinationView(for: route)
                    }
            }
            .tabItem {
                Label(Tab.search.title, systemImage: Tab.search.icon)
            }
            .tag(Tab.search)

            // Favorites Tab
            NavigationStack(path: $coordinator.favoritesPath) {
                FavoritesView()
                    .navigationDestination(for: Route.self) { route in
                        destinationView(for: route)
                    }
            }
            .tabItem {
                Label(Tab.favorites.title, systemImage: Tab.favorites.icon)
            }
            .tag(Tab.favorites)

            // Inquiries Tab
            NavigationStack(path: $coordinator.inquiriesPath) {
                InquiriesView()
                    .navigationDestination(for: Route.self) { route in
                        destinationView(for: route)
                    }
            }
            .tabItem {
                Label(Tab.inquiries.title, systemImage: Tab.inquiries.icon)
            }
            .tag(Tab.inquiries)
            .badge(coordinator.inquiriesBadgeCount > 0 ? coordinator.inquiriesBadgeCount : 0)

            // Account Tab
            NavigationStack(path: $coordinator.accountPath) {
                AccountView()
                    .navigationDestination(for: Route.self) { route in
                        destinationView(for: route)
                    }
            }
            .tabItem {
                Label(Tab.account.title, systemImage: Tab.account.icon)
            }
            .tag(Tab.account)
        }
        .onChange(of: coordinator.selectedTab) { oldValue, newValue in
            handleTabChange(from: oldValue, to: newValue)
        }
        .sheet(isPresented: $showLoginSheet) {
            LoginView()
        }
    }

    // MARK: - Navigation Destinations

    @ViewBuilder
    private func destinationView(for route: Route) -> some View {
        switch route {
        case .home:
            HomeView()

        case .featuredListings:
            // Featured listings expanded view
            Text("Featured Listings")
                .navigationTitle("Featured")

        case .search:
            SearchView()

        case .searchResults(let query, let filters):
            // Search results view with query
            Text("Search Results for: \(query)")
                .navigationTitle("Results")

        case .listingDetail(let id):
            ListingDetailView(listingId: id)

        case .listingGallery(let id):
            // Photo gallery view
            Text("Gallery for listing: \(id)")
                .navigationTitle("Photos")

        case .listingMap(let id):
            ListingMapView(listingId: id)

        case .favorites:
            FavoritesView()

        case .inquiries:
            InquiriesView()

        case .inquiryDetail(let id):
            InquiryDetailView(inquiryId: id)

        case .newInquiry(let listingId):
            SendInquiryView(listingId: listingId)

        case .account:
            AccountView()

        case .profile:
            // Profile editing view
            Text("Edit Profile")
                .navigationTitle("Profile")

        case .settings:
            // Settings view
            Text("Settings")
                .navigationTitle("Settings")

        case .login:
            LoginView()

        case .register:
            // Registration view
            Text("Register")
                .navigationTitle("Create Account")

        case .savedSearches:
            SavedSearchesView()

        case .compareListings(let ids):
            CompareListingsView(listingIds: ids)

        case .realtors:
            RealtorsView()

        case .agencies:
            AgenciesView()
        }
    }

    // MARK: - Tab Change Handling

    private func handleTabChange(from oldTab: Tab, to newTab: Tab) {
        // Check if the new tab requires authentication
        if newTab.requiresAuth && !authManager.isAuthenticated {
            // Store the intended destination
            coordinator.pendingDestination = routeForTab(newTab)

            // Show login sheet
            showLoginSheet = true

            // Revert to previous tab
            coordinator.selectedTab = oldTab
        }
    }

    private func routeForTab(_ tab: Tab) -> Route {
        switch tab {
        case .home: return .home
        case .search: return .search
        case .favorites: return .favorites
        case .inquiries: return .inquiries
        case .account: return .account
        }
    }
}

// MARK: - Preview

#Preview {
    MainTabView()
        .environment(NavigationCoordinator())
        .environment(AuthManager())
}
