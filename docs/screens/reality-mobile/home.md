---
id: reality-mobile/home
name: Home (iOS SwiftUI)
product: reality-mobile
sitemapRefs:
  reality-web: reality-home
implementations:
  ios-swiftui:
    component: HomeView
    route: Tab.home / Route.home
    buildStatus: in-progress
    redesignStatus: not-started
    apiStatus: partial
endpoints:
  - listings_search
relatedScreens:
  - id: reality/home
    rel: web-counterpart
  - id: reality-mobile/search
    rel: sibling
  - id: reality-mobile/listing-detail
    rel: child
sharedComponents:
  - FeaturedListingCard
  - ListingRowCard
  - CategoryChip
  - SectionHeader
diagrams: []
useCases:
  - UC-31
  - UC-44
epics:
  - "82"
designSources: []
owner: reality-frontend
---

## Functionality Checklist

- [x] [m] Tab bar root screen (Tab.home)
- [x] [m] Search bar button tap → switches to search tab
- [x] [m] Horizontal category chip strip (For sale / For rent / Apartments / Houses / Land)
- [x] [m] Featured listings horizontal carousel with FeaturedListingCard (4:3 photo, FEATURED badge, price, title, location, rooms/sqm)
- [x] [m] Recent listings vertical list with ListingRowCard (thumbnail, price, title, location)
- [x] [m] "View all" button → switches to search tab
- [x] [m] Pull-to-refresh
- [x] [m] Async data load via KMP ListingRepository.getFeaturedListings / getRecentListings
- [x] [m] Loading state (ProgressView + localized text)
- [x] [m] Error state (warning icon + message + Retry button)
- [x] [m] Empty state ("no listings available" message when both lists empty)
- [x] [m] Toolbar: authenticated → Inquiries / Favorites / Account icons; unauthenticated → Sign In button
- [ ] [m] Category chip filter navigation not wired (chips have placeholder closures)
- [ ] [m] No SectionHeader "See all" links wired to filtered search results

## States

- **Loading**: ProgressView centered in 200pt frame.
- **Error**: Warning icon + error message string + Retry button, 200pt frame.
- **Empty**: errorMessage set to "no_listings_available" string (reuses error view).
- **Success**: Featured carousel + recent list displayed.

## Notes

### Broader context

Root tab screen of Reality Portal iOS. Uses KMP `ListingRepository` (via `DependencyContainer.shared.listingRepository`) for data. Reads `shared.ListingSummary` and maps to Swift `ListingPreview` via `KMPBridge.toListingPreview()`. Component mirrors KmpHomeScreen on Android but with SwiftUI patterns.

### Specific (recent)

- Category chips currently have empty closures — navigation to filtered search not yet wired (post-epic-82 task).
- AsyncImage used for thumbnails with graceful fallback to `photo` SF symbol.
- `ListingPreview.sampleFeatured/sampleRecent` arrays are `#if DEBUG` guarded — not in production builds.

## Agent Log

- 2026-05-25 — agent: created screen map from audit of mobile-native/iosApp/iosApp/Features/Home/HomeView.swift (epic-82 story 82.3). NavigationCoordinator integration confirmed. Category chip actions unimplemented.
