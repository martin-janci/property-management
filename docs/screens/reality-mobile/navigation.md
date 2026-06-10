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
- [ ] [m] **GAP — `CFBundleURLTypes` not registered in `Info.plist`.** The
  parsing + routing + auth-guard logic is complete and unit-tested, but
  `mobile-native/iosApp/Info.plist` does not declare a `CFBundleURLTypes`
  entry for the `realityportal` scheme, nor an `applinks:` associated-domains
  entitlement. Without the URL-type registration iOS will not deliver
  `realityportal://` URLs to `onOpenURL`, so deep-linking cannot fire at
  runtime. Fix is owned by `pm-mobile` (touches `mobile-native/**`) — see
  Agent Log. The scheme name is pinned in `Configuration.urlScheme`
  (`"realityportal"`) and by `testEnvironmentDeepLinkSchemeIsStable`.

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
- One runtime gap found: the `realityportal` URL scheme is not declared in
  `Info.plist` (`CFBundleURLTypes`) and there is no `applinks:` entitlement for
  universal links. The deep-link handler is dead at runtime until that
  registration is added under `mobile-native/**` (pm-mobile owner).

## Agent Log

<!-- newest entries on top -->

- 2026-06-10 — agent: Coverage 82-2 — verified AC-4 (nav state preservation), URL-scheme deep-linking, and AC-5 (auth guard) against mobile-native iOS source. All three implemented + unit-tested; no feature code needed. Added this infra screen-map. Flagged GAP: `Info.plist` is missing the `realityportal` `CFBundleURLTypes` registration and `applinks:` entitlement (pm-mobile owner) — handler logic is complete but the OS never delivers the URL until that is added.
