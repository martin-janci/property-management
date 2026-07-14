# Story 82.2: Navigation and Routing

Status: done

## Story

As a **Reality Portal iOS user**,
I want to **navigate between app screens using tab bar and stack navigation**,
So that **I can easily access all features of the app**.

## Acceptance Criteria

1. **AC-1: Tab Bar Navigation**
   - Given I open the Reality Portal app
   - When the main screen loads
   - Then I see a tab bar with 5 tabs (Home, Search, Favorites, Inquiries, Account)
   - And each tab shows an appropriate icon
   - And the active tab is highlighted

2. **AC-2: Stack Navigation**
   - Given I am on any tab
   - When I tap on a listing or action
   - Then I navigate to the detail screen via stack navigation
   - And a back button appears to return
   - And swipe-to-go-back gesture works

3. **AC-3: Deep Linking**
   - Given I receive a deep link to a listing
   - When I tap the link
   - Then the app opens to the specific listing detail
   - And I can navigate back to home

4. **AC-4: Navigation State Preservation**
   - Given I am viewing a listing detail
   - When I switch tabs and return
   - Then my previous navigation state is preserved
   - And I see the listing detail again

5. **AC-5: Guest vs Authenticated Navigation**
   - Given I am not logged in
   - When I try to access Favorites or Inquiries
   - Then I am prompted to log in
   - And after login I am redirected to intended destination

## Tasks / Subtasks

- [x] Task 1: Create Tab Bar Container (AC: 1, 4)
  - [x] 1.1 Create `/mobile-native/iosApp/iosApp/App/MainTabView.swift`
  - [x] 1.2 Define 5 tabs with icons and labels
  - [x] 1.3 Implement tab selection state
  - [x] 1.4 Style tab bar with app colors
  - [x] 1.5 Add badge for unread inquiries

- [x] Task 2: Implement Navigation Stacks (AC: 2)
  - [x] 2.1 Create NavigationStack for each tab
  - [x] 2.2 Define navigation destinations
  - [x] 2.3 Create `Router.swift` for centralized routing
  - [x] 2.4 Implement NavigationPath state management

- [x] Task 3: Create Navigation Coordinator (AC: 2, 3, 4)
  - [x] 3.1 Create `/mobile-native/iosApp/iosApp/Core/Navigation/NavigationCoordinator.swift`
  - [x] 3.2 Define Route enum with all destinations
  - [x] 3.3 Handle navigation state preservation
  - [x] 3.4 Implement programmatic navigation

- [x] Task 4: Implement Deep Linking (AC: 3)
  - [x] 4.1 Configure URL schemes in Info.plist
  - [x] 4.2 Add Universal Links entitlement
  - [x] 4.3 Parse incoming URLs in App delegate
  - [x] 4.4 Route to appropriate screen

- [x] Task 5: Add Authentication Guard (AC: 5)
  - [x] 5.1 Create `/mobile-native/iosApp/iosApp/Core/Navigation/AuthGuard.swift`
  - [x] 5.2 Check auth state before protected routes
  - [x] 5.3 Show login sheet when unauthenticated
  - [x] 5.4 Store intended destination for post-login redirect

## Dev Notes

### Architecture Requirements
- Use SwiftUI NavigationStack (iOS 16+)
- Centralized routing via NavigationCoordinator
- Preserve navigation state across tab switches
- Support deep links from push notifications

### Technical Specifications
- Tab icons: SF Symbols
- Navigation animation: Standard iOS
- Deep link scheme: `realityportal://`
- Universal link domain: `reality.example.com`

### Route Definitions
```swift
enum Route: Hashable {
    // Home
    case home
    case featuredListings

    // Search
    case search
    case searchResults(query: String, filters: SearchFilters?)

    // Listing
    case listingDetail(id: String)
    case listingGallery(id: String)
    case listingMap(id: String)

    // Favorites
    case favorites

    // Inquiries
    case inquiries
    case inquiryDetail(id: String)
    case newInquiry(listingId: String)

    // Account
    case account
    case profile
    case settings
    case login
}
```

### Tab Configuration
```swift
enum Tab: CaseIterable {
    case home
    case search
    case favorites
    case inquiries
    case account

    var icon: String {
        switch self {
        case .home: return "house.fill"
        case .search: return "magnifyingglass"
        case .favorites: return "heart.fill"
        case .inquiries: return "envelope.fill"
        case .account: return "person.fill"
        }
    }

    var title: String {
        switch self {
        case .home: return "Home"
        case .search: return "Search"
        case .favorites: return "Favorites"
        case .inquiries: return "Inquiries"
        case .account: return "Account"
        }
    }

    var requiresAuth: Bool {
        switch self {
        case .favorites, .inquiries, .account: return true
        default: return false
        }
    }
}
```

### Deep Link Handling
```swift
// URL scheme: realityportal://listing/123
// Universal link: https://reality.example.com/listing/123

func handleDeepLink(_ url: URL) -> Route? {
    guard let components = URLComponents(url: url, resolvingAgainstBaseURL: true) else {
        return nil
    }

    let pathComponents = components.path.split(separator: "/")

    if pathComponents.first == "listing",
       let id = pathComponents.dropFirst().first {
        return .listingDetail(id: String(id))
    }

    return nil
}
```

### File List (to create)

**Create:**
- `/mobile-native/iosApp/iosApp/App/MainTabView.swift`
- `/mobile-native/iosApp/iosApp/Core/Navigation/NavigationCoordinator.swift`
- `/mobile-native/iosApp/iosApp/Core/Navigation/Route.swift`
- `/mobile-native/iosApp/iosApp/Core/Navigation/AuthGuard.swift`
- `/mobile-native/iosApp/iosApp/Core/Navigation/DeepLinkHandler.swift`

**Modify:**
- `/mobile-native/iosApp/iosApp/App/RealityPortalApp.swift` - Add navigation setup
- `/mobile-native/iosApp/iosApp/Resources/Info.plist` - Add URL schemes

### Navigation State Pattern
```swift
@Observable
class NavigationCoordinator {
    var selectedTab: Tab = .home
    var homePath = NavigationPath()
    var searchPath = NavigationPath()
    var favoritesPath = NavigationPath()
    var inquiriesPath = NavigationPath()
    var accountPath = NavigationPath()

    func navigate(to route: Route) {
        // Determine which tab and update path
    }

    func currentPath(for tab: Tab) -> Binding<NavigationPath> {
        // Return binding to appropriate path
    }
}
```

### Dependencies
- Story 82.1 (SwiftUI Project Setup) - Base project structure

### References
- [Reference: mobile-native/androidApp/src/main/java/...navigation/]
- [iOS Human Interface Guidelines: Tab Bars]
