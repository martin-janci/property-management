# Epic 82 — Story-vs-Implementation Mapping (Reality Portal iOS)

**Date:** 2026-06-17
**Task ID:** feat-swiftui-project-setup-vs-implementation-mapping-mobile
**Owner:** pm-frontend
**Type:** Documentation (mapping note — no code changes)

---

## Why this note exists

"Epic 82" is defined **two different ways** in this repository, and the two
definitions disagree on what Story 82.1 ("SwiftUI Project Setup") is and
whether it is even in scope. A research task surfaced the ambiguity:

> story-vs-implementation mapping unclear — epic 82 in `epics-007.md` targets a
> different scope than `82-1-swiftui-project-setup` "SwiftUI Project Setup".

This document reconciles the two and maps each story onto the **actual**
`mobile-native/iosApp/` implementation. It is the single source of truth for
"which Epic 82 are we talking about?".

---

## The two Epic-82 definitions

| | Source A — "SwiftUI build" | Source B — "Mobile Native Completion" |
|---|---|---|
| **File** | `_bmad-output/implementation-artifacts/stories/82-*.md` | `_bmad-output/epics-007.md` (§ Epic 82) |
| **Date / origin** | Greenfield iOS build plan | 2025-12-31, gap-remediation epics 79–85 |
| **Epic goal** | Build the iOS Reality Portal app from scratch with KMP | Fix broken functionality / complete API integration in the **already-built** iOS app |
| **Story 82.1** | **SwiftUI Project Setup** (Xcode project, KMP framework, xcconfig, Info.plist, schemes) | **Remove Broken Direct Login Form** |
| **Story 82.2** | Navigation and Routing | Fix Session Restoration Race Condition |
| **Story 82.3** | Home and Search Screens | Wire Category Filter Chips |
| **Story 82.4** | Listing Detail and Favorites | Persist Recent Searches |
| **Story 82.5** | Inquiries and Account | Add Map Integration |
| **Story 82.6** | — (5 stories) | Initialize ApiConfig at Startup |

**Key takeaway:** the story *numbers* collide but the story *content* does not.
Source A is the **build** epic; Source B is the **finish/fix** epic. They are
sequential, not duplicates: Source B presupposes Source A is done.

### Which one is authoritative?

- **`epics-007.md` (Source B) is the authoritative epic backlog.** It is the
  curated, dated epic breakdown that downstream planning (`sprint-status.yaml`,
  research backlog) tracks. When someone says "Epic 82" without qualification,
  they mean **Mobile Native Completion**.
- **The `implementation-artifacts/stories/82-*.md` files (Source A) are the
  historical build stories** that produced the current `iosApp/` tree. They are
  retained for traceability of the original implementation, but their story
  numbering (82.1 = SwiftUI Project Setup, etc.) is **legacy** and should not be
  cited as the current backlog.

> Existing docs that reference "82.2 / 82.3 / 82.4 / 82.5" — namely
> `docs/screens/reality-mobile/README.md` and
> `docs/superpowers/plans/gap-82-1-swiftui-audit.md` — use the **Source A**
> (build-story) numbering. That is correct for those documents (they describe
> the build), but readers must not conflate them with the `epics-007.md`
> completion stories of the same number.

---

## Source A (SwiftUI build) → implementation mapping

The build stories map cleanly onto the current `mobile-native/iosApp/` tree.
All are effectively **implemented in code** (some screens still have stub
destinations — see the audit).

| Build story | Description | Implemented by | Status |
|---|---|---|---|
| **82.1** | SwiftUI Project Setup | `iosApp/iosApp/App/RealityPortalApp.swift` (@main), `Core/Configuration.swift`, `Configurations/{Base,Development,Staging,Production}.xcconfig`, `Resources/Info.plist`, `Resources/Assets.xcassets/AppIcon*.appiconset`, KMP framework export in `shared/build.gradle.kts` | **Done** (verified by `gap-82-1-swiftui-audit.md` § Project Setup Verification) |
| **82.2** | Navigation and Routing | `Core/Navigation/{NavigationCoordinator,DeepLinkHandler,Route,NavigationStateRestorationService}.swift`, `App/MainTabView.swift` | **Done** (shipped) |
| **82.3** | Home and Search Screens | `Features/Home/HomeView.swift`, `Features/Search/SearchView.swift` | In-progress (category chips unwired) |
| **82.4** | Listing Detail and Favorites | `Features/Listing/ListingDetailView.swift`, `Features/Favorites/FavoritesView.swift` | In-progress |
| **82.5** | Inquiries and Account | `Features/Inquiries/InquiriesView.swift`, `Features/Account/AccountView.swift`, `Features/Auth/LoginView.swift` | In-progress |

Beyond the build epic, the iosApp also already ships `SavedSearchesView`,
`CompareListingsView`, `RealtorsView`, and `AgenciesView` (mapped to UCs, not to
Epic 82).

### Story 82.1 "SwiftUI Project Setup" specifically

This is the story the research task called out. Its acceptance criteria (AC-1
Xcode structure / AC-2 KMP framework / AC-3 dependencies / AC-4 Info.plist &
icons / AC-5 build schemes) are **all satisfied** by the current tree:

- AC-1 / AC-4 — Xcode project under `iosApp/`, bundle ID
  `three.two.bit.ppt.reality` (`.dev` suffix for Development), `Info.plist`,
  `Assets.xcassets` per-env app icons. ✓
- AC-2 — KMP `shared` framework exported and consumed from Swift
  (`Core/DI/DependencyContainer.swift`, `KMPBridge`). ✓
- AC-3 — dependencies resolved (Keychain via native `KeychainService.swift`;
  imaging via Coil/Kingfisher per stack). ✓
- AC-5 — `Base/Development/Staging/Production` xcconfig + per-env API URL via
  `$(API_BASE_URL)` token. ✓

So the **build** 82.1 is complete. The `Status: pending` header in
`82-1-swiftui-project-setup.md` is **stale** — it predates the implementation
and was never flipped. Treat the code + audit as ground truth over that header.

---

## Source B (Mobile Native Completion) → implementation mapping

These are the *current* Epic-82 stories from `epics-007.md`. They are
fix/finish tasks on top of the build above.

| Completion story | Description | Touch point | Notes |
|---|---|---|---|
| **82.1** | Remove Broken Direct Login Form | `Features/Auth/LoginView.swift` | Audit confirms email/password `login()` always throws `AuthError.ssoRequired`; form is UI-only. This story removes it or makes SSO-only messaging explicit. |
| **82.2** | Fix Session Restoration Race Condition | `Core/AuthManager.swift`, `App/RealityPortalApp.swift` | Make `restoreSession` async; add loading state to `MainTabView`. |
| **82.3** | Wire Category Filter Chips | `Features/Home/HomeView.swift`, `Features/Search/SearchView.swift` | Audit flags `HomeView.categoryFilters` chips have empty tap closures. |
| **82.4** | Persist Recent Searches | `Features/Search/SearchView.swift` | Replace hardcoded suggestions with `UserDefaults`-persisted history. |
| **82.5** | Add Map Integration | `Features/Listing/ListingDetailView.swift` (and `Features/Listing/ListingMapView.swift`) | Replace map placeholder with MapKit + pin + directions. |
| **82.6** | Initialize ApiConfig at Startup | `App/RealityPortalApp.swift`, `shared/.../api/ApiConfig.kt` | Initialize `ApiConfig.baseUrl` in `configureApp()`. |

Several of these line up with gaps already recorded in
`gap-82-1-swiftui-audit.md` (login form throws, category chips unwired, map
placeholder) — the audit is the technical evidence backing the completion
stories.

---

## Practical guidance for agents

1. **A task that says "Epic 82" / "82.x" with no other qualifier** → use the
   **`epics-007.md` Mobile Native Completion** definition (Source B).
2. **A task or doc citing "SwiftUI Project Setup", "Navigation and Routing",
   "Home and Search Screens"** etc. → that is the **build** numbering
   (Source A); it is historical. The work is already in `iosApp/`.
3. **Do not re-do Source A 82.1 ("SwiftUI Project Setup")** — the project is set
   up. Any remaining iOS work is Source-B completion or the stub destinations
   listed in the audit and `docs/screens/reality-mobile/README.md`.
4. When you finish a Source-B story, update `epics-007.md` is **not** required
   (it is a frozen breakdown); record status in the research backlog /
   sprint-status instead.

---

## References

- `_bmad-output/epics-007.md` § Epic 82 (Mobile Native Completion) — authoritative epic.
- `_bmad-output/implementation-artifacts/stories/82-{1..5}-*.md` — legacy build stories (Source A).
- `docs/superpowers/plans/gap-82-1-swiftui-audit.md` — implementation audit of the built iOS app.
- `docs/screens/reality-mobile/README.md` — per-screen build status (uses Source-A numbering).
- `mobile-native/iosApp/` — the actual SwiftUI implementation.
