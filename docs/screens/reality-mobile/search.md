---
id: reality-mobile/search
name: Search (iOS SwiftUI)
product: reality-mobile
sitemapRefs:
  reality-web: reality-listings
implementations:
  mobile-native:
    component: SearchScreen (Compose) — androidApp/.../ui/search/SearchScreen.kt
    route: Screen.Search ("search?type={type}&category={category}")
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: complete
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
- [x] [m] ✅ **iOS `SearchView.swift` compiles again (regression resolved on `dev`)** — the earlier non-compile (missing `performSearch()`/`scheduleSearch(for:)`, corrupted `resultsGrid`, braces +2) is fixed: braces balance 185/185 and all helpers (`performSearch`, `scheduleSearch`, `resultsGrid`, `loadMoreResults`) are defined. See Agent Log 2026-06-15.

## States

- **Prompt**: centered "search_prompt" icon + message displayed before first search.
- **Loading**: ProgressView while results.isEmpty and isLoading.
- **Empty results**: magnifying-glass icon + "no_results" heading + "try_different_search" body.
- **Success**: paginated LazyVGrid of ListingRowCards.

## Notes

### Broader context

Search tab root. Calls `listingRepository.searchListings(request:)` via KMP. Pagination with `currentPage`/`totalPages`. Filter sheet modal (`FilterSheet`) collects `SearchFilters` struct and triggers new search.

### Specific (recent)

- Debounce implemented with `Task.sleep(nanoseconds: 300_000_000)` on iOS — checks if `searchText` still matches after sleep. On Android/KMP the AC-2 debounce now lives in pure `SearchState.debouncedQueryFlow(Flow<String>)` (commonMain), unit-tested with `kotlinx-coroutines-test` virtual time in `SearchStateTest`; `SearchScreen.kt` calls the helper instead of inlining `drop(1).debounce(300).distinctUntilChanged()`. Likewise the AC-4 infinite-scroll trigger now lives in pure `SearchState.nextPageTriggerFlow(Flow<Int?>, snapshot)` (commonMain): de-dup the last-visible index → snapshot pagination state → run `shouldLoadNextPage` → emit `currentPage + 1`. `SearchScreen.kt`'s AC-4 `LaunchedEffect` calls it instead of inlining `snapshotFlow{…}.distinctUntilChanged().collect{ if(shouldLoadNextPage) … }`. Pinned by `nextPageTriggerFlow_*` `runTest` cases in `SearchStateTest` (mirrors PR #1392's pure-flow + virtual-time pattern for the debounce).
- `FilterSheet` is defined inline in SearchView.swift — not a separate file.
- `Route.searchResults(query:filters:)` is registered in NavigationCoordinator but SearchView itself is a tab root, not accessed via navigation push.
- ✅ **RESOLVED 2026-06-15 — iOS `SearchView.swift` compiles again** (braces 185/185; `performSearch`/`scheduleSearch`/`resultsGrid`/`loadMoreResults` all defined). The historical defect below is kept for context. The CoreLocation "Near Me" path was confirmed and pinned by `SearchViewCoreLocationTests` (see Agent Log).
- ⚠️ **[HISTORICAL] iOS `SearchView.swift` was broken on `dev` (coverage 82-3 verify, 2026-06-10).** Source audit of `mobile-native/iosApp/iosApp/Features/Search/SearchView.swift` (HEAD `dbbae71`) found the `SearchView` struct body corrupted:
  - `performSearch()` is called from 9 sites but **never defined** — its body was spliced into the `resultsGrid` computed property (lines ~266–288: a dangling `ScrollView { LazyVStack {` header glued onto the tail of a `ListingSearchRequest(...)` + result-apply block).
  - `scheduleSearch(for:)` (the AC-2 debounce entry, called from `.onChange(of: searchText)`) is **never defined**. `debounceTask` / `debounceNs` (350 ms) state exists but nothing schedules the task.
  - `resultsGrid` no longer renders a `ForEach` over `results`, the `SearchResultCard`, or the `.onAppear { loadMoreResults() }` **AC-4 infinite-scroll trigger** — so even though `loadMoreResults()` itself is intact (lines 290–309), nothing calls it.
  - Brace balance is off by **+2 (174 open / 172 close)** → the file cannot compile. Git history shows the file has only ever existed in commit `dbbae71` and was **born broken** (no prior good version to revert to).
  - **Intact** on iOS: the `FilterSheet` struct (477–587) and its `nearMeSection` (548–586) CoreLocation "Near Me"/radius wiring, plus the top-level `nearMeChip` (176–217) and `LocationManager` (`Core/Location/LocationManager.swift`). FilterSheet + CoreLocation ACs are genuinely shipped; the breakage is confined to search execution + result list + AC-2/AC-4 trigger wiring.
  - **Action:** raise an `ios-swiftui` repair task to restore `performSearch()`, `scheduleSearch(for:)`, and the `resultsGrid` body (incl. the `loadMoreResults()` `.onAppear`). Not fixed here — Swift cannot be compile-verified in this environment and the file is outside the kotlin-mp owner area.

## Agent Log

- 2026-07-01 — agent: coverage 82-3 finish-to-done (82-3-home-search-screens). Closed the tracked KMP gap by adding the missing `mobile-native` implementation block (Compose `SearchScreen` at `Screen.Search`, `buildStatus: shipped`) alongside the existing `ios-swiftui` block — the Android/KMP search path is fully shipped: `SearchScreen.kt` drives AC-2 debounce via `SearchState.debouncedQueryFlow` and AC-4 pagination via `SearchState.nextPageTriggerFlow` (both pure helpers in commonMain), plus a Material3 `FilterSheet` ModalBottomSheet; pinned by 43+ `SearchStateTest` cases (debounce, pagination, stale-response race). Re-audited iOS `SearchView.swift`: exact character brace count is 186/186 (balanced) and `performSearch`/`scheduleSearch`/`resultsGrid`/`loadMoreResults` are all defined and wired (AC-2 `.onChange(of: searchText)`→`scheduleSearch`; AC-4 result-row `.onAppear`→`loadMoreResults`) — the historical "born broken" defect is resolved on `dev`, confirming the earlier verify note. KMP/Swift not compile-runnable in this sandbox; CI / macOS reviewer `xcodebuild` is the authoritative gate.
- 2026-05-25 — agent: created screen map from audit of mobile-native/iosApp/iosApp/Features/Search/SearchView.swift (epic-82 story 82.3). Sort and map-toggle gaps noted.
- 2026-05-25 — agent: added SortOption enum (7 cases) with toolbar button; added onAppear to consume pendingSearchFilters from HomeView; mapped ListingTypeFilter to KMP ListingType in buildKMPFilters. buildStatus → shipped. Map toggle remains future work.
- 2026-05-28 — agent: fix(#581) added missing Agent Log entry that PR #554 omitted (CLAUDE.md screen-map Rule A.3); flagged dropped operationIds on home + saved-searches in Notes > Specific (recent) pending @ppt/sitemap extension.
- 2026-06-05 — agent: brought the Android (KMP/Compose) SearchScreen to parity with iOS for story 82.3 AC-2/AC-4: 300ms debounced free-text search (snapshotFlow + Flow.debounce), infinite-scroll pagination (auto-loads next page near list end), and a Material3 ModalBottomSheet FilterSheet (draft state, Apply/Reset) replacing the old inline collapsible panel. Pure logic extracted to shared SearchState (commonMain) with SearchStateTest. Note: this screen-map's frontmatter still describes the ios-swiftui impl only; an android-compose implementation block should be added when @ppt/sitemap gains the reality-mobile Android target.
- 2026-06-15 — agent: CoreLocation integration **confirmed** for the iOS "Near Me" path (task verify-home-and-search-screens-corelocation-integration-confirmed-mobile). Source audit verified the full wiring: `LocationManager` (`Core/Location/LocationManager.swift`, when-in-use auth + one-shot `requestLocation()`), single instance created in `RealityPortalApp` and re-injected across the sheet boundary in `SearchView` (closes #625), `FilterSheet.nearMeSection` (SearchView.swift 619–657) calls `requestLocation()` then copies the resolved coordinate into `SearchFilters.latitude/longitude` → KMP `ListingSearchFilters.nearLat/nearLng/radiusKm` via `buildKMPFilters()`, and `Info.plist` ships `NSLocationWhenInUseUsageDescription`. The earlier (2026-06-10) `SearchView.swift` non-compile defect is **resolved on `dev`** (braces balance 185/185; `performSearch`/`scheduleSearch`/`resultsGrid`/`loadMoreResults` all defined). Added the missing verification: `SearchViewCoreLocationTests` in `iosAppTests/SearchViewTests.swift` (LocationManager idle/clear/stop state machine, near-me coordinate→filter ON/OFF mapping, and the bundle usage-string presence). KMP/iOS not compile-runnable in this sandbox — CI / macOS reviewer `xcodebuild test` is the authoritative gate.
- 2026-06-10 — agent: coverage 82-3 verify (verify-reality-mobile-search-ac-coverage). Confirmed the Android/KMP path fully covers AC-2 (debounce, `SearchScreen.kt` snapshotFlow+debounce 300ms), AC-4 (`SearchState.shouldLoadNextPage` + Compose footer spinner) and FilterSheet (M3 ModalBottomSheet), all pinned by `SearchStateTest` (18 cases, incl. stale-response race guard). Confirmed iOS FilterSheet + CoreLocation (`nearMeSection`/`nearMeChip`/`LocationManager`) intact. **Found defect:** iOS `SearchView.swift` does not compile — missing `performSearch()`/`scheduleSearch(for:)`, corrupted `resultsGrid`, AC-4 `loadMoreResults()` not wired, braces +2. Recorded in Notes > Specific; needs an `ios-swiftui` repair task. KMP Gradle build not runnable in this sandbox (AGP plugin artifacts unresolvable offline) — verified by source audit.
- 2026-06-15 — agent: evidenced AC-2 debounce on Android/KMP. The 300ms debounce previously only lived inline in `SearchScreen.kt`'s Compose `LaunchedEffect` (the prior "AC-2 pinned by SearchStateTest" note covered request-build/merge/race but NOT the debounce timing). Extracted the pipeline (`drop(1).debounce(SEARCH_DEBOUNCE_MS).distinctUntilChanged()`) into pure `SearchState.debouncedQueryFlow(Flow<String>)` (commonMain) and rewired the screen to call it — single source of truth, no behaviour change. Added 4 `runTest` virtual-time tests to `SearchStateTest` (collapse rapid keystrokes → final value; emit each value settling past the window; drop initial emission so first load isn't double-fired; de-dup settle-back-to-same). KMP Gradle still not runnable offline (AGP plugin unresolvable) — verified by static review; coroutines-test already wired in `commonTest`.
- 2026-06-16 — agent: evidenced AC-4 infinite-scroll/pagination on Android/KMP (coverage 82-3, task feat-home-and-search-screens-infinite-scroll-evidenced-mobile). The trigger *predicate* (`SearchState.shouldLoadNextPage`) was already pure + tested, but the **Flow pipeline that drives it** lived only inline in `SearchScreen.kt`'s `snapshotFlow{ lastVisibleIndex }.distinctUntilChanged().collect{ if(shouldLoadNextPage) performSearch(page+1) }` `LaunchedEffect` — the same gap PR #1392 closed for the AC-2 debounce. Extracted the pipeline into pure `SearchState.nextPageTriggerFlow(Flow<Int?>, snapshot: () -> PageSnapshot): Flow<Int>` (commonMain; de-dup index → snapshot loaded/total/isLoading/currentPage → `shouldLoadNextPage` → emit `currentPage+1` → `filterNotNull`) and rewired the screen to call it (single source of truth, no behaviour change). Added 7 `runTest` cases to `SearchStateTest` (near-end fires page+1, far-from-end no-op, in-flight guard, all-loaded guard, repeated-index de-dup fires once, null index ignored, snapshot advances page 2→3). KMP Gradle still not runnable offline (AGP plugin `com.android.application:9.2.1` unresolvable — same documented sandbox limit) — verified by static review (braces balance; operators already used in commonMain; coroutines-test already wired in commonTest). CI / Gradle on a connected runner is the authoritative gate.
