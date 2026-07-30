# Mobile Native - CLAUDE.md

> **Parent:** See root `CLAUDE.md` for namespace and architecture.

## Overview

Kotlin Multiplatform project for Reality Portal mobile apps.

**Package ID:** `three.two.bit.ppt.reality`

## Tech Stack

| Component | Version |
|-----------|---------|
| Kotlin | 2.3.21 |
| Ktor | 3.5.0 |
| Compose BOM | 2026.05.01 |
| AGP | 9.2.1 |
| Gradle | 9.1.0 |
| KSP | 2.3.21-2.0.4 |
| Kotlinx Serialization | 1.7.3 |
| Kotlinx Coroutines | 1.9.0 |

> Source of truth: `mobile-native/gradle/libs.versions.toml`. Update both this table and the catalog together.

> **Note:** The `:shared` module uses the AGP 9 `com.android.kotlin.multiplatform.library`
> plugin (`kotlin { androidLibrary { } }` DSL). This plugin is variant-agnostic — it has no
> product flavors and no `BuildConfig` generation. Android environment config is read at
> runtime from the `:androidApp` `BuildConfig` via reflection in `PlatformConfig` (androidMain).

## Targets

| Platform | App |
|----------|-----|
| Android | Reality Portal (three.two.bit.ppt.reality) |
| iOS | Reality Portal (three.two.bit.ppt.reality) |

## Quick Commands

```bash
# Code quality (Spotless)
./gradlew spotlessCheck    # Verify formatting
./gradlew spotlessApply    # Auto-fix formatting

# Build shared module
./gradlew :shared:build

# Build Android app
./gradlew :androidApp:assembleDebug

# Build release APK
./gradlew :androidApp:assembleRelease

# Build iOS framework
./gradlew :shared:linkDebugFrameworkIosSimulatorArm64

# Generate + build the iOS app (macOS only; requires `brew install xcodegen`)
cd iosApp && xcodegen generate          # materialise iosApp.xcodeproj from project.yml
../../scripts/build-ios.sh development   # KMP framework + xcodebuild (Dev scheme)

# Run tests
./gradlew :shared:allTests

# Clean build
./gradlew clean
```

## Code Quality

Uses **Spotless** with **ktfmt** (Kotlin lang style):

- Runs automatically in CI before build
- Auto-fix: `./gradlew spotlessApply`
- Check only: `./gradlew spotlessCheck`

## Project Structure

```
mobile-native/
├── build.gradle.kts        # Root build config
├── settings.gradle.kts     # Project settings
├── gradle.properties       # Gradle config
├── gradle/
│   ├── libs.versions.toml  # Version catalog
│   └── wrapper/            # Gradle wrapper
├── shared/                 # KMP shared code
│   ├── build.gradle.kts
│   └── src/
│       ├── commonMain/     # Shared Kotlin
│       │   └── kotlin/three/two/bit/ppt/reality/
│       │       ├── api/    # API client
│       │       └── models/ # Data models
│       ├── androidMain/    # Android-specific
│       └── iosMain/        # iOS-specific
├── androidApp/             # Android application
│   ├── build.gradle.kts
│   ├── proguard-rules.pro
│   └── src/main/java/three/two/bit/ppt/reality/
└── iosApp/                 # iOS application (SwiftUI)
    ├── project.yml         # XcodeGen manifest — single source of truth for iosApp.xcodeproj
    ├── Configurations/     # Base/Development/Staging/Production xcconfig
    ├── xcschemes/          # RealityPortal-{Dev,Staging,Prod} schemes
    └── iosApp/             # Swift sources (App/, Core/{DI,Services,Navigation}, Features/)
```

> **iOS project is generated, not committed.** `iosApp.xcodeproj` is produced by
> `xcodegen generate` from `iosApp/project.yml` and is git-ignored. The manifest
> declares the three build configs (each wired to its `Configurations/*.xcconfig`),
> the three schemes, the `iosApp` + `iosAppTests` targets, and a pre-build step
> that links the `:shared` KMP framework (`link<Build>FrameworkIos<Arch>`) before
> the Swift compile. The SwiftUI entry point is `iosApp/App/RealityPortalApp.swift`;
> DI lives in `iosApp/Core/DI/DependencyContainer.swift`; Ktor networking is
> bootstrapped by the shared module's `HttpClientProvider` (ktor-client-darwin).

## Version Catalog

Dependencies are centralized in `gradle/libs.versions.toml`:

```kotlin
// Usage in build.gradle.kts
implementation(libs.ktor.client.core)
implementation(libs.kotlinx.serialization.json)
```

## API Client Generation

```bash
openapi-generator generate \
  -i docs/api/generated/by-service/reality-server.yaml \
  -g kotlin \
  -o shared/src/commonMain/kotlin/three/two/bit/ppt/reality/api \
  --additional-properties=library=multiplatform
```

## Dependencies

- **Ktor Client** - HTTP networking (v3.x with content negotiation)
- **Kotlin Serialization** - JSON parsing with @SerialName
- **Kotlin Coroutines** - Async operations
- **Kotlinx DateTime** - Date/time handling
- **Jetpack Compose** - Android UI (Material3)

## Platform-Specific Engines

| Platform | Ktor Engine |
|----------|-------------|
| Android | ktor-client-android |
| iOS | ktor-client-darwin |

## SSO deep-link CSRF contract

The SSO callback (`reality://sso?token=…&state=…`) is delivered through an **exported** deep-link
intent-filter, so any app/browser/notification can hand it to us. **Both platforms MUST verify a
per-flow `state` nonce before validating the token** — skipping it allows session fixation / account
takeover (an attacker's token silently signs the victim in).

- **Android** — `SsoStateStore` (commonMain): `SsoInitiation.begin()` mints the nonce and builds the
  outbound `propertymanagement://sso?callback=reality://sso&state=<nonce>` hop (wired to the
  `LoginScreen` "Sign in via PM App" button in `Navigation.kt`); `MainActivity.handleDeepLink` calls
  `consume(target.state)` and validates the token only on a match. Default-reject: an unsolicited
  callback with no pending flow is dropped.
- **iOS** — `AuthManager.beginSsoFlow()` / `consumeSsoState(_:)` (owns its own nonce; does not use
  the KMP store).

`DeepLinkTarget.Sso` carries `state: String?`; a `null`/mismatched state is rejected. Nonces are
single-use (cleared on every `consume`).

## Screen-Map integration

When implementing or modifying a screen in this KMP app:

1. Identify the screen-map id under the `reality/` product (mobile-native screens share screen-maps with reality-web).
2. **Before coding**: run `/screens edit reality/<id>` to load full context.
3. **After coding**: update `implementations.mobile-native` block (`buildStatus` / `apiStatus` / `redesignStatus`).
4. Add an Agent Log entry: `<date> — agent: <what changed>`.
5. Run `/screens validate`.
6. New screens: `/screens update` then `/screens init --add "<Screen Name>"`.
