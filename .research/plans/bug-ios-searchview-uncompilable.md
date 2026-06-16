# bug-ios-searchview-uncompilable

**Vector:** bug
**Score:** 3
**Source:** Issue #1266 | PR #1257 (verify-redesign-search-promote)
**Confidence:** high

## Hypothesis
`mobile-native/iosApp/iosApp/Features/Search/SearchView.swift` does not compile: it calls `performSearch()` from 9 sites and `scheduleSearch(for:)` from 1 site, but neither function is defined in the file. Only `loadMoreResults()` and `buildKMPFilters()` exist among the helpers the view body references. Issue #1266 (filed as a follow-up from the `verify-redesign-search-promote` review of PR #1257) recorded the gap but no fix landed. Smallest change: restore the two missing async helpers (debounce-aware `scheduleSearch` + the actual KMP-bridged `performSearch`) mirroring the Android `SearchScreen.kt` contract so the SwiftUI view compiles and the AC-2 debounce / AC-4 infinite-scroll flows work end-to-end.

## Evidence
- Issue #1266: SearchView.swift body corrupted — `performSearch()` called from ~9 sites but never defined; `scheduleSearch(for:)` AC-2 debounce never defined; `resultsGrid` lost ForEach over results + `.onAppear { loadMoreResults() }` AC-4 infinite-scroll wiring.
- `mobile-native/iosApp/iosApp/Features/Search/SearchView.swift:50,74,82,88,94,102,120,166,184,190` — each line invokes `performSearch()` or `scheduleSearch(for:)`; `grep -n '^\s*private func\|^\s*func' SearchView.swift` returns only `loadMoreResults` (line 290) and `buildKMPFilters` (line 311).
- Brace count off by one in `SearchView.swift` (166 `{` vs 167 `}` — `wc -l` 595). The +2 mismatch the original brief recorded has been partly closed by an interim edit, but the file is still uncompilable because of the missing helpers.
- Android/KMP search path is fine and tested: `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/search/SearchScreen.kt` defines an equivalent `performSearch()` body and `SearchStateTest` covers it — the SwiftUI port simply lost the helpers.
- PR #1257 was a verify-only promotion that recorded the gap on the iOS side in #1266 rather than fixing it; that ticket is the open work item.

## Files
- `mobile-native/iosApp/iosApp/Features/Search/SearchView.swift`
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/search/SearchScreen.kt`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [x] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

Mode: local-only (reason: C5 — iOS Simulator + `xcodebuild`/`xcrun swiftc` toolchain is macOS-only; the routine sandbox cannot compile or runtime-verify Swift)

## Repro steps
1. From a fresh checkout on `dev`: `cd mobile-native/iosApp && xcodegen generate`.
2. Open `iosApp.xcodeproj` in Xcode (or run `xcodebuild -scheme RealityPortal-Dev -destination 'platform=iOS Simulator,name=iPhone 15' build`).
3. Expected: build succeeds. Actual: Swift compile fails in `Features/Search/SearchView.swift` with two errors of shape "Cannot find 'performSearch' in scope" and "Cannot find 'scheduleSearch' in scope" emitted for each of the ~10 call sites.

## Suggested approach
1. Add a private `@MainActor`-isolated async `performSearch()` on `SearchView` that mirrors `SearchScreen.kt` — assemble `ListingSearchRequest` from `searchText`, `buildKMPFilters()`, `sortOption.kmpSortOption`, page `1`, pageSize `20`; await `listingRepository.searchListings(request:)`; on success replace `results`, `totalResults`, `totalPages`, reset `currentPage = 1`; on failure clear `results` and surface the error via the existing `#if DEBUG` print. Set/clear `isLoading` around the await.
2. Add a private `scheduleSearch(for newValue: String)` that cancels a stored `Task?` debounce handle and starts a 250ms debounce (matching the Android `SearchScreen.kt` AC-2 timing) before calling `performSearch()`. Store the handle as `@State private var debounceTask: Task<Void, Never>?`.
3. Audit `resultsGrid` (line 266) — confirm it iterates `results` via `ForEach` and attaches `.onAppear { Task { await loadMoreResults() } }` to the last cell (AC-4 infinite scroll). If the body is missing, restore it.
4. Fix the brace mismatch (`grep -c '{'` vs `grep -c '}'` must agree) — likely a single stray `}` left behind by the corruption.
5. Add a stale-response guard the way SearchScreen.kt should (and currently doesn't — separate backlog item `code-review-mobile-native-kmp-search-stale-response-race`): capture an incrementing `requestSeq` before `await`, drop the response if a newer one started. Keep this scoped to SwiftUI to avoid widening the PR into the Compose side.
6. Verify locally: `xcodebuild -scheme RealityPortal-Dev -destination 'platform=iOS Simulator,name=iPhone 15' build` exits 0; `xcodebuild test -scheme RealityPortal-Dev -only-testing:iosAppTests/SearchViewTests` (add the test in step 7) passes.
7. Write `iosApp/iosAppTests/SearchViewTests.swift` exercising `performSearch()` against a stub `ListingRepository` that returns a fixed `ListingSearchResponse` — assert `results.count`, `totalResults`, and that the loading state cycles. Drives IG3: this test fails on `dev` (the file does not even compile) and passes on the branch.

## Alternatives considered
- **Delete `SearchView.swift` and stub the route** — rejected because the iOS Reality Portal app advertises search as a top-level feature; removing the screen leaves the redesigned tab bar pointing at a missing destination and would force a parallel screen-map status downgrade. Restoring the helpers is the same amount of code (the bodies already exist in Android) and preserves the AC-2/AC-4 coverage the redesign promised.
- **Revert to a prior good `SearchView.swift`** — rejected because the file is born broken: `git log --diff-filter=A` shows no earlier good revision and the brief on 2026-06-11 already noted "Born broken: file only ever existed in [the introducing] commit". There is nothing to revert to.

## Root-cause trace
1. Symptom: `xcodebuild` emits "Cannot find 'performSearch' in scope" / "Cannot find 'scheduleSearch' in scope" at every call site in `SearchView.swift` — the iOS app target fails to link and the search tab is dark on device.
2. ← `mobile-native/iosApp/iosApp/Features/Search/SearchView.swift:50,74,82,88,94,102,120,166,184,190` — the SwiftUI view body invokes the helpers, but `grep '^\s*private func\|^\s*func'` against the file returns only `loadMoreResults` (`:290`) and `buildKMPFilters` (`:311`); the two driver helpers are absent.
3. ← Introducing commit landed the SwiftUI view body *without* the helper implementations — the brace count is also off by one (166 `{` vs 167 `}`), so the file was committed in an incomplete state. There is no preceding green revision to bisect against (see Alternatives #2).
4. Origin: the `verify-redesign-search-promote` track shipped PR #1257 (verify-only, no production code change) and filed Issue #1266 to track the gap rather than fixing it; nothing has landed against #1266 since.

## Test plan
- [ ] `mobile-native/iosApp/iosAppTests/SearchViewTests.swift` — new test exercising `performSearch()` against a stub `ListingRepository`; asserts that a success response populates `results`/`totalResults` and that the loading state cycles. Fails on `dev` (compile error in the SUT) and passes on the branch.
- [ ] Regression: `scheduleSearch(for:)` debounce coalesces three keystrokes within 250ms into a single repository call (use a recording stub `ListingRepository` and a `Task.sleep` timeline) — protects AC-2.
- [ ] Local command: `cd mobile-native/iosApp && xcodegen generate && xcodebuild -scheme RealityPortal-Dev -destination 'platform=iOS Simulator,name=iPhone 15' build test -only-testing:iosAppTests/SearchViewTests`
- [ ] CI parity: a re-run of the Android-side `SearchStateTest` (`./gradlew :androidApp:testDebugUnitTest --tests '*SearchStateTest'`) stays green — the iOS fix must not change KMP shared signatures.

## Out of scope
- The KMP shared `searchListings` repository contract — this plan restores the iOS caller, it does not change `ListingRepository`, `ListingSearchRequest`, or `ListingSearchResponse`.
- The Android `SearchScreen.kt` stale-response race (`code-review-mobile-native-kmp-search-stale-response-race`, score 2 in backlog) — out of scope here; the iOS-side guard added in step 5 is only on the SwiftUI side to keep the PR narrow.
- iOS deep-link Universal Links wiring (`bug-ios-deeplink-info-plist-missing`, score 2) — separate backlog item; this plan does not touch `Info.plist`.
- Any redesign / screen-map status flip for `reality/search` — that follows after this fix lands and the screen actually runs.

## After-merge
- Move this file to `plans/_archive/bug-ios-searchview-uncompilable.md`
- Mark the matching `backlog.json` row as `status: "done"`
