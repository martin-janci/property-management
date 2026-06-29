---
id: reality-mobile/env-config
name: Environment Variable Setup (KMP + iOS + RN)
product: reality-mobile
sitemapRefs: {}
implementations:
  mobile-native:
    component: ApiConfig / expect-actual PlatformConfig (commonMain / androidMain / iosMain)
    route: Cross-cutting config layer — no visible route
    buildStatus: shipped
    redesignStatus: n/a
    apiStatus: n/a
  ios-swiftui:
    component: Configurations/{Base,Development,Staging,Production}.xcconfig + Info.plist token expansion
    route: Build settings — no visible route
    buildStatus: shipped
    redesignStatus: n/a
    apiStatus: n/a
endpoints: []
relatedScreens:
  - id: reality-mobile/project-setup
    rel: sibling
  - id: reality-mobile/navigation
    rel: sibling
sharedComponents: []
diagrams: []
useCases: []
epics:
  - "85"
designSources: []
owner: reality-frontend
lastReview: 2026-06-28
---

## Functionality Checklist

Cross-cutting environment-variable / configuration infrastructure for the Reality
Portal mobile apps (Epic 85, Story 85.1). This is **not a visible "screen"** — it
documents how `API_BASE_URL`, `ENVIRONMENT`, and `ENABLE_LOGGING` flow into the
KMP shared module, the iOS app, and the React Native app per build environment
(development / staging / production). It is the sibling of
`reality-mobile/project-setup` (Story 82.1 app-shell) and the foundation every
networked screen's base URL resolves through.

- [x] [m] **KMP shared config object.** `ApiConfig` (commonMain) exposes
  `baseUrl`, `wsUrl` (derived by swapping `http`→`ws`), `environment`,
  `isDebug`, `enableLogging`, and `requireBaseUrl()`. It delegates every value
  to an `expect object PlatformConfig`, so no endpoint is hardcoded in shared
  code (Epic 48 code-review fix carried into Story 85.1).
- [x] [m] **Android actual.** `androidMain/PlatformConfig` reads
  `API_BASE_URL` / `ENVIRONMENT` / `ENABLE_LOGGING` / `DEBUG` from the
  `:androidApp` generated `BuildConfig` via reflection
  (`three.two.bit.ppt.reality.BuildConfig`). The AGP-9
  `com.android.kotlin.multiplatform.library` plugin is variant-agnostic and no
  longer emits `BuildConfig` for `:shared`, so the values live in the app
  module's product flavors (`buildConfigField`) and are read at runtime with
  safe defaults (production base URL, `production` env).
- [x] [m] **iOS actual.** `iosMain/PlatformConfig` reads the same keys from
  `Info.plist` via `NSBundle.mainBundle.objectForInfoDictionaryKey(...)`, with
  production fallbacks. `isDebug` is derived as `environment != "production"`.
- [x] [m] **iOS xcconfig environment matrix.** Four files —
  `Base.xcconfig` (shared bundle ID / product / arch settings),
  `Development.xcconfig` (`http://localhost:8081`, `ENVIRONMENT=development`,
  `ENABLE_LOGGING=true`, `.dev` bundle suffix), `Staging.xcconfig`, and
  `Production.xcconfig` — drive the per-environment `API_BASE_URL` /
  `ENVIRONMENT` / `ENABLE_LOGGING` values that Info.plist expands as `$(VAR)`
  and `PlatformConfig` then reads. Wired to the `RealityPortal-{Dev,Staging,Prod}`
  xcschemes.
- [x] [m] **React Native (Expo) env loading.** `frontend/apps/mobile/app.config.ts`
  selects `.env.{development,staging,production}` from `APP_ENV` (defaulting by
  `__DEV__` / `NODE_ENV`), loads it via `dotenv`, and injects the parsed values
  into the Expo `extra` block (read through `expo-constants`) plus
  `ios.infoPlist` and `android`. `EXPO_PUBLIC_*` vars are surfaced by Metro
  automatically. All four `.env.*` files (`.env.development`, `.env.staging`,
  `.env.production`, `.env.example`) are present and gitignored where secret.
- [x] [o] **Safe defaults everywhere.** Every reader (KMP Android reflection,
  iOS Info.plist, RN dotenv) falls back to the production base URL / `production`
  environment when a key is missing, so a misconfigured build fails closed onto
  prod config rather than a blank/localhost URL.

## States

- **Development build**: localhost endpoints (`:8081` iOS, `10.0.2.2:8081`
  Android emulator), logging on, `.dev` bundle suffix / DEV-badged RN icon.
- **Staging build**: staging endpoints, logging on, staging bundle/scheme.
- **Production build**: production endpoints, logging off by default,
  release bundle/scheme.

## Notes

### Broader context

This config layer spans three trees:
- KMP shared: `mobile-native/shared/src/{commonMain,androidMain,iosMain}/kotlin/three/two/bit/ppt/reality/api/{ApiConfig,PlatformConfig}.kt`.
- iOS: `mobile-native/iosApp/Configurations/*.xcconfig`, `Info.plist`, and the
  `mobile-native/iosApp/xcschemes/RealityPortal-{Dev,Staging,Prod}.xcscheme` files.
- React Native (PPT management app): `frontend/apps/mobile/app.config.ts` +
  `.env.{development,staging,production,example}` and `metro.config.js`.

The xcconfig matrix is shared with Story 85.2 (Build Configuration by
Environment); the source files carry both Story 85.1 (env-var values) and
Story 85.2 (build settings) tags. Bundle/signing/app-icon-variant concerns
belong to 85.2 and are out of scope for this map.

### Specific (recent)

- Story 85.1 verified complete by source audit (Coverage `85-1-environment-variables`,
  status `done`, confidence `high`). KMP `ApiConfig` + expect/actual
  `PlatformConfig`, the iOS xcconfig→Info.plist→`NSBundle` chain, and the RN
  Expo `app.config.ts` `.env` loader are all shipped on `dev`. No hardcoded
  endpoints remain in shared code.
- This map closes the only recorded gap for the story — "no screen-map (orphan
  epic)" — by giving Epic 85's env-config infrastructure the same kind of
  non-visible infra screen-map that Story 82.1 has (`reality-mobile/project-setup`).

## Agent Log

<!-- newest entries on top -->

- 2026-06-28 — agent: created the missing Story-85.1 (Environment Variable Setup) infrastructure screen-map. Reality-mobile had the 82.1 project-setup infra map but no map for the Epic-85 env-config layer (the coverage row flagged "no screen-map (orphan epic)"). Content grounded in source on dev: KMP `ApiConfig`/`PlatformConfig` (commonMain + android/ios actuals), the iOS `*.xcconfig`→Info.plist→`NSBundle` chain, and the RN Expo `app.config.ts` `.env` loader. Story code already shipped; this map is the status-reconciliation deliverable. buildStatus shipped, redesign/api n/a (non-visible config layer).
