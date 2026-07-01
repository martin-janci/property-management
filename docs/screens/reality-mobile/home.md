---
id: reality-mobile/home
name: Home (iOS SwiftUI)
product: reality-mobile
sitemapRefs:
  reality-web: reality-home
implementations:
  mobile-native:
    component: HomeScreen (Compose) — androidApp/.../ui/home/HomeScreen.kt
    route: Screen.Home ("home")
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: complete
  ios-swiftui:
    component: HomeView
    route: Tab.home / Route.home
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: complete
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
- [x] [m] Category chip filter navigation wired (pendingSearchFilters via NavigationCoordinator)
- [x] [m] SectionHeader "See all" links navigate to search tab

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
- ⚠️ **Sitemap gap (#581):** Featured carousel calls `GET /api/v1/listings/featured` and recent list calls `GET /api/v1/listings/recent`, but `@ppt/sitemap` has no `listings_featured` / `listings_recent` operationIds — only the more general `listings_search` is referenced above. Extend the sitemap and restore both specific endpoints here.

## Agent Log

- 2026-05-25 — agent: created screen map from audit of mobile-native/iosApp/iosApp/Features/Home/HomeView.swift (epic-82 story 82.3). NavigationCoordinator integration confirmed. Category chip actions unimplemented.
- 2026-05-25 — agent: wired category chips to pendingSearchFilters on NavigationCoordinator; SearchView.onAppear consumes and auto-searches. buildStatus → shipped.
- 2026-05-28 — agent: fix(#581) added missing Agent Log entry that PR #554 omitted (CLAUDE.md screen-map Rule A.3); flagged dropped operationIds on home + saved-searches in Notes > Specific (recent) pending @ppt/sitemap extension.
- 2026-07-01 — agent: coverage 82-3 finish-to-done (82-3-home-search-screens). Closed the tracked KMP gap by adding the missing `mobile-native` implementation block (Compose `HomeScreen` at `Screen.Home`, `buildStatus: shipped`) alongside the existing `ios-swiftui` block — the Android/KMP Home screen (`androidApp/.../ui/home/HomeScreen.kt`) is fully shipped (featured carousel, recent list, category grid, loading/error/empty states; no TODO/stub markers) but its build status was previously untracked here. iOS `HomeView.swift` re-audited: braces balance, category chips wired to `pendingSearchFilters`/`NavigationCoordinator` — intact. KMP/Swift not compile-runnable in this sandbox; CI / macOS reviewer is the authoritative gate.
- 2026-06-10 — agent: coverage 82-3 verify (verify-reality-mobile-search-ac-coverage). Home view (`HomeView.swift`) audited and intact — no AC-2/AC-4 logic lives here (Home defers all search to the Search tab via `pendingSearchFilters`). The story-82.3 AC-2 (debounce) / AC-4 (infinite scroll) / CoreLocation / FilterSheet ACs are verified on the **search** screen; see `reality-mobile/search.md` for the full findings, including a compile-blocking defect found in the sibling iOS `SearchView.swift`.
