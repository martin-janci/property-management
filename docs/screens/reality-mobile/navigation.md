---
id: reality-mobile/navigation
name: Navigation & Deep-Linking Infrastructure (iOS SwiftUI)
product: reality-mobile
sitemapRefs: {}
implementations:
  ios-swiftui:
    component: NavigationCoordinator / DeepLinkHandler / NavigationStateRestorationService
    route: MainTabView (TabView + per-tab NavigationStack), Route enum
    buildStatus: shipped
    redesignStatus: n/a
    apiStatus: n/a
endpoints: []
relatedScreens:
  - id: reality-mobile/auth-login
    rel: child
  - id: reality-mobile/home
    rel: child
  - id: reality-mobile/account
    rel: child
sharedComponents: []
diagrams: []
useCases: []
epics:
  - "82"
designSources: []
owner: reality-frontend
---

## Functionality Checklist

Cross-cutting navigation/routing infrastructure for the Reality Portal iOS app
(Epic 82, Story 82.2). This is not a visible "screen" — it documents the
navigation, deep-link, state-restoration, and auth-guard layer that the visible
screens sit on top of, and records the AC-4 / AC-5 verification evidence
(Coverage 82-2).

- [x] [m] **AC-4 — Navigation state preservation across launches/backgrounding.**
  `NavigationStateRestorationService` snapshots `selectedTab` + per-tab route
  stacks to `UserDefaults` (`navigation_state`) on `scenePhase == .background`
  and restores them at launch (`RealityPortalApp.configureApp()`).
- [x] [m] **AC-4 — Route mirrors keep `NavigationPath` serialisable.**
  `NavigationCoordinator` maintains `*Routes` mirror arrays shadowing the opaque
  `NavigationPath`s; `restoreStack(_:for:)` writes both atomically so a
  save→restore→save cycle does not silently drop stacks (regression test
  `testRoundTripPersistenceSurvivesDoubleRestoreCycle`).
- [x] [m] **URL-scheme deep-linking.** `DeepLinkHandler.parse(_:)` maps
  `realityportal://...` custom-scheme URLs and allow-listed `https://`
  universal links to typed `Route` values (listing/gallery/map, search +
  filters, favorites, inquiries + detail + new, account/profile/settings,
  realtors, agencies, saved-searches, compare). SSO callbacks
  (`realityportal://sso?token=&state=`) are returned as a distinct
  `.ssoCallback` result, never as a navigation route.
- [x] [m] **Universal-link host allow-list.** `allowedUniversalLinkHosts`
  rejects unknown `https://` hosts as defence-in-depth even before AASA is wired.
- [x] [m] **AC-5 — Auth guard on deep links.** `RealityPortalApp.handleIncomingURL`
  intercepts routes whose `Route.requiresAuth == true` when
  `!authManager.isAuthenticated`, stashes them on
  `coordinator.pendingDestination`, and bounces to `.login`; the destination is
  replayed after a successful SSO login.
- [x] [m] **AC-5 — Auth guard on tab selection.** `MainTabView.handleTabChange`
  reverts to the previous tab and presents the login sheet when an
  unauthenticated user selects a protected tab (Favorites / Inquiries / Account).
- [x] [m] **AC-5 — Auth guard on restore.** `NavigationStateRestorationService.restore`
  drops protected tab stacks and falls back to `.home` when not authenticated, so
  protected content never reappears after a re-install / device transfer.
- [x] [m] **Logout wipes persisted nav state.** `AuthManager.logout()` calls
  `restorationService?.clear()` so protected stacks do not outlive the session
  on disk.
- [x] [m] **SSO CSRF nonce.** `beginSsoFlow()` mints a single-use nonce;
  `consumeSsoState(_:)` validates it exactly once (rejects nil/empty/mismatch/
  replay) before the SSO token is accepted.
- [x] [m] **`CFBundleURLTypes` registered in `Info.plist` (GAP closed).** The
  `realityportal` custom scheme is now declared in
  `mobile-native/iosApp/iosApp/Resources/Info.plist` under `CFBundleURLTypes`
  → `CFBundleURLSchemes` (`CFBundleURLName = three.two.bit.ppt.reality`, role
  `Editor`), added in commit `db3ccf1` (2026-06-16). iOS now delivers
  `realityportal://` URLs to `onOpenURL`, so the deep-link handler is live at
  runtime, not dead. The registration is pinned by
  `testInfoPlistRegistersDeepLinkScheme` (asserts the plist declares the scheme
  and that it matches `Environment.urlScheme`), and the scheme value itself by
  `testEnvironmentDeepLinkSchemeIsStable`.
- [ ] [m] **Remaining: no `applinks:` associated-domains entitlement.**
  Custom-scheme deep-linking works; `https://` universal links are parsed and
  host-allow-listed in `DeepLinkHandler` but iOS will not route them until an
  `applinks:` entitlement + AASA file are wired. Lower priority than the custom
  scheme (push-notification deep links use the custom scheme). Owner
  `pm-mobile` (`mobile-native/**`).

## States

- **Cold launch (authenticated)**: prior tab + per-tab stacks restored verbatim.
- **Cold launch (unauthenticated)**: protected tab/stacks dropped, lands on `.home`.
- **Background → foreground**: state saved on background, restored unchanged.
- **Deep link (allowed route)**: navigates directly to the typed `Route`.
- **Deep link (protected, signed-out)**: stashed as `pendingDestination`, routed to `.login`, replayed post-login.
- **Deep link (SSO callback)**: CSRF state validated, token exchanged, pending destination replayed.
- **Deep link (unknown / foreign host)**: `.unrecognized`, ignored (logged in DEBUG).

## Notes

### Broader context

This layer is implemented across:
`mobile-native/iosApp/iosApp/Core/Navigation/{NavigationCoordinator,DeepLinkHandler,NavigationStateRestorationService,Route}.swift`,
wired in `App/RealityPortalApp.swift` + `App/MainTabView.swift`, with auth
state from `Core/AuthManager.swift`. The Android counterpart uses scheme
`reality` (see `testEnvironmentDeepLinkSchemeIsStable` note) — the two schemes
are intentionally different and not yet reconciled.

### Specific (recent)

- AC-4 / AC-5 verified by static audit (Coverage 82-2). All three concerns —
  navigation state preservation, URL-scheme deep-linking, and the auth guard —
  are shipped in source and covered by `iosAppTests/RealityPortalTests.swift`
  (`DeepLinkHandlerTests`, `NavigationStateRestorationServiceTests`,
  `AuthenticationTests` SSO-nonce suite). No new feature code was required.
- 2026-06-17 re-verification: the runtime GAP previously flagged below is now
  **closed**. `Info.plist` declares the `realityportal` scheme under
  `CFBundleURLTypes` (added 2026-06-16, commit `db3ccf1`), so custom-scheme
  deep-linking is live, not dead. Pinned by the new
  `testInfoPlistRegistersDeepLinkScheme`. Only the `applinks:` universal-link
  entitlement remains outstanding (lower priority — push deep links use the
  custom scheme).
- Cross-platform scheme reconciliation is still intentionally deferred: Android
  uses `reality://` (manifest + shared `DeepLinkRouter.SCHEME`), iOS uses
  `realityportal://` (`Configuration.urlScheme` + plist). Both schemes are
  pinned by tests on their own platform; a single deep link still cannot target
  both apps. Tracked, not blocking.

## Agent Log

<!-- newest entries on top -->

- 2026-06-17 — agent: Re-verified deep-linking + URL-scheme handling for the Reality KMP/SwiftUI app (verify task, static — KMP/Xcode not buildable offline). Confirmed: Android registers `reality://` (AndroidManifest intent-filters for sso/listing/search/favorites/inquiries) routed through shared `DeepLinkRouter` (`MainActivity.handleDeepLink` + `RealityNavHost`); iOS registers `realityportal://` via `Info.plist` `CFBundleURLTypes` (the GAP flagged on 2026-06-10 is now CLOSED — added 2026-06-16 in `db3ccf1`) routed through `DeepLinkHandler` + `onOpenURL`. Added test evidence `testInfoPlistRegistersDeepLinkScheme` (RealityPortalTests.swift) pinning the plist registration against `Environment.urlScheme`. Updated checklist + Notes. Remaining outstanding: `applinks:` universal-link entitlement (iOS) and the intentional Android/iOS scheme divergence (`reality` vs `realityportal`) — both tracked, neither blocking.
- 2026-06-10 — agent: Coverage 82-2 — verified AC-4 (nav state preservation), URL-scheme deep-linking, and AC-5 (auth guard) against mobile-native iOS source. All three implemented + unit-tested; no feature code needed. Added this infra screen-map. Flagged GAP: `Info.plist` is missing the `realityportal` `CFBundleURLTypes` registration and `applinks:` entitlement (pm-mobile owner) — handler logic is complete but the OS never delivers the URL until that is added.
