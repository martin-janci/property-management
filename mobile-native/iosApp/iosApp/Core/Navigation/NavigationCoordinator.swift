import Foundation
import Observation
import SwiftUI

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

    // MARK: - Navigation Methods

    /// Navigate to a specific route.
    /// - Parameter route: The destination route.
    func navigate(to route: Route) {
        switch route {
        case .home, .featuredListings:
            selectedTab = .home
            if case .featuredListings = route {
                homePath.append(route)
            }

        case .search, .searchResults:
            selectedTab = .search
            if case .searchResults = route {
                searchPath.append(route)
            }

        case .listingDetail, .listingGallery, .listingMap:
            // Listings can be accessed from multiple tabs
            // Append to current tab's path
            currentPath.append(route)

        case .favorites:
            selectedTab = .favorites

        case .inquiries, .inquiryDetail, .newInquiry:
            selectedTab = .inquiries
            if case .inquiryDetail = route {
                inquiriesPath.append(route)
            } else if case .newInquiry = route {
                inquiriesPath.append(route)
            }

        case .account, .profile, .settings:
            selectedTab = .account
            if case .profile = route {
                accountPath.append(route)
            } else if case .settings = route {
                accountPath.append(route)
            }

        case .login, .register:
            accountPath.append(route)

        case .savedSearches:
            // Saved searches are personal — show inside the account tab
            // so the navigation stack matches the rest of the user's
            // saved data (favorites is its own tab; inquiries too).
            selectedTab = .account
            accountPath.append(route)

        case .compareListings:
            // Comparison is a cross-tab feature — opening it from
            // favorites/search/listing-detail should preserve the
            // user's current tab and stack onto it.
            currentPath.append(route)

        case .realtors, .agencies:
            // Directory browsers belong to the search tab so the user
            // can pivot between "search by listing" and "search by
            // realtor/agency" without leaving the tab.
            selectedTab = .search
            searchPath.append(route)
        }
    }

    /// Pop the current navigation stack.
    func pop() {
        switch selectedTab {
        case .home:
            if !homePath.isEmpty { homePath.removeLast() }
        case .search:
            if !searchPath.isEmpty { searchPath.removeLast() }
        case .favorites:
            if !favoritesPath.isEmpty { favoritesPath.removeLast() }
        case .inquiries:
            if !inquiriesPath.isEmpty { inquiriesPath.removeLast() }
        case .account:
            if !accountPath.isEmpty { accountPath.removeLast() }
        }
    }

    /// Pop to root of current tab.
    func popToRoot() {
        switch selectedTab {
        case .home:
            homePath = NavigationPath()
        case .search:
            searchPath = NavigationPath()
        case .favorites:
            favoritesPath = NavigationPath()
        case .inquiries:
            inquiriesPath = NavigationPath()
        case .account:
            accountPath = NavigationPath()
        }
    }

    /// Reset all navigation state.
    func reset() {
        selectedTab = .home
        homePath = NavigationPath()
        searchPath = NavigationPath()
        favoritesPath = NavigationPath()
        inquiriesPath = NavigationPath()
        accountPath = NavigationPath()
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

    /// Current navigation path based on selected tab.
    private var currentPath: NavigationPath {
        get { path(for: selectedTab) }
        set {
            switch selectedTab {
            case .home: homePath = newValue
            case .search: searchPath = newValue
            case .favorites: favoritesPath = newValue
            case .inquiries: inquiriesPath = newValue
            case .account: accountPath = newValue
            }
        }
    }
}

// MARK: - Deep Link Handling

extension NavigationCoordinator {
    /// Handle a deep link URL.
    /// - Parameter url: The incoming URL.
    /// - Returns: Whether the URL was handled.
    @discardableResult
    func handleDeepLink(_ url: URL) -> Bool {
        guard let route = parseDeepLink(url) else {
            return false
        }

        navigate(to: route)
        return true
    }

    /// Parse a deep link URL into a route.
    /// - Parameter url: The URL to parse.
    /// - Returns: The corresponding route, or nil if invalid.
    private func parseDeepLink(_ url: URL) -> Route? {
        // Handle custom URL scheme: realityportal://listing/123
        // Handle universal link: https://reality.example.com/listing/123

        guard let components = URLComponents(url: url, resolvingAgainstBaseURL: true) else {
            return nil
        }

        let pathComponents = components.path.split(separator: "/").map(String.init)

        // Check for SSO callback
        if components.host == "sso",
           let token = components.queryItems?.first(where: { $0.name == "token" })?.value {
            // Handle SSO token - this should be processed by AuthManager
            _ = token
            return .account
        }

        // Handle listing deep link
        if let firstComponent = pathComponents.first {
            switch firstComponent {
            case "listing":
                if let id = pathComponents.dropFirst().first {
                    return .listingDetail(id: id)
                }
            case "search":
                let query = components.queryItems?.first(where: { $0.name == "q" })?.value ?? ""
                return .searchResults(query: query, filters: nil)
            case "favorites":
                return .favorites
            case "inquiries":
                if let id = pathComponents.dropFirst().first {
                    return .inquiryDetail(id: id)
                }
                return .inquiries
            case "account":
                return .account
            default:
                break
            }
        }

        return nil
    }
}
