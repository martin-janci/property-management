---
id: reality-mobile/favorites
name: Favorites (iOS SwiftUI)
product: reality-mobile
sitemapRefs:
  reality-web: reality-favorites
implementations:
  ios-swiftui:
    component: FavoritesView
    route: Tab.favorites / Route.favorites
    buildStatus: in-progress
    redesignStatus: not-started
    apiStatus: partial
endpoints:
  - favorites_list
  - favorites_remove
relatedScreens:
  - id: reality/favorites
    rel: web-counterpart
  - id: reality-mobile/listing-detail
    rel: child
  - id: reality-mobile/saved-searches
    rel: sibling
sharedComponents: []
diagrams: []
useCases:
  - UC-44
epics:
  - "82"
designSources: []
owner: reality-frontend
---

## Functionality Checklist

- [x] [m] Tab bar root screen requiring auth (Tab.favorites.requiresAuth = true)
- [x] [m] Load favorites via KMP `favoritesRepository.getFavorites()`
- [x] [m] Listing cards with Remove button
- [x] [m] Tap card → Route.listingDetail(id:)
- [x] [m] Remove favorite → `favoritesRepository.removeFavorite(id:)`
- [x] [m] Loading state (ProgressView)
- [x] [m] Empty state (heart icon + "no_favorites" + Browse button → Tab.search)
- [x] [m] Error state (warning icon + message + Retry)
- [ ] [m] Compare selected listings (Route.compareListings — button not present in FavoritesView)
- [ ] [m] Sort/filter tabs (All / Sale / Rent as in web version)
- [ ] [m] Share / Export list (web feature not implemented on iOS)

## States

- **Loading**: ProgressView centered in 200pt frame.
- **Error**: Warning icon + message + Retry button.
- **Empty**: Heart SF symbol + "no_favorites" text + "Browse properties" button switching to search tab.
- **Success**: List of FavoriteListingCard rows.

## Notes

### Broader context

Auth-gated tab (favorites, inquiries, account all require auth). When unauthenticated user taps the tab, `MainTabView.handleTabChange()` stores `pendingDestination = .favorites`, presents `LoginView` as sheet, reverts to previous tab. After successful SSO login, `pendingDestination` is consumed and navigation proceeds.

### Specific (recent)

- `FavoritesView` maps `KMP FavoritesResponse.favorites` → `[FavoriteEntry]` then to local `FavoriteItem` struct.
- Remove action calls `removeFavorite` and refreshes the list; no optimistic removal yet.

## Agent Log

- 2026-05-25 — agent: created screen map from audit of mobile-native/iosApp/iosApp/Features/Favorites/FavoritesView.swift (epic-82 story 82.4). Compare/sort gaps noted.
