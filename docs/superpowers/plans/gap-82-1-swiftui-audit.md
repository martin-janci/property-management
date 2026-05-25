# Gap 82-1: SwiftUI Reality Portal — Project Structure Audit

**Date:** 2026-05-25
**Task ID:** gap-82-1-swiftui-audit
**Specialist:** ios-swiftui
**Branch:** auto-impl/gap-82-1-swiftui-audit

---

## Executive Summary

Epic-82 SwiftUI iOS project (`mobile-native/iosApp/`) is substantially complete
for its 5 stories. All tab screens, navigation infrastructure, auth integration,
and KMP bridge are implemented. 9 route destination views remain as stub `Text`
placeholders in `MainTabView.destinationView()`. Screen maps created in
`docs/screens/reality-mobile/` for all 11 implemented screens.

---

## NavigationCoordinator Audit

**File:** `mobile-native/iosApp/iosApp/Core/Navigation/NavigationCoordinator.swift`

### Implementation

- Uses Swift 5.9 `@Observable` macro (not `ObservableObject`) — correct for iOS 17+.
- Injected via `.environment(navigationCoordinator)` at app root; consumed with
  `@Environment(NavigationCoordinator.self)` in all views — correct pattern.
- `Tab` enum (5 cases: home, search, favorites, inquiries, account) with:
  - `icon` (SF Symbol names)
  - `title` (raw strings — not localized; minor gap)
  - `requiresAuth` flag

### Per-Tab NavigationPath

One `NavigationPath` per tab (`homePath`, `searchPath`, `favoritesPath`,
`inquiriesPath`, `accountPath`). `navigate(to:)` switches tabs and appends to
the correct path. `pop()` / `popToRoot()` / `reset()` all correctly scoped.

### Cross-Tab Route Placement

| Route | Tab Placement | Rationale |
|---|---|---|
| `.listingDetail/.listingGallery/.listingMap` | Current tab (via `currentPath`) | Multi-context access |
| `.compareListings` | Current tab | Cross-tab feature |
| `.savedSearches` | Account tab | Personal data proximity |
| `.realtors/.agencies` | Search tab | Search-mode pivot |
| `.login/.register` | Account tab path | Auth stack |

Assessment: Placement rationale is sound and matches Android KMP `AppNavigation`.

### Deep Link Handling

Custom scheme `realityportal://` and universal links parsed in
`parseDeepLink(_:)`. Handles: `listing/<id>`, `search?q=`, `favorites`,
`inquiries/<id>`, `account`, and SSO callback (`sso?token=`). SSO callback
delegated to `RealityPortalApp.handleSsoCallback()`. Coverage is adequate for
epic-82 scope.

### Known Issue

`path(for:)` returns a copy of the `NavigationPath` value, not a binding. This
is correct for reading but the `@Bindable` workaround in `MainTabView` is
needed to get a `$coordinator.homePath` binding — which is correctly done.

---

## Auth Guard Implementation

**File:** `mobile-native/iosApp/iosApp/Core/AuthManager.swift`

### Token Storage

- Access token and refresh token stored in iOS Keychain via
  `SecItemAdd`/`SecItemCopyMatching` with `kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly`.
- Never `UserDefaults` — correct per specialist conventions.
- Keychain service key = bundle ID (`three.two.bit.ppt.reality`).

### SSO Flow

1. User taps SSO button in `LoginView` → opens `propertymanagement://sso?callback=realityportal://sso`.
2. PM app redirects back to `realityportal://sso?token=<token>`.
3. `RealityPortalApp.handleIncomingURL()` detects SSO host → `loginWithSsoToken(_:)`.
4. KMP `SsoService.validateAndLogin(ssoToken:)` validates token with reality-server.
5. Session token stored to Keychain; `currentUser` set from `SsoUserInfo`.
6. `pendingDestination` consumed and navigation proceeds.

### Session Restoration

`restoreSession()` called on `configureApp()`. Loads token from Keychain →
calls `SsoService.restoreSession(token:)` → if valid, calls `getSession()` to
reconstruct `currentUser`.

### Auth Guard (MainTabView)

`handleTabChange(from:to:)` checks `newTab.requiresAuth && !authManager.isAuthenticated`:
- Stores `coordinator.pendingDestination = routeForTab(newTab)`.
- Presents `LoginView` as sheet.
- Reverts `coordinator.selectedTab = oldTab`.

Assessment: Guard logic is correct and handles the SSO pending-destination
pattern properly.

### Gaps

- `AuthManager.refreshAccessToken()` stores a placeholder `User` from refresh
  response that lacks `avatarUrl` (maps `result.userId/email/name` directly).
  The `SsoService.refreshSession()` KMP method's return type may not expose all
  fields — follow-up needed.
- Email/password `login()` always throws `AuthError.ssoRequired`. The form is
  present as UI-only future extension.

---

## Project Setup Verification (Story 82.1)

### Bundle Configuration

- **Bundle ID:** `three.two.bit.ppt.reality` (all configurations)
  - Dev override: `three.two.bit.ppt.reality.dev`
- **Min OS:** iOS 15.0 (`IPHONEOS_DEPLOYMENT_TARGET = 15.0`)
- **Architectures:** `arm64` only

### xcconfig Structure

4 files covering all environments:
- `Base.xcconfig` — shared product/compiler settings
- `Development.xcconfig` — localhost:8081, logging, `.dev` suffix
- `Staging.xcconfig` — staging endpoints
- `Production.xcconfig` — production endpoints

### Info.plist

Tokens present: `API_BASE_URL`, `ENVIRONMENT`, `ENABLE_LOGGING` (all `$(VAR)`
expansions from xcconfig). URL scheme `realityportal://` registered.
Location, camera, photo library usage descriptions present.

### Localization

6 locales: `sk.lproj`, `cs.lproj`, `de.lproj`, `en.lproj`, `hu.lproj`, `pl.lproj`.
All string keys use `String(localized:)` in Swift — correct pattern.

### @main Entry Point

`RealityPortalApp` uses SwiftUI App protocol. Injects `NavigationCoordinator`
and `AuthManager` as `@State` (persists through scene lifecycle).

---

## Screen Coverage vs Epic-82 Scope

### Stories Mapped

| Story | Description | Screens | Status |
|---|---|---|---|
| 82.1 | SwiftUI Project Setup | App infra (not a screen) | Complete |
| 82.2 | Navigation and Routing | NavigationCoordinator | Complete |
| 82.3 | Home and Search Screens | HomeView, SearchView | In-progress |
| 82.4 | Listing Detail and Favorites | ListingDetailView, FavoritesView | In-progress |
| 82.5 | Inquiries and Account | InquiriesView, AccountView, LoginView | In-progress |

### Beyond Epic-82 (also implemented)

- SavedSearchesView (UC-45.2)
- CompareListingsView (UC-46.5)
- RealtorsView (UC-49.1)
- AgenciesView (UC-51.1)

### Stub Destinations (9 routes have no real view)

| Route | Stub | Priority |
|---|---|---|
| `listingGallery(id:)` | Text placeholder | High — linked from ListingDetailView |
| `listingMap(id:)` | Text placeholder | High — linked from ListingDetailView |
| `inquiryDetail(id:)` | Text placeholder | High — core inquiry flow |
| `newInquiry(listingId:)` | Text placeholder | High — core contact flow |
| `profile` | Text placeholder | Medium |
| `settings` | Text placeholder | Medium |
| `register` | Text placeholder | Low (SSO-only currently) |
| `featuredListings` | Text placeholder | Low |
| `searchResults(query:filters:)` | Text placeholder | Medium (used for deep links) |

---

## Architecture Findings

### Positive

1. `@Observable` used correctly throughout — no mixing with `ObservableObject`.
2. Keychain correctly used for all token storage.
3. KMP bridge (`KMPBridge` enum, `DependencyContainer`) cleanly separates KMP
   types from SwiftUI models.
4. `async/await` throughout, no completion-handler nesting.
5. `#if DEBUG` guards on sample data — production builds are clean.
6. `@discardableResult` on `handleDeepLink` — correct.

### Issues / Gaps

1. **Tab titles not localized** — `Tab.title` returns raw English strings. Should
   use `String(localized:)` or `LocalizedStringKey` for full i18n support.
2. **Category chip actions unimplemented** — `HomeView.categoryFilters` has empty
   closures on all chips. No navigation to filtered search happens on tap.
3. **No optimistic UI on favorites/inquiries** — state refreshed after server
   round-trip; visible lag on slow connections.
4. **`DependencyContainer` uses unauthenticated repositories by default** — lazy
   vars don't inject session tokens. Views use separate `make*` factory methods
   for authenticated operations, which is inconsistent.
5. **`NavigationCoordinator.path(for:)` returns value copy** — this is a
   documentation/naming issue only (it's used correctly); but could confuse
   future contributors.
6. **`SsoService` instantiated twice** — once in `AuthManager` (own field) and
   once in `DependencyContainer.shared.ssoService`. These are separate instances;
   session state may diverge.

---

## Screen Maps Created

All under `docs/screens/reality-mobile/`:

- `README.md` — index and stub-destination table
- `home.md`
- `search.md`
- `listing-detail.md`
- `favorites.md`
- `inquiries.md`
- `account.md`
- `auth-login.md`
- `saved-searches.md`
- `compare-listings.md`
- `realtors.md`
- `agencies.md`
