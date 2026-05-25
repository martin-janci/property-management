---
id: reality-mobile/search
name: Search (iOS SwiftUI)
product: reality
sitemapRefs:
  reality-web: reality-listings
implementations:
  ios-swiftui:
    component: SearchView
    route: Tab.search / Route.search
    buildStatus: in-progress
    redesignStatus: not-started
    apiStatus: partial
endpoints:
  - POST /api/v1/listings/search
relatedScreens:
  - id: reality/listings
    rel: web-counterpart
  - id: reality-mobile/home
    rel: sibling
  - id: reality-mobile/listing-detail
    rel: child
  - id: reality-mobile/realtors
    rel: sibling
  - id: reality-mobile/agencies
    rel: sibling
sharedComponents:
  - FilterSheet
  - ListingRowCard
diagrams: []
useCases:
  - UC-31
  - UC-45
epics:
  - "82"
designSources: []
owner: reality-frontend
---

## Functionality Checklist

- [x] [m] Search bar with text input (300ms debounce on change)
- [x] [m] Filter bar (price range, property types, rooms, radius, location via SearchFilters)
- [x] [m] "Show Filters" sheet presentation (FilterSheet)
- [x] [m] Paginated search results grid
- [x] [m] Loading state (ProgressView in 300pt frame)
- [x] [m] Empty results state ("no_results" + "try_different_search")
- [x] [m] Search prompt state when no query entered
- [x] [m] Tap card → navigate to Route.listingDetail(id:)
- [ ] [m] Horizontal scroll categories on search screen (mentioned in comments, not implemented)
- [ ] [m] Sort order selector (not yet implemented)
- [ ] [m] Map view toggle (Route.listingMap mentioned in Route enum but SearchView has no map toggle)

## States

- **Prompt**: centered "search_prompt" icon + message displayed before first search.
- **Loading**: ProgressView while results.isEmpty and isLoading.
- **Empty results**: magnifying-glass icon + "no_results" heading + "try_different_search" body.
- **Success**: paginated LazyVGrid of ListingRowCards.

## Notes

### Broader context

Search tab root. Calls `listingRepository.searchListings(request:)` via KMP. Pagination with `currentPage`/`totalPages`. Filter sheet modal (`FilterSheet`) collects `SearchFilters` struct and triggers new search.

### Specific (recent)

- Debounce implemented with `Task.sleep(nanoseconds: 300_000_000)` — checks if `searchText` still matches after sleep.
- `FilterSheet` is defined inline in SearchView.swift — not a separate file.
- `Route.searchResults(query:filters:)` is registered in NavigationCoordinator but SearchView itself is a tab root, not accessed via navigation push.

## Agent Log

- 2026-05-25 — agent: created screen map from audit of mobile-native/iosApp/iosApp/Features/Search/SearchView.swift (epic-82 story 82.3). Sort and map-toggle gaps noted.
