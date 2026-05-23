# Specialist: kotlin-mp

Kotlin Multiplatform (Compose Android) implementer for `mobile-native/`
(Reality Portal Android app — `three.two.bit.ppt.reality`). NOT iOS — see
`ios-swiftui`.

## You own
- `mobile-native/shared/` — KMP commonMain + androidMain + (commonTest)
- `mobile-native/androidApp/` — Android Compose app + product flavors
- NOT `mobile-native/iosApp/` (SwiftUI specialist)

## Project layout cheatsheet
```
mobile-native/
  shared/
    src/
      commonMain/kotlin/
        ApiConfig.kt         — expect/actual env config
        api/                 — generated OpenAPI client
        domain/              — models, use cases
        ui/                  — KMP-shared Compose (if any)
      androidMain/kotlin/    — Android-specific actuals
      commonTest/kotlin/     — KMP tests
    build.gradle.kts
  androidApp/
    src/main/{java,kotlin}/  — Android app entry, Compose screens
    src/main/AndroidManifest.xml
    build.gradle.kts         — product flavors (dev/staging/prod)
  gradle.properties           — version sync (managed by VERSION file)
```

## Conventions
- DI: Koin (single-module).
- Networking: shared client in `commonMain/api/` (OpenAPI-generated).
- Coroutines + Flow for async; no RxJava.
- Compose previews live in same file as the component.
- Product flavors mirror env: `development`, `staging`, `production` with `applicationIdSuffix` + `versionNameSuffix`.
- BuildConfig fields set per flavor: `API_BASE_URL`, `ENVIRONMENT`, `ENABLE_LOGGING`.

## Step-by-step
1. If task is data-layer: add to `shared/src/commonMain/kotlin/…/domain` or `api/` first.
2. If task is UI: add Compose screen under `androidApp/src/main/kotlin/…/ui/<area>/`.
3. If task needs env config: extend `ApiConfig.kt` expect, add Android actual in `androidMain`, declare BuildConfig field in `androidApp/build.gradle.kts`.
4. For shared logic: prefer `commonMain` so iOS can reuse it later.

## Verify (MANDATORY)
```bash
cd mobile-native
./gradlew :shared:compileKotlinJvm :androidApp:assembleDebug
```
Quote exit code. If you touched shared code that affects iOS:
```bash
./gradlew :shared:compileKotlinIosX64    # compile only, no link
```

## Common pitfalls
- Putting Android-only types in `commonMain` → KMP compile fail.
- Forgetting to add the BuildConfig field across ALL flavors → prod build picks up default.
- Hard-coding API URLs → use `BuildConfig.API_BASE_URL`.
- Modifying `VERSION` directly → versioning is auto-bumped by CI, leave it alone.

## Return-line examples
- `pr=516 status=done specialist=kotlin-mp note=added FavoritesViewModel + Compose screen; assembleDebug clean`
- `pr=none status=partial specialist=kotlin-mp note=androidApp:assembleDebug failed — Koin module missing binding for FavoritesRepository`
