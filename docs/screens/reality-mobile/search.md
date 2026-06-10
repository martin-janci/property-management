---
id: reality-mobile/search
name: Search (iOS SwiftUI)
product: reality-mobile
sitemapRefs:
  reality-web: reality-listings
implementations:
  ios-swiftui:
    component: SearchView
    route: Tab.search / Route.search
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: complete
endpoints:
  - listings_search
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
- [x] [m] PropertyType quick-filter chips in horizontal scroll filter bar
- [x] [m] Sort order selector (toolbar button → confirmationDialog with all 7 SortOptions)
- [ ] [m] Map view toggle — Route.listingMap defined but SearchView has no map UI (future task)
- [ ] [m] ⚠️ **iOS `SearchView.swift` does not compile (regression)** — see Notes > Specific. AC-2 debounce entry (`scheduleSearch`) and the core `performSearch()` are referenced but undefined; `resultsGrid` is corrupted and the AC-4 `loadMoreResults()` trigger is not wired into the list. Needs an `ios-swiftui` repair task.

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
- ⚠️ **iOS `SearchView.swift` is broken on `dev` (coverage 82-3 verify, 2026-06-10).** Source audit of `mobile-native/iosApp/iosApp/Features/Search/SearchView.swift` (HEAD `dbbae71`) found the `SearchView` struct body corrupted:
  - `performSearch()` is called from 9 sites but **never defined** — its body was spliced into the `resultsGrid` computed property (lines ~266–288: a dangling `ScrollView { LazyVStack {` header glued onto the tail of a `ListingSearchRequest(...)` + result-apply block).
  - `scheduleSearch(for:)` (the AC-2 debounce entry, called from `.onChange(of: searchText)`) is **never defined**. `debounceTask` / `debounceNs` (350 ms) state exists but nothing schedules the task.
  - `resultsGrid` no longer renders a `ForEach` over `results`, the `SearchResultCard`, or the `.onAppear { loadMoreResults() }` **AC-4 infinite-scroll trigger** — so even though `loadMoreResults()` itself is intact (lines 290–309), nothing calls it.
  - Brace balance is off by **+2 (174 open / 172 close)** → the file cannot compile. Git history shows the file has only ever existed in commit `dbbae71` and was **born broken** (no prior good version to revert to).
  - **Intact** on iOS: the `FilterSheet` struct (477–587) and its `nearMeSection` (548–586) CoreLocation "Near Me"/radius wiring, plus the top-level `nearMeChip` (176–217) and `LocationManager` (`Core/Location/LocationManager.swift`). FilterSheet + CoreLocation ACs are genuinely shipped; the breakage is confined to search execution + result list + AC-2/AC-4 trigger wiring.
  - **Action:** raise an `ios-swiftui` repair task to restore `performSearch()`, `scheduleSearch(for:)`, and the `resultsGrid` body (incl. the `loadMoreResults()` `.onAppear`). Not fixed here — Swift cannot be compile-verified in this environment and the file is outside the kotlin-mp owner area.

## Agent Log

- 2026-05-25 — agent: created screen map from audit of mobile-native/iosApp/iosApp/Features/Search/SearchView.swift (epic-82 story 82.3). Sort and map-toggle gaps noted.
- 2026-05-25 — agent: added SortOption enum (7 cases) with toolbar button; added onAppear to consume pendingSearchFilters from HomeView; mapped ListingTypeFilter to KMP ListingType in buildKMPFilters. buildStatus → shipped. Map toggle remains future work.
- 2026-05-28 — agent: fix(#581) added missing Agent Log entry that PR #554 omitted (CLAUDE.md screen-map Rule A.3); flagged dropped operationIds on home + saved-searches in Notes > Specific (recent) pending @ppt/sitemap extension.
- 2026-06-05 — agent: brought the Android (KMP/Compose) SearchScreen to parity with iOS for story 82.3 AC-2/AC-4: 300ms debounced free-text search (snapshotFlow + Flow.debounce), infinite-scroll pagination (auto-loads next page near list end), and a Material3 ModalBottomSheet FilterSheet (draft state, Apply/Reset) replacing the old inline collapsible panel. Pure logic extracted to shared SearchState (commonMain) with SearchStateTest. Note: this screen-map's frontmatter still describes the ios-swiftui impl only; an android-compose implementation block should be added when @ppt/sitemap gains the reality-mobile Android target.
- 2026-06-10 — agent: coverage 82-3 verify (verify-reality-mobile-search-ac-coverage). Confirmed the Android/KMP path fully covers AC-2 (debounce, `SearchScreen.kt` snapshotFlow+debounce 300ms), AC-4 (`SearchState.shouldLoadNextPage` + Compose footer spinner) and FilterSheet (M3 ModalBottomSheet), all pinned by `SearchStateTest` (18 cases, incl. stale-response race guard). Confirmed iOS FilterSheet + CoreLocation (`nearMeSection`/`nearMeChip`/`LocationManager`) intact. **Found defect:** iOS `SearchView.swift` does not compile — missing `performSearch()`/`scheduleSearch(for:)`, corrupted `resultsGrid`, AC-4 `loadMoreResults()` not wired, braces +2. Recorded in Notes > Specific; needs an `ios-swiftui` repair task. KMP Gradle build not runnable in this sandbox (AGP plugin artifacts unresolvable offline) — verified by source audit.
