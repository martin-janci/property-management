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
    buildStatus: shipped
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
  - Epic-82
designSources: []
owner: reality-frontend
---

## Functionality Checklist

- [x] [m] Tab bar root screen requiring auth (Tab.favorites.requiresAuth = true)
- [x] [m] Load favorites via KMP `favoritesRepository.getFavorites()`
- [x] [m] Listing cards with Remove button
- [x] [m] Tap card → Route.listingDetail(id:)
- [x] [m] Remove favorite → optimistic prune via `FavoritesService.remove(listingId:)` (reverts + reloads on server error)
- [x] [m] Cross-view sync — `.onChange(of: favoritesService.favoriteIds)` prunes the display list when a listing is un-hearted from `ListingDetailView`
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

- `FavoritesView` maps `KMP FavoritesResponse.favorites` → `[ListingPreview]` via `KMPBridge.toListingPreview` (each `FavoriteEntry.listing` is unwrapped; entries without a hydrated listing are dropped).
- Remove is now **optimistic** and cross-view-synced: the card's heart/swipe-to-remove drops the row from the local list immediately, delegates to `FavoritesService.remove(listingId:)` (app-wide `@Environment` source of truth shared with `ListingDetailView`), and reloads only if the service reverted its optimistic change after a server error. A `.onChange(of: favoritesService.favoriteIds)` observer prunes the list when a listing is un-hearted from the detail screen. The earlier "no optimistic removal yet" note is obsolete (delivered via `FavoritesService`, PR #641).

## Agent Log

- 2026-05-25 — agent: created screen map from audit of mobile-native/iosApp/iosApp/Features/Favorites/FavoritesView.swift (epic-82 story 82.4). Compare/sort gaps noted.
- 2026-05-28 — agent: fix(#581) added missing Agent Log entry that PR #554 omitted (CLAUDE.md screen-map Rule A.3); flagged dropped operationIds on home + saved-searches in Notes > Specific (recent) pending @ppt/sitemap extension.
- 2026-06-17 — agent: coverage 82-4 polish follow-through. Reconciled stale screen-map: flipped `buildStatus: in-progress → shipped` (FavoritesView ships via FavoritesService/PR #641), marked the optimistic-remove + cross-view-sync checklist items DONE, and corrected the obsolete "no optimistic removal yet" note + the FavoriteItem→ListingPreview mapping description. Added `FavoritesServiceTests.swift` (host-runnable session-gating assertions: fresh-empty, no-session toggle/remove no-op, configure(nil) clears). Still open: Compare-from-favorites button, Sort/filter tabs, Share/Export.
