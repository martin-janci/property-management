---
id: reality-mobile/project-setup
name: SwiftUI Project Setup & App Shell (iOS SwiftUI)
product: reality-mobile
sitemapRefs: {}
implementations:
  ios-swiftui:
    component: RealityPortalApp (@main) / DependencyContainer / KMPBridge / xcconfig + Info.plist
    route: App entry point (SwiftUI App protocol) — no visible route
    buildStatus: shipped
    redesignStatus: n/a
    apiStatus: n/a
endpoints: []
relatedScreens:
  - id: reality-mobile/navigation
    rel: sibling
  - id: reality-mobile/home
    rel: child
  - id: reality-mobile/auth-login
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

Cross-cutting project-setup / app-shell infrastructure for the Reality Portal
iOS app (Epic 82, Story 82.1). This is not a visible "screen" — it documents the
Xcode project configuration, build settings, KMP bridge wiring, dependency
container, localization, and `@main` entry point that every visible screen sits
on top of. Findings are grounded in the static audit
(`docs/superpowers/plans/gap-82-1-swiftui-audit.md`, 2026-05-25).

- [x] [m] **`@main` entry point.** `RealityPortalApp` conforms to the SwiftUI
  `App` protocol and injects `NavigationCoordinator` and `AuthManager` as
  `@State` so they persist through the scene lifecycle. `configureApp()` runs
  session restoration and navigation-state restoration at launch.
- [x] [m] **Bundle configuration.** Bundle ID `three.two.bit.ppt.reality` across
  all configurations, with a `three.two.bit.ppt.reality.dev` override on
  Development. `IPHONEOS_DEPLOYMENT_TARGET = 15.0`, `arm64`-only.
- [x] [m] **xcconfig environment matrix.** Four files —
  `Base.xcconfig` (shared product/compiler settings),
  `Development.xcconfig` (localhost:8081, logging, `.dev` suffix),
  `Staging.xcconfig`, and `Production.xcconfig` — drive per-environment
  endpoints and flags.
- [x] [m] **Info.plist token expansion.** `API_BASE_URL`, `ENVIRONMENT`, and
  `ENABLE_LOGGING` are declared as `$(VAR)` expansions resolved from xcconfig.
  Location, camera, and photo-library usage descriptions are present.
- [x] [m] **KMP bridge layer.** `KMPBridge` enum + `DependencyContainer` cleanly
  separate shared KMP types from SwiftUI models (e.g.
  `KMPBridge.toListingPreview()` maps `shared.ListingSummary` →
  Swift `ListingPreview`). Repositories are reached via
  `DependencyContainer.shared.*`.
- [x] [m] **Keychain-backed token storage.** `AuthManager` stores access /
  refresh tokens in the iOS Keychain
  (`kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly`, service key = bundle ID),
  never `UserDefaults`.
- [x] [m] **Localization scaffold.** Six locale bundles —
  `sk.lproj`, `cs.lproj`, `de.lproj`, `en.lproj`, `hu.lproj`, `pl.lproj` —
  with Swift string lookups via `String(localized:)`.
- [x] [m] **Modern concurrency / observation.** Swift `@Observable` macro used
  throughout (iOS 17+ pattern, no `ObservableObject` mixing); `async/await`
  with no completion-handler nesting; `#if DEBUG` guards keep sample data out
  of production builds.
- [ ] [m] **GAP — Tab titles not localized.** `Tab.title` returns raw English
  strings rather than `String(localized:)` / `LocalizedStringKey`, so the tab
  bar is not internationalised even though six locale bundles exist.
- [ ] [m] **GAP — `DependencyContainer` defaults to unauthenticated
  repositories.** Lazy repository vars do not inject session tokens; views use
  separate `make*` factory methods for authenticated calls, an inconsistent
  pattern that risks accidental unauthenticated reads.
- [ ] [m] **GAP — `SsoService` instantiated twice.** One instance lives in
  `AuthManager`, another in `DependencyContainer.shared.ssoService`; the two
  are independent and their session state can diverge.

## States

- **Cold launch**: `configureApp()` restores Keychain session + navigation
  state, then renders `MainTabView`.
- **Development build**: `.dev` bundle suffix, localhost:8081 endpoints,
  logging enabled (Development.xcconfig).
- **Staging build**: staging endpoints (Staging.xcconfig).
- **Production build**: production endpoints, sample/`#if DEBUG` data stripped.

## Notes

### Broader context

This infrastructure spans the iOS app shell at `mobile-native/iosApp/`:
`iosApp/App/RealityPortalApp.swift` (`@main`),
`iosApp/Core/{AuthManager,DependencyContainer,KMPBridge}.swift`,
the `*.xcconfig` build files, `Info.plist`, and the `*.lproj` localization
bundles. It is the foundation the Story-82.2 navigation layer
(`reality-mobile/navigation`) and all visible screens build on. The Android
counterpart is the KMP `composeApp` shell; the shared KMP module under
`mobile-native/shared/` provides repositories consumed via the bridge.

### Specific (recent)

- Story 82.1 verified complete by static audit (Coverage 82-1). Bundle config,
  xcconfig matrix, Info.plist token expansion, KMP bridge, Keychain storage,
  localization scaffold, and the `@main` entry point are all shipped in source.
- Three non-blocking gaps recorded: tab titles unlocalized, `DependencyContainer`
  defaulting to unauthenticated repositories, and `SsoService` being
  instantiated twice. None block epic-82 functionality; tracked as follow-ups.

## Agent Log

<!-- newest entries on top -->

- 2026-06-17 — agent: created the missing Story-82.1 (SwiftUI Project Setup & App Shell) infrastructure screen-map. Reality-mobile had maps for all 11 visible screens + the 82.2 navigation infra, but no map for the 82.1 project-setup layer. Content grounded in the 2026-05-25 static audit (`docs/superpowers/plans/gap-82-1-swiftui-audit.md`): bundle/xcconfig/Info.plist config, KMP bridge, Keychain, localization, `@main` entry. Carried over the three audit gaps (unlocalized tab titles, unauthenticated default repositories, duplicate `SsoService`).
