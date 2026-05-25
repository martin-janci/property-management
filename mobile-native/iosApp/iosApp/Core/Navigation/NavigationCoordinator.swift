import Foundation
import Observation
import SwiftUI

// MARK: - Tab

/// Tab enumeration for the main tab bar.
///
/// Epic 82 - Story 82.2: Navigation and Routing
enum Tab: String, CaseIterable, Hashable {
    case home
    case search
    case favorites
    case inquiries
    case account

    /// SF Symbol icon name for the tab.
    var icon: String {
        switch self {
        case .home: return "house.fill"
        case .search: return "magnifyingglass"
        case .favorites: return "heart.fill"
        case .inquiries: return "envelope.fill"
        case .account: return "person.fill"
        }
    }

    /// Display title for the tab.
    var title: String {
        switch self {
        case .home: return "Home"
        case .search: return "Search"
        case .favorites: return "Favorites"
        case .inquiries: return "Inquiries"
        case .account: return "Account"
        }
    }

    /// Whether this tab requires authentication.
    var requiresAuth: Bool {
        switch self {
        case .favorites, .inquiries, .account:
            return true
        case .home, .search:
            return false
        }
    }
}

/// Central navigation coordinator for the app.
///
/// Manages tab selection and navigation paths for each tab.
///
/// Also maintains per-tab ``routeMirror`` arrays that shadow the opaque
/// ``NavigationPath`` values so ``NavigationStateRestorationService`` can
/// serialise the stacks without having to decode the path internals.
///
/// Epic 82 - Story 82.2: Navigation and Routing
@Observable
final class NavigationCoordinator {
    // MARK: - Properties

    /// Currently selected tab.
    var selectedTab: Tab = .home

    /// Navigation path for the Home tab.
    var homePath = NavigationPath()

    /// Navigation path for the Search tab.
    var searchPath = NavigationPath()

    /// Navigation path for the Favorites tab.
    var favoritesPath = NavigationPath()

    /// Navigation path for the Inquiries tab.
    var inquiriesPath = NavigationPath()

    /// Navigation path for the Account tab.
    var accountPath = NavigationPath()

    /// Badge count for unread inquiries.
    var inquiriesBadgeCount: Int = 0

    /// Intended destination after login (for auth-protected routes).
    var pendingDestination: Route?

    // MARK: - Route Mirrors (for state restoration serialisation)

    /// Mirrors of each tab's navigation stack as plain `Route` arrays.
    /// These shadow the opaque `NavigationPath` values so the restoration
    /// service can encode/decode them without relying on path internals.
    private(set) var homeRoutes: [Route] = []
    private(set) var searchRoutes: [Route] = []
    private(set) var favoritesRoutes: [Route] = []
    private(set) var inquiriesRoutes: [Route] = []
    private(set) var accountRoutes: [Route] = []

    /// Returns the route mirror for the given tab.
    func routeMirror(for tab: Tab) -> [Route] {
        switch tab {
        case .home:      return homeRoutes
        case .search:    return searchRoutes
        case .favorites: return favoritesRoutes
        case .inquiries: return inquiriesRoutes
        case .account:   return accountRoutes
        }
    }

    // MARK: - Navigation Methods

    /// Navigate to a specific route.
    /// - Parameter route: The destination route.
    func navigate(to route: Route) {
        switch route {
        case .home, .featuredListings:
            selectedTab = .home
            if case .featuredListings = route {
                homePath.append(route)
                homeRoutes.append(route)
            }

        case .search, .searchResults:
            selectedTab = .search
            if case .searchResults = route {
                searchPath.append(route)
                searchRoutes.append(route)
            }

        case .listingDetail, .listingGallery, .listingMap:
            // Listings can be accessed from multiple tabs
            // Append to current tab's path
            appendToCurrent(route)

        case .favorites:
            selectedTab = .favorites

        case .inquiries, .inquiryDetail, .newInquiry:
            selectedTab = .inquiries
            if case .inquiryDetail = route {
                inquiriesPath.append(route)
                inquiriesRoutes.append(route)
            } else if case .newInquiry = route {
                inquiriesPath.append(route)
                inquiriesRoutes.append(route)
            }

        case .account, .profile, .settings:
            selectedTab = .account
            if case .profile = route {
                accountPath.append(route)
                accountRoutes.append(route)
            } else if case .settings = route {
                accountPath.append(route)
                accountRoutes.append(route)
            }

        case .login, .register:
            accountPath.append(route)
            accountRoutes.append(route)

        case .savedSearches:
            // Saved searches are personal — show inside the account tab
            // so the navigation stack matches the rest of the user's
            // saved data (favorites is its own tab; inquiries too).
            selectedTab = .account
            accountPath.append(route)
            accountRoutes.append(route)

        case .compareListings:
            // Comparison is a cross-tab feature — opening it from
            // favorites/search/listing-detail should preserve the
            // user's current tab and stack onto it.
            appendToCurrent(route)

        case .realtors, .agencies:
            // Directory browsers belong to the search tab so the user
            // can pivot between "search by listing" and "search by
            // realtor/agency" without leaving the tab.
            selectedTab = .search
            searchPath.append(route)
            searchRoutes.append(route)
        }
    }

    /// Pop the current navigation stack.
    func pop() {
        switch selectedTab {
        case .home:
            if !homePath.isEmpty { homePath.removeLast(); if !homeRoutes.isEmpty { homeRoutes.removeLast() } }
        case .search:
            if !searchPath.isEmpty { searchPath.removeLast(); if !searchRoutes.isEmpty { searchRoutes.removeLast() } }
        case .favorites:
            if !favoritesPath.isEmpty { favoritesPath.removeLast(); if !favoritesRoutes.isEmpty { favoritesRoutes.removeLast() } }
        case .inquiries:
            if !inquiriesPath.isEmpty { inquiriesPath.removeLast(); if !inquiriesRoutes.isEmpty { inquiriesRoutes.removeLast() } }
        case .account:
            if !accountPath.isEmpty { accountPath.removeLast(); if !accountRoutes.isEmpty { accountRoutes.removeLast() } }
        }
    }

    /// Pop to root of current tab.
    func popToRoot() {
        switch selectedTab {
        case .home:
            homePath = NavigationPath(); homeRoutes = []
        case .search:
            searchPath = NavigationPath(); searchRoutes = []
        case .favorites:
            favoritesPath = NavigationPath(); favoritesRoutes = []
        case .inquiries:
            inquiriesPath = NavigationPath(); inquiriesRoutes = []
        case .account:
            accountPath = NavigationPath(); accountRoutes = []
        }
    }

    /// Reset all navigation state.
    func reset() {
        selectedTab = .home
        homePath = NavigationPath();      homeRoutes = []
        searchPath = NavigationPath();    searchRoutes = []
        favoritesPath = NavigationPath(); favoritesRoutes = []
        inquiriesPath = NavigationPath(); inquiriesRoutes = []
        accountPath = NavigationPath();   accountRoutes = []
        pendingDestination = nil
    }

    // MARK: - Path Bindings

    /// Get the navigation path for a specific tab.
    func path(for tab: Tab) -> NavigationPath {
        switch tab {
        case .home: return homePath
        case .search: return searchPath
        case .favorites: return favoritesPath
        case .inquiries: return inquiriesPath
        case .account: return accountPath
        }
    }

    /// Append a route to the current tab's path and its mirror array.
    private func appendToCurrent(_ route: Route) {
        switch selectedTab {
        case .home:      homePath.append(route);      homeRoutes.append(route)
        case .search:    searchPath.append(route);    searchRoutes.append(route)
        case .favorites: favoritesPath.append(route); favoritesRoutes.append(route)
        case .inquiries: inquiriesPath.append(route); inquiriesRoutes.append(route)
        case .account:   accountPath.append(route);   accountRoutes.append(route)
        }
    }
}

// MARK: - Deep Link Handling

extension NavigationCoordinator {
    /// Handle a deep link URL.
    ///
    /// SSO callbacks are **not** processed here — the app-level handler
    /// (``RealityPortalApp``) intercepts those first and calls
    /// ``AuthManager.loginWithSsoToken(_:)`` before optionally navigating
    /// to a ``pendingDestination``.
    ///
    /// - Parameter url: The incoming URL.
    /// - Returns: Whether the URL was handled and a navigation was triggered.
    @discardableResult
    func handleDeepLink(_ url: URL) -> Bool {
        let result = DeepLinkHandler().parse(url)
        switch result {
        case .route(let route):
            navigate(to: route)
            return true
        case .ssoCallback:
            // SSO is handled at the app level — signal "handled" so the
            // system doesn't log it as unrecognised, but don't navigate.
            return true
        case .unrecognized:
            return false
        }
    }
}
