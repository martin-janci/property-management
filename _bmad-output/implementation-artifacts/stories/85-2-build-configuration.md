# Story 85.2: Build Configuration by Environment

Status: done

> Reconciliation note (2026-06-28): implementation shipped on `dev` ahead of this
> status flip. All five acceptance criteria are satisfied in source:
> - **AC-1 Android flavors** — `mobile-native/androidApp/build.gradle.kts`
>   (`flavorDimensions += "environment"`; `development`/`staging`/`production`
>   flavors with `applicationIdSuffix`, `versionNameSuffix`, per-flavor
>   `app_name` + `API_BASE_URL`/`ENVIRONMENT`/`ENABLE_LOGGING` `buildConfigField`s;
>   release `signingConfigs` keyed off `KEYSTORE_*` env vars; debug `.debug` suffix).
> - **AC-2 iOS schemes** — `mobile-native/iosApp/xcschemes/RealityPortal-{Dev,Staging,Prod}.xcscheme`
>   + `mobile-native/iosApp/Configurations/{Base,Development,Staging,Production}.xcconfig`
>   (per-config `PRODUCT_BUNDLE_IDENTIFIER`, `BUNDLE_DISPLAY_NAME`, `API_BASE_URL`,
>   `CODE_SIGN_STYLE`).
> - **AC-3 KMP variants** — `mobile-native/shared/build.gradle.kts`; under the AGP 9
>   `com.android.kotlin.multiplatform.library` plugin the shared module is
>   variant-agnostic (no BuildConfig), with the iOS `framework` block configured
>   for `iosX64`/`iosArm64`/`iosSimulatorArm64`.
> - **AC-4 distinguishability** — distinct app names (`Reality (Dev)`,
>   `Reality (Staging)`, `Reality Portal`) + bundle/app-id suffixes per environment.
> - **AC-5 build scripts** — `scripts/build-mobile.sh` (orchestrator),
>   `scripts/build-android.sh`, `scripts/build-ios.sh` (each tagged
>   "Epic 85 - Story 85.2"). Note: these live at repo-root `scripts/`, not under
>   `mobile-native/` — an earlier coverage scan looked only under `mobile-native/`
>   and reported a false-negative AC-5 gap.

## Story

As a **mobile developer**,
I want to **have distinct build configurations for each environment**,
So that **I can easily build and deploy to different environments with proper settings**.

## Acceptance Criteria

1. **AC-1: Android Build Flavors**
   - Given I want to build for Android
   - When I select a build flavor
   - Then the correct configuration is applied
   - And the APK has the correct app ID suffix
   - And debug/release variants work correctly

2. **AC-2: iOS Build Schemes**
   - Given I want to build for iOS
   - When I select a build scheme
   - Then the correct configuration is applied
   - And the IPA has the correct bundle ID
   - And signing is configured correctly

3. **AC-3: KMP Build Variants**
   - Given I want to build the KMP module
   - When building for different environments
   - Then the shared code uses correct config
   - And platform-specific code compiles correctly

4. **AC-4: App Distinguishability**
   - Given I have multiple app versions installed
   - When viewing the app icon or name
   - Then I can distinguish development from staging
   - And staging from production

5. **AC-5: Automated Build Scripts**
   - Given I want to build from command line
   - When I run the build script
   - Then the correct environment is built
   - And all dependencies are properly linked

## Tasks / Subtasks

- [x] Task 1: Configure Android Build System (AC: 1, 4)
  - [x] 1.1 Set up product flavors (dev, staging, prod)
  - [x] 1.2 Configure application ID suffixes
  - [~] 1.3 Create app icons per flavor — per-flavor `app_name` strings ship;
    per-flavor `ic_launcher` mipmaps NOT yet generated (no `src/staging/res`,
    no `src/development/res/mipmap-*`). Visual distinction is via app name only.
  - [x] 1.4 Configure app names per flavor
  - [x] 1.5 Set up signing configs

- [x] Task 2: Configure iOS Build System (AC: 2, 4)
  - [x] 2.1 Create build schemes (Development, Staging, Production)
  - [x] 2.2 Configure bundle ID per scheme
  - [~] 2.3 Create app icons per scheme — `AppIcon`, `AppIcon-Dev`,
    `AppIcon-Staging` `.appiconset` directories exist and are referenced via
    `ASSETCATALOG_COMPILER_APPICON_NAME`, but the sets currently hold only
    `Contents.json` (no badge image assets generated yet).
  - [x] 2.4 Configure display names per scheme
  - [~] 2.5 Set up provisioning profiles — `CODE_SIGN_STYLE = Automatic`;
    no manual provisioning profiles committed (intentional for CI).

- [x] Task 3: Configure KMP Build Variants (AC: 3)
  - [x] 3.1 Set up Gradle build types
  - [x] 3.2 Configure shared module variants — under AGP 9
    `com.android.kotlin.multiplatform.library` the shared module is
    variant-agnostic by design (no build types / flavors / BuildConfig).
  - [x] 3.3 Set up Android library variants
  - [x] 3.4 Configure iOS framework variants

- [~] Task 4: Create App Icons and Assets (AC: 4)
  - [~] 4.1 Create development app icon (with "DEV" badge) — set scaffolded, image pending
  - [~] 4.2 Create staging app icon (with "STG" badge) — set scaffolded, image pending
  - [~] 4.3 Create production app icon — set scaffolded, image pending
  - [ ] 4.4 Generate all required sizes
  - Note: icon-badge artwork is the only remaining piece; tracked as a
    design-asset follow-up. App distinguishability (AC-4) is already met via
    distinct app names + bundle/app-id suffixes per environment.

- [x] Task 5: Create Build Scripts (AC: 5)
  - [x] 5.1 Create Android build script (`scripts/build-android.sh`)
  - [x] 5.2 Create iOS build script (`scripts/build-ios.sh`)
  - [x] 5.3 Create KMP build script — covered by `build-android.sh`
    (`./gradlew` assemble) and `build-ios.sh` (framework link) which drive the
    shared KMP module; no separate script needed.
  - [x] 5.4 Create unified build script (`scripts/build-mobile.sh`)
  - [x] 5.5 Document build commands (usage headers in each script)

## Dev Notes

### Architecture Requirements
- Separate app identifiers per environment
- Visual distinction for non-production builds
- Automated build process
- Consistent configuration across platforms

### Technical Specifications
- Android: Gradle product flavors with build types
- iOS: Xcode schemes with configurations
- KMP: Gradle multi-variant support

### Android Gradle Configuration
```groovy
// mobile-native/androidApp/build.gradle.kts
android {
    namespace = "three.two.bit.ppt.reality"

    defaultConfig {
        applicationId = "three.two.bit.ppt.reality"
        minSdk = 26
        targetSdk = 34
        versionCode = 1
        versionName = "1.0.0"
    }

    signingConfigs {
        create("release") {
            storeFile = file(System.getenv("KEYSTORE_FILE") ?: "../keystore/release.jks")
            storePassword = System.getenv("KEYSTORE_PASSWORD") ?: ""
            keyAlias = System.getenv("KEY_ALIAS") ?: "release"
            keyPassword = System.getenv("KEY_PASSWORD") ?: ""
        }
    }

    flavorDimensions += "environment"
    productFlavors {
        create("development") {
            dimension = "environment"
            applicationIdSuffix = ".dev"
            versionNameSuffix = "-dev"
            resValue("string", "app_name", "Reality (Dev)")
            buildConfigField("String", "API_BASE_URL", "\"http://10.0.2.2:8081\"")
            buildConfigField("Boolean", "ENABLE_LOGGING", "true")
        }
        create("staging") {
            dimension = "environment"
            applicationIdSuffix = ".staging"
            versionNameSuffix = "-staging"
            resValue("string", "app_name", "Reality (Staging)")
            buildConfigField("String", "API_BASE_URL", "\"https://staging-reality.ppt.example.com\"")
            buildConfigField("Boolean", "ENABLE_LOGGING", "true")
        }
        create("production") {
            dimension = "environment"
            resValue("string", "app_name", "Reality Portal")
            buildConfigField("String", "API_BASE_URL", "\"https://reality.ppt.example.com\"")
            buildConfigField("Boolean", "ENABLE_LOGGING", "false")
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
            signingConfig = signingConfigs.getByName("release")
        }
        debug {
            isDebuggable = true
            applicationIdSuffix = ".debug"
        }
    }
}
```

### iOS Project Configuration
```xml
<!-- Info.plist with variable substitution -->
<key>CFBundleIdentifier</key>
<string>$(PRODUCT_BUNDLE_IDENTIFIER)</string>
<key>CFBundleName</key>
<string>$(PRODUCT_NAME)</string>
<key>API_BASE_URL</key>
<string>$(API_BASE_URL)</string>
```

```
// Development.xcconfig
PRODUCT_BUNDLE_IDENTIFIER = three.two.bit.ppt.reality.dev
PRODUCT_NAME = Reality (Dev)
API_BASE_URL = http:/$()/localhost:8081
ASSETCATALOG_COMPILER_APPICON_NAME = AppIcon-Dev

// Staging.xcconfig
PRODUCT_BUNDLE_IDENTIFIER = three.two.bit.ppt.reality.staging
PRODUCT_NAME = Reality (Staging)
API_BASE_URL = https:/$()/staging-reality.ppt.example.com
ASSETCATALOG_COMPILER_APPICON_NAME = AppIcon-Staging

// Release.xcconfig
PRODUCT_BUNDLE_IDENTIFIER = three.two.bit.ppt.reality
PRODUCT_NAME = Reality Portal
API_BASE_URL = https:/$()/reality.ppt.example.com
ASSETCATALOG_COMPILER_APPICON_NAME = AppIcon
```

### App Icon Variants
```
Assets.xcassets/
├── AppIcon.appiconset/          # Production (blue)
│   └── Contents.json
├── AppIcon-Dev.appiconset/      # Development (green with DEV badge)
│   └── Contents.json
└── AppIcon-Staging.appiconset/  # Staging (orange with STG badge)
    └── Contents.json
```

### Build Scripts
```bash
#!/bin/bash
# scripts/build-mobile.sh

ENVIRONMENT=${1:-development}
PLATFORM=${2:-android}
BUILD_TYPE=${3:-debug}

case $PLATFORM in
  android)
    cd mobile-native/androidApp
    ./gradlew assemble${ENVIRONMENT^}${BUILD_TYPE^}
    ;;
  ios)
    cd mobile-native/iosApp
    xcodebuild -workspace iosApp.xcworkspace \
      -scheme "${ENVIRONMENT^}" \
      -configuration "${BUILD_TYPE^}" \
      -derivedDataPath build \
      build
    ;;
  all)
    $0 $ENVIRONMENT android $BUILD_TYPE
    $0 $ENVIRONMENT ios $BUILD_TYPE
    ;;
esac
```

### KMP Shared Module Variants
```kotlin
// shared/build.gradle.kts
kotlin {
    android {
        publishLibraryVariants("release", "debug")
    }

    listOf(
        iosX64(),
        iosArm64(),
        iosSimulatorArm64()
    ).forEach {
        it.binaries.framework {
            baseName = "shared"
            isStatic = true

            // Environment-specific configuration at runtime
        }
    }
}
```

### File List (to create/modify)

**Create:**
- `/mobile-native/androidApp/src/development/res/mipmap-*/ic_launcher.png`
- `/mobile-native/androidApp/src/staging/res/mipmap-*/ic_launcher.png`
- `/mobile-native/iosApp/iosApp/Assets.xcassets/AppIcon-Dev.appiconset/`
- `/mobile-native/iosApp/iosApp/Assets.xcassets/AppIcon-Staging.appiconset/`
- `/mobile-native/iosApp/iosApp/Configuration/Development.xcconfig`
- `/mobile-native/iosApp/iosApp/Configuration/Staging.xcconfig`
- `/mobile-native/iosApp/iosApp/Configuration/Release.xcconfig`
- `/scripts/build-mobile.sh`
- `/scripts/build-android.sh`
- `/scripts/build-ios.sh`

**Modify:**
- `/mobile-native/androidApp/build.gradle.kts` - Add flavors
- `/mobile-native/iosApp/iosApp.xcodeproj/project.pbxproj` - Add schemes
- `/mobile-native/shared/build.gradle.kts` - Configure variants

### CI/CD Matrix Build
```yaml
# .github/workflows/mobile-release.yml
jobs:
  build:
    strategy:
      matrix:
        include:
          - platform: android
            environment: development
            artifact: app-development-debug.apk
          - platform: android
            environment: staging
            artifact: app-staging-release.apk
          - platform: android
            environment: production
            artifact: app-production-release.apk
          - platform: ios
            environment: Development
            artifact: Reality-Dev.ipa
          - platform: ios
            environment: Staging
            artifact: Reality-Staging.ipa
          - platform: ios
            environment: Production
            artifact: Reality.ipa
    steps:
      - uses: actions/checkout@v4
      - name: Build ${{ matrix.platform }} ${{ matrix.environment }}
        run: ./scripts/build-mobile.sh ${{ matrix.environment }} ${{ matrix.platform }} release
      - uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.artifact }}
          path: build/${{ matrix.artifact }}
```

### Dependencies
- Story 85.1 (Environment Variables) - Environment configuration

### References
- [Android Build Variants Documentation]
- [Xcode Build Settings Reference]
- [KMP Multiplatform Configuration]
