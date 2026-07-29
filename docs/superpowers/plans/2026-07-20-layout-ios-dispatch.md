# Layout iOS Dispatch Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The iOS Reality app renders listing-detail section order/visibility from the shared resolved layout (completing spec §9 step 6): a SwiftUI dispatch over `ResolvedLayoutScreen` from the KMP framework, seeded with the compiled default, fetched once at screen entry.

**Architecture:** No new networking or models — the KMP shared `layout` package (merged in #2432) is iOS-ready. iOS adds: `LayoutRepository` to `DependencyContainer`, a fetch in `ListingDetailView`'s load path (default on any failure — the repository never throws domain-wise, but KMP `suspend` surfaces as `async throws` in Swift, so wrap in `try?`), and the section `switch` replacing the fixed VStack order. Mapping: `gallery.v1`→photoHeader, `listing-header.v1`→priceSection, `key-details.v1`→featuresRow, `description.v1`→descriptionSection, `features.v1`→amenitiesGrid, `agent-contact.v1`→agentCard+contactButton (both gated together), `additional-info.v1`/`resources.v1`→no-op (nothing distinct on iOS), unknown→no-op, placeholder→`LayoutPlaceholderSection` view. `locationSection` (the map) stays UNMANAGED — always renders after the managed sections (deep-native surface, spec's own out-of-scope guidance). Dividers render between adjacent visible managed sections. Pixel-identical under the compiled default.

**⚠️ Verification constraint:** NO macOS builder exists in this environment (fleet checked — all Linux). The Swift code CANNOT be compiled here. This matches the repo's existing posture (CI never builds the Swift app; `iosApp.xcodeproj` is generated on-demand via xcodegen). Mitigation: strict pattern-fidelity to existing Swift files, adversarial review focused on KMP↔Swift interop correctness, and an explicit PR note that a `scripts/build-ios.sh development` run on a Mac is required before release.

## Global Constraints

- **Branch:** `feature/layout-ios-dispatch` from `dev`.
- KMP↔Swift interop facts the implementer MUST honor (verify each against how `ListingDetailView.swift` + `DependencyContainer.swift` already consume the shared framework — mirror those idioms exactly):
  - Shared framework module import name (check existing `import` lines in Swift files — likely `import shared` or similar).
  - Kotlin top-level val `DEFAULT_LISTING_DETAIL_LAYOUT` in `DefaultLayout.kt` surfaces as `DefaultLayoutKt.DEFAULT_LISTING_DETAIL_LAYOUT` (file-facade class naming; ADAPT to how other top-level members are accessed, if any precedent exists).
  - Kotlin `suspend fun getListingDetailLayout()` surfaces as `async throws` in Swift — call as `try? await`, fall back to the default on nil.
  - Kotlin `List<ResolvedLayoutSection>` bridges as an NSArray-backed collection — iterate as Swift array of the bridged class; `Int` bridges as `Int32`; `isPlaceholder`/`isVisible` vals surface as Swift Bool properties.
  - Construct nothing Kotlin-side from Swift except via existing factory paths; the default layout object comes from the framework, never re-declared in Swift.
- Swift style: mirror `ListingDetailView.swift`'s existing private `@ViewBuilder` computed-property style; no new dependencies; strings — check how user-facing strings are handled in the file (literals vs Localizable) and match (ADAPT + report).
- `agent-contact.v1` gates BOTH `agentCard` and `contactButton`; placeholder → one placeholder block in their position; hidden → neither.
- Fetch-once semantics: layout state is `@State`, seeded with the default, set at most once from the fetch result in the existing `.task`/load path (mirror how the listing fetch is structured); no re-fetch, no mid-view swap after content renders (fetch layout BEFORE or alongside the listing fetch so both settle together — mirror the existing loading-state machine so the managed sections only appear after loading completes, making the single settle invisible).
- Runnable gates (Linux): `cd mobile-native && JAVA_HOME=/home/linuxbrew/.linuxbrew/opt/openjdk@17 ./gradlew spotlessCheck :shared:allTests` (must stay green — shared code untouched, this is a regression guard). NO pipes on build/test commands. Swift files: no automated gate — state this in reports.
- Commit scopes: `feat(mobile-native)`, `docs(...)`. ADAPT rule; report every interop assumption you verified vs guessed.

## File Structure

```
mobile-native/iosApp/iosApp/Features/Listing/ListingDetailView.swift   # dispatch refactor
mobile-native/iosApp/iosApp/Features/Listing/LayoutPlaceholderSection.swift  # new view
mobile-native/iosApp/iosApp/Core/DI/DependencyContainer.swift          # + layoutRepository
docs/repo-map.md                                                        # extend layout bullet
docs/screens/reality/listing-detail.md                                  # Agent Log entry
```

---

### Task 1: iOS dispatch implementation

**Files:** the three Swift files above.

- [ ] **Step 1:** Read `ListingDetailView.swift`, `DependencyContainer.swift`, and one more KMP-consuming Swift file end-to-end; write down (in your report) the verified interop idioms: framework import name, repository call pattern (`try? await` vs Result-bridging), how DependencyContainer registers repos.
- [ ] **Step 2:** `DependencyContainer`: add `lazy var layoutRepository = LayoutRepository(...)` mirroring the other repos' construction exactly.
- [ ] **Step 3:** `LayoutPlaceholderSection.swift`: neutral rounded-rect block, ~96pt min height, secondary-color "Sekcia nie je dostupná" text styled like the app's empty/placeholder states (match string-handling convention).
- [ ] **Step 4:** `ListingDetailView` refactor: `@State private var layout: ResolvedLayoutScreen = <default from framework>`; fetch in the existing load path (`try? await layoutRepository.getListingDetailLayout()` → set when non-nil, before loading completes); `listingContent` becomes a `ForEach`-free loop (a `ForEach` over the bridged array needs `Identifiable`/indices — use `Array(enumerated())` with indices as ids, or a plain loop inside a `@ViewBuilder` — pick what compiles under SwiftUI's result-builder limits: an explicit `ForEach(0..<sections.count, id: \.self)` over indices is the safe, boring choice) dispatching per the mapping; dividers between adjacent visible managed sections; `locationSection` appended after the loop, always.
- [ ] **Step 5:** Self-review against the interop facts; run the Linux regression gate (`spotlessCheck :shared:allTests`, JAVA_HOME as specified, foreground, no pipes).
- [ ] **Step 6:** Commit — `feat(mobile-native): iOS listing detail renders through the shared resolved layout`

---

### Task 2: Docs

- `docs/repo-map.md`: layout bullet — replace "iOS follow-up pending" with the iOS dispatch pointer + "compile-unverified on Linux; run scripts/build-ios.sh on macOS before release".
- `docs/screens/reality/listing-detail.md`: Agent Log — `2026-07-20 — agent: iOS listing detail renders via shared resolved layout dispatch (Swift compile pending macOS verification).`
- Rerun the Linux regression gate; commit — `docs(repo-map): layout iOS dispatch pointers`

---

## Deliberate scope decisions

- **Swift code ships compile-unverified** (no macOS builder; repo CI never builds Swift). The PR carries a prominent verification note; `scripts/build-ios.sh development` on a Mac is the acceptance step.
- **`locationSection` (map) stays unmanaged** — deep-native surface.
- **No iOS tests added** — `iosAppTests` only run on a Mac; adding untestable tests here is theater.
- **No KMP persistence** — same fetch-at-entry semantics as Android.

## Out of scope

KMP layout persistence; `layout_editor_*` capability; per-tenant preview; ppt-web frame-protection.
