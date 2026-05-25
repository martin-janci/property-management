import Foundation
import SwiftUI

/// Persists and restores the navigation state (selected tab + per-tab route
/// stacks) across app launches, multitasking kills, and backgrounding.
///
/// **Storage format** (`UserDefaults` key `"navigation_state"`):
/// ```json
/// {
///   "selectedTab": "home",
///   "stacks": {
///     "home":     ["home", "listing:lst-1"],
///     "search":   [],
///     "favorites": [],
///     "inquiries": ["inquiries", "inquiry:inq-7"],
///     "account":  []
///   }
/// }
/// ```
///
/// Limitations (by design):
/// - `compareListings` stacks are **not** restored; they are ephemeral
///   sessions and restoring a stale comparison payload would confuse users.
/// - The saved state is wiped on logout (call ``clear()`` from `AuthManager`
///   on sign-out so protected routes don't leak after re-install or device
///   transfer).
///
/// Epic 82 - Story 82.2: Navigation and Routing
final class NavigationStateRestorationService {

    // MARK: - Coding

    /// Serialized form of the coordinator state.
    private struct PersistedState: Codable {
        var selectedTab: String
        var stacks: [String: [String]]
    }

    // MARK: - Persistence Keys

    private let userDefaultsKey = "navigation_state"
    private let defaults: UserDefaults

    init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
    }

    // MARK: - Public API

    /// Snapshot the current coordinator state and write it to `UserDefaults`.
    ///
    /// Call this from `scenePhase == .background` or `willResignActive`.
    func save(coordinator: NavigationCoordinator) {
        let state = PersistedState(
            selectedTab: coordinator.selectedTab.rawValue,
            stacks: [
                Tab.home.rawValue:      encode(routes: coordinator.routeMirror(for: .home)),
                Tab.search.rawValue:    encode(routes: coordinator.routeMirror(for: .search)),
                Tab.favorites.rawValue: encode(routes: coordinator.routeMirror(for: .favorites)),
                Tab.inquiries.rawValue: encode(routes: coordinator.routeMirror(for: .inquiries)),
                Tab.account.rawValue:   encode(routes: coordinator.routeMirror(for: .account)),
            ]
        )

        if let data = try? JSONEncoder().encode(state) {
            defaults.set(data, forKey: userDefaultsKey)
        }
    }

    /// Restore previously persisted state into the coordinator.
    ///
    /// Call this once at launch, after auth state is known. When
    /// `requiresAuthTabs` is provided (normally `[.favorites, .inquiries,
    /// .account]`) and the user is not authenticated, those tab stacks are
    /// silently dropped to avoid showing protected content.
    ///
    /// - Parameters:
    ///   - coordinator: The coordinator to restore into.
    ///   - isAuthenticated: Whether the user is currently signed in.
    func restore(into coordinator: NavigationCoordinator, isAuthenticated: Bool) {
        guard let data = defaults.data(forKey: userDefaultsKey),
              let state = try? JSONDecoder().decode(PersistedState.self, from: data)
        else { return }

        // Restore tab selection — fall back to .home if unknown.
        let restoredTab = Tab(rawValue: state.selectedTab) ?? .home

        // Don't land on a protected tab if the user is not authenticated.
        if restoredTab.requiresAuth && !isAuthenticated {
            coordinator.selectedTab = .home
        } else {
            coordinator.selectedTab = restoredTab
        }

        // Restore per-tab stacks.
        for tab in Tab.allCases {
            let encoded = state.stacks[tab.rawValue] ?? []
            // Skip protected stacks when not authenticated.
            let effectiveEncoded = (tab.requiresAuth && !isAuthenticated) ? [] : encoded
            let routes = effectiveEncoded.compactMap { decodeRoute($0) }
            coordinator.restoreStack(routes, for: tab)
        }
    }

    /// Wipe persisted state (call on logout).
    func clear() {
        defaults.removeObject(forKey: userDefaultsKey)
    }

    // MARK: - Encoding / Decoding

    /// Convert a ``Route`` array to an array of URL-safe strings for JSON storage.
    func encode(routes: [Route]) -> [String] {
        routes.compactMap { encodeRoute($0) }
    }

    func encodeRoute(_ route: Route) -> String? {
        switch route {
        case .home:                       return "home"
        case .featuredListings:           return "featuredListings"
        case .search:                     return "search"
        case .searchResults(let q, _):    return "searchResults:\(q)"
        case .listingDetail(let id):      return "listing:\(id)"
        case .listingGallery(let id):     return "listingGallery:\(id)"
        case .listingMap(let id):         return "listingMap:\(id)"
        case .favorites:                  return "favorites"
        case .inquiries:                  return "inquiries"
        case .inquiryDetail(let id):      return "inquiry:\(id)"
        case .newInquiry(let lid):        return "newInquiry:\(lid)"
        case .account:                    return "account"
        case .profile:                    return "profile"
        case .settings:                   return "settings"
        case .login:                      return nil   // ephemeral — not persisted
        case .register:                   return nil   // ephemeral — not persisted
        case .savedSearches:              return "savedSearches"
        case .compareListings:            return nil   // ephemeral — not persisted
        case .realtors:                   return "realtors"
        case .agencies:                   return "agencies"
        }
    }

    func decodeRoute(_ encoded: String) -> Route? {
        let parts = encoded.split(separator: ":", maxSplits: 1).map(String.init)
        let key = parts[0]
        let value = parts.count > 1 ? parts[1] : nil

        switch key {
        case "home":              return .home
        case "featuredListings":  return .featuredListings
        case "search":            return .search
        case "searchResults":     return .searchResults(query: value ?? "", filters: nil)
        case "listing":           return value.map { .listingDetail(id: $0) }
        case "listingGallery":    return value.map { .listingGallery(id: $0) }
        case "listingMap":        return value.map { .listingMap(id: $0) }
        case "favorites":         return .favorites
        case "inquiries":         return .inquiries
        case "inquiry":           return value.map { .inquiryDetail(id: $0) }
        case "newInquiry":        return value.map { .newInquiry(listingId: $0) }
        case "account":           return .account
        case "profile":           return .profile
        case "settings":          return .settings
        case "login":             return .login
        case "register":          return .register
        case "savedSearches":     return .savedSearches
        case "realtors":          return .realtors
        case "agencies":          return .agencies
        default:                  return nil
        }
    }

    private func makePath(from routes: [Route]) -> NavigationPath {
        var path = NavigationPath()
        for route in routes { path.append(route) }
        return path
    }
}
