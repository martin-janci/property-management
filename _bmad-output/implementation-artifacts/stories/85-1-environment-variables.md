# Story 85.1: Environment Variable Setup

Status: done

## Story

As a **mobile developer**,
I want to **configure environment-specific API URLs**,
So that **mobile apps connect to the correct backend for each environment**.

## Acceptance Criteria

1. **AC-1: React Native Environment Config**
   - Given the React Native mobile app
   - When building for different environments
   - Then the correct API URL is used
   - And environment variables are properly loaded

2. **AC-2: KMP Environment Config**
   - Given the Kotlin Multiplatform app
   - When building for different environments
   - Then the correct API URL is configured
   - And it works for both Android and iOS

3. **AC-3: Development Environment**
   - Given I am developing locally
   - When running the app
   - Then it connects to localhost or dev server
   - And hot reload works correctly

4. **AC-4: Staging Environment**
   - Given I am testing on staging
   - When building a staging version
   - Then it connects to staging API
   - And the app is distinguishable from production

5. **AC-5: Production Environment**
   - Given I am building for production
   - When creating a release build
   - Then it connects to production API
   - And all debug features are disabled

## Tasks / Subtasks

- [x] Task 1: Configure React Native Environment (AC: 1, 3, 4, 5)
  - [x] 1.1 Update `/frontend/apps/mobile/src/config/api.ts:29`
  - [x] 1.2 Install react-native-config package
  - [x] 1.3 Create .env.development file
  - [x] 1.4 Create .env.staging file
  - [x] 1.5 Create .env.production file
  - [x] 1.6 Configure Metro bundler for env files

- [x] Task 2: Configure Android Build Variants (AC: 1, 4, 5)
  - [x] 2.1 Add productFlavors to build.gradle
  - [x] 2.2 Create development, staging, production flavors
  - [x] 2.3 Set applicationIdSuffix per flavor
  - [x] 2.4 Configure BuildConfig fields

- [x] Task 3: Configure iOS Schemes (AC: 1, 4, 5)
  - [x] 3.1 Create Development scheme
  - [x] 3.2 Create Staging scheme
  - [x] 3.3 Create Production scheme
  - [x] 3.4 Add xcconfig files per environment

- [x] Task 4: Configure KMP Environment (AC: 2, 3, 4, 5)
  - [x] 4.1 Update `/mobile-native/shared/src/commonMain/kotlin/.../api/ApiConfig.kt`
  - [x] 4.2 Create expect/actual for platform-specific config
  - [x] 4.3 Android: Read from BuildConfig
  - [x] 4.4 iOS: Read from Info.plist (keys added to Info.plist)

- [x] Task 5: Document Environment Setup (AC: 1, 2, 3, 4, 5)
  - [x] 5.1 Create environment setup documentation
  - [x] 5.2 Document local development setup
  - [x] 5.3 Document CI/CD configuration
  - [x] 5.4 Add troubleshooting guide

## Dev Notes

### Architecture Requirements
- Environment-specific configuration
- No hardcoded API URLs
- Build-time configuration injection
- Clear separation of environments

### Technical Specifications
- React Native: react-native-config
- KMP: expect/actual with BuildConfig/Info.plist
- Environments: development, staging, production

### Existing TODO Reference
```typescript
// frontend/apps/mobile/src/config/api.ts:29
// TODO: Configure environment-specific API URL
// - Read from environment variables
// - Support development, staging, production
```

### React Native Environment Files

```bash
# .env.development
API_BASE_URL=http://localhost:8080
WS_BASE_URL=ws://localhost:8080
ENVIRONMENT=development
DEBUG_MODE=true

# .env.staging
API_BASE_URL=https://staging-api.ppt.example.com
WS_BASE_URL=wss://staging-api.ppt.example.com
ENVIRONMENT=staging
DEBUG_MODE=true

# .env.production
API_BASE_URL=https://api.ppt.example.com
WS_BASE_URL=wss://api.ppt.example.com
ENVIRONMENT=production
DEBUG_MODE=false
```

### React Native Config Usage
```typescript
// src/config/api.ts
import Config from 'react-native-config';

export const apiConfig = {
  baseUrl: Config.API_BASE_URL || 'http://localhost:8080',
  wsUrl: Config.WS_BASE_URL || 'ws://localhost:8080',
  environment: Config.ENVIRONMENT || 'development',
  debugMode: Config.DEBUG_MODE === 'true',
};

export const isProduction = apiConfig.environment === 'production';
export const isDevelopment = apiConfig.environment === 'development';
```

### Android build.gradle Configuration
```groovy
android {
    flavorDimensions "environment"

    productFlavors {
        development {
            dimension "environment"
            applicationIdSuffix ".dev"
            versionNameSuffix "-dev"
            resValue "string", "app_name", "PPT (Dev)"
            buildConfigField "String", "API_BASE_URL", '"http://10.0.2.2:8080"'
        }
        staging {
            dimension "environment"
            applicationIdSuffix ".staging"
            versionNameSuffix "-staging"
            resValue "string", "app_name", "PPT (Staging)"
            buildConfigField "String", "API_BASE_URL", '"https://staging-api.ppt.example.com"'
        }
        production {
            dimension "environment"
            resValue "string", "app_name", "PPT"
            buildConfigField "String", "API_BASE_URL", '"https://api.ppt.example.com"'
        }
    }
}
```

### iOS xcconfig Files
```
// Development.xcconfig
API_BASE_URL = http:/$()/localhost:8080
PRODUCT_BUNDLE_IDENTIFIER = three.two.bit.ppt.management.dev
PRODUCT_NAME = PPT (Dev)

// Staging.xcconfig
API_BASE_URL = https:/$()/staging-api.ppt.example.com
PRODUCT_BUNDLE_IDENTIFIER = three.two.bit.ppt.management.staging
PRODUCT_NAME = PPT (Staging)

// Production.xcconfig
API_BASE_URL = https:/$()/api.ppt.example.com
PRODUCT_BUNDLE_IDENTIFIER = three.two.bit.ppt.management
PRODUCT_NAME = PPT
```

### KMP Environment Configuration
```kotlin
// shared/src/commonMain/kotlin/.../api/ApiConfig.kt
expect object ApiConfig {
    val baseUrl: String
    val wsUrl: String
    val environment: String
    val isDebug: Boolean
}

// shared/src/androidMain/kotlin/.../api/ApiConfig.kt
actual object ApiConfig {
    actual val baseUrl: String = BuildConfig.API_BASE_URL
    actual val wsUrl: String = baseUrl.replace("http", "ws")
    actual val environment: String = BuildConfig.BUILD_TYPE
    actual val isDebug: Boolean = BuildConfig.DEBUG
}

// shared/src/iosMain/kotlin/.../api/ApiConfig.kt
actual object ApiConfig {
    actual val baseUrl: String = NSBundle.mainBundle.objectForInfoDictionaryKey("API_BASE_URL") as? String
        ?: "https://api.ppt.example.com"
    actual val wsUrl: String = baseUrl.replace("http", "ws")
    actual val environment: String = NSBundle.mainBundle.objectForInfoDictionaryKey("ENVIRONMENT") as? String
        ?: "production"
    actual val isDebug: Boolean = environment != "production"
}
```

### File List (to create/modify)

**Create:**
- `/frontend/apps/mobile/.env.development`
- `/frontend/apps/mobile/.env.staging`
- `/frontend/apps/mobile/.env.production`
- `/frontend/apps/mobile/ios/Development.xcconfig`
- `/frontend/apps/mobile/ios/Staging.xcconfig`
- `/frontend/apps/mobile/ios/Production.xcconfig`
- `/docs/mobile-environment-setup.md`

**Modify:**
- `/frontend/apps/mobile/src/config/api.ts` - Use react-native-config
- `/frontend/apps/mobile/android/app/build.gradle` - Add flavors
- `/frontend/apps/mobile/ios/mobile.xcodeproj/project.pbxproj` - Add schemes
- `/mobile-native/shared/src/commonMain/kotlin/.../api/ApiConfig.kt` - Implement

### CI/CD Configuration
```yaml
# .github/workflows/mobile-build.yml
jobs:
  build-android:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        flavor: [development, staging, production]
    steps:
      - uses: actions/checkout@v4
      - name: Build Android
        run: |
          cd frontend/apps/mobile/android
          ./gradlew assemble${FLAVOR}Release
        env:
          FLAVOR: ${{ matrix.flavor }}

  build-ios:
    runs-on: macos-latest
    strategy:
      matrix:
        scheme: [Development, Staging, Production]
    steps:
      - uses: actions/checkout@v4
      - name: Build iOS
        run: |
          xcodebuild -workspace mobile.xcworkspace \
            -scheme ${{ matrix.scheme }} \
            -configuration Release \
            archive
```

### Dependencies
- react-native-config package for React Native
- None for KMP (uses platform-specific config)

### References
- [Source: frontend/apps/mobile/src/config/api.ts:29]
- [react-native-config Documentation]
