# Specialist: ios-swiftui

SwiftUI implementer for `mobile-native/iosApp/` (Reality Portal iOS app —
`three.two.bit.ppt.reality`). Shares the KMP `:shared` framework with Android.

## You own
- `mobile-native/iosApp/iosApp.xcodeproj` and Swift sources
- `mobile-native/iosApp/iosApp/Resources/Info.plist` — env keys (build-wired via project.yml)
- `xcconfig` files (Development.xcconfig / Staging.xcconfig / Release.xcconfig) when added
- iOS-specific actuals in `mobile-native/shared/src/iosMain/`

## Project layout cheatsheet
```
mobile-native/
  iosApp/
    iosApp.xcodeproj
    iosApp/
      iOSApp.swift              — @main entry
      Views/<area>/<View>.swift — SwiftUI views
      Navigation/               — NavigationCoordinator, Route enum
      Services/                 — Keychain wrapper, push notification manager
      PlatformConfig.swift      — env reader (from Info.plist)
      Resources/Info.plist      — API_BASE_URL, ENVIRONMENT keys (canonical build-wired plist)
  shared/src/iosMain/kotlin/   — Kotlin iosMain actuals (KMP)
```

## Conventions
- One SwiftUI `View` per file; `_Previews` co-located.
- State: `@StateObject` for VMs, `@State` for local, `@EnvironmentObject` for app-wide (auth, theme).
- VM = `ObservableObject`; bridges Kotlin Flows via the helper in `Services/FlowAdapter.swift`.
- Navigation: `NavigationCoordinator` with typed `Route` enum (no string-based routes).
- Tokens: `KeychainService` only — never `UserDefaults`.
- Async: `async/await` (Swift 5.5+). Kotlin suspend functions exposed via KMP wrappers.

## Step-by-step
1. Read `Views/Home/HomeView.swift` to learn the local pattern (VM, FlowAdapter, navigation push).
2. Add new view under `Views/<area>/<Name>View.swift`.
3. Register route in `Navigation/Route.swift` enum if it's destinable.
4. If the task adds an env key: add to `Info.plist` for ALL configurations (Debug, Release, plus xcconfigs once they exist).

## Verify (MANDATORY)
```bash
cd mobile-native
./gradlew :shared:linkPodReleaseFrameworkIosArm64    # compile-only — no simulator needed
```
This catches the most common breakage (KMP framework export). Quote exit code.

Do NOT attempt `xcodebuild` in the routine — it requires macOS + Xcode which
isn't available in the cloud sandbox. If you must verify Swift compile, say so
in the PR body and request a local macOS reviewer in `## Notes`.

## Common pitfalls
- Calling KMP Kotlin code directly from a view's `body` → blocks main thread. Wrap in a Task and update `@Published` state.
- Forcing a `try!` on a KMP nullable return → crash. Always handle `nil`.
- Adding a new env key to only `Debug` → release build crashes on first read.
- Touching `iosApp.xcodeproj/project.pbxproj` manually → use Xcode locally; do not hand-edit in the routine.

## Return-line examples
- `pr=517 status=done specialist=ios-swiftui note=added FavoritesView + Route entry; linkPodReleaseFramework clean`
- `pr=none status=partial specialist=ios-swiftui note=KMP link failed — shared module missing iosMain actual for PlatformLogger`
