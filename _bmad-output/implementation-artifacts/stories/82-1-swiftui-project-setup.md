# Story 82.1: SwiftUI Project Setup

Status: pending

> **Note (2026-06-17):** This is a *legacy build* story. It is **implemented in
> code** — see `mobile-native/iosApp/` and `docs/superpowers/plans/gap-82-1-swiftui-audit.md`.
> The `Status: pending` above is stale. Note also that its story number (82.1)
> collides with a *different* story in `_bmad-output/epics-007.md` Epic 82
> ("Remove Broken Direct Login Form"). The authoritative Epic 82 is the
> `epics-007.md` "Mobile Native Completion" epic. Reconciliation:
> `docs/superpowers/plans/epic-82-story-mapping.md`.

## Story

As a **mobile developer**,
I want to **set up the iOS SwiftUI project with KMP integration**,
So that **we can build the Reality Portal iOS app using shared Kotlin code**.

## Acceptance Criteria

1. **AC-1: Xcode Project Structure**
   - Given the mobile-native repository
   - When the iOS project is set up
   - Then a proper Xcode project exists in `iosApp/`
   - And it follows Apple's iOS project conventions
   - And the bundle identifier is `three.two.bit.ppt.reality`

2. **AC-2: KMP Framework Integration**
   - Given the shared KMP module
   - When building the iOS app
   - Then the shared framework is properly linked
   - And Kotlin code is callable from Swift
   - And the app compiles successfully

3. **AC-3: Dependency Management**
   - Given the project dependencies
   - When configured with Swift Package Manager
   - Then all required packages are resolved
   - And the project builds in Xcode

4. **AC-4: App Configuration**
   - Given the iOS app target
   - When building for development
   - Then the app has proper Info.plist configuration
   - And app icons and launch screen are set up
   - And required permissions are declared

5. **AC-5: Build Schemes**
   - Given multiple environments (dev, staging, prod)
   - When selecting a build scheme
   - Then the correct API URL is used
   - And build configurations are separate

## Tasks / Subtasks

- [ ] Task 1: Create Xcode Project (AC: 1, 4)
  - [ ] 1.1 Create new Xcode project in `/mobile-native/iosApp/`
  - [ ] 1.2 Configure bundle identifier: `three.two.bit.ppt.reality`
  - [ ] 1.3 Set deployment target (iOS 15.0+)
  - [ ] 1.4 Configure team and signing
  - [ ] 1.5 Add app icons (1024x1024 source)
  - [ ] 1.6 Configure launch screen storyboard or SwiftUI

- [ ] Task 2: Integrate KMP Shared Module (AC: 2)
  - [ ] 2.1 Configure Gradle for iOS framework export
  - [ ] 2.2 Add XCFramework dependency to Xcode project
  - [ ] 2.3 Create bridge/wrapper for Kotlin types
  - [ ] 2.4 Verify API client is accessible from Swift
  - [ ] 2.5 Test basic Kotlin function calls

- [ ] Task 3: Configure Swift Package Manager (AC: 3)
  - [ ] 3.1 Add Package.swift or use Xcode SPM integration
  - [ ] 3.2 Add Kingfisher for image loading
  - [ ] 3.3 Add KeychainAccess for secure storage
  - [ ] 3.4 Configure package resolution

- [ ] Task 4: Set Up Build Configurations (AC: 5)
  - [ ] 4.1 Create Debug, Release, and Staging configurations
  - [ ] 4.2 Add environment-specific xcconfig files
  - [ ] 4.3 Configure API URL per environment
  - [ ] 4.4 Create build schemes for each environment

- [ ] Task 5: Configure App Permissions (AC: 4)
  - [ ] 5.1 Add location permission for nearby listings
  - [ ] 5.2 Add photo library permission for uploads
  - [ ] 5.3 Add push notification entitlement
  - [ ] 5.4 Configure App Transport Security

## Dev Notes

### Architecture Requirements
- SwiftUI for all UI (no UIKit unless necessary)
- MVVM architecture aligned with Android implementation
- Shared KMP code for API client and domain logic
- Swift for iOS-specific features

### Technical Specifications
- Minimum iOS: 15.0
- Swift version: 5.9+
- Xcode version: 15.0+
- KMP framework: XCFramework format

### Project Structure
```
mobile-native/
├── iosApp/
│   ├── iosApp.xcodeproj/
│   ├── iosApp/
│   │   ├── App/
│   │   │   ├── RealityPortalApp.swift
│   │   │   └── AppDelegate.swift
│   │   ├── Features/
│   │   │   ├── Home/
│   │   │   ├── Search/
│   │   │   ├── Favorites/
│   │   │   ├── Inquiries/
│   │   │   └── Account/
│   │   ├── Core/
│   │   │   ├── DI/
│   │   │   ├── Navigation/
│   │   │   └── Extensions/
│   │   ├── Resources/
│   │   │   ├── Assets.xcassets
│   │   │   ├── Localizable.strings
│   │   │   └── Info.plist
│   │   └── Preview Content/
│   └── iosAppTests/
├── shared/           # KMP shared module
└── androidApp/       # Reference implementation
```

### Gradle Configuration for iOS
```kotlin
// shared/build.gradle.kts
kotlin {
    listOf(
        iosX64(),
        iosArm64(),
        iosSimulatorArm64()
    ).forEach {
        it.binaries.framework {
            baseName = "shared"
            isStatic = true
        }
    }
}
```

### Swift Package Dependencies
```swift
// Package.resolved
dependencies: [
    .package(url: "https://github.com/onevcat/Kingfisher", from: "7.0.0"),
    .package(url: "https://github.com/kishikawakatsumi/KeychainAccess", from: "4.2.0"),
]
```

### Environment Configuration
```swift
// Configuration.swift
enum Environment {
    case development
    case staging
    case production

    var apiBaseUrl: String {
        switch self {
        case .development: return "http://localhost:8081"
        case .staging: return "https://staging-api.reality.example.com"
        case .production: return "https://api.reality.example.com"
        }
    }
}
```

### File List (to create)

**Create:**
- `/mobile-native/iosApp/iosApp.xcodeproj/` - Xcode project
- `/mobile-native/iosApp/iosApp/App/RealityPortalApp.swift` - App entry point
- `/mobile-native/iosApp/iosApp/Core/DI/DependencyContainer.swift` - DI setup
- `/mobile-native/iosApp/iosApp/Core/Configuration.swift` - Environment config
- `/mobile-native/iosApp/iosApp/Resources/Info.plist` - App configuration
- `/mobile-native/iosApp/iosApp/Resources/Assets.xcassets` - App icons

**Modify:**
- `/mobile-native/shared/build.gradle.kts` - Add iOS targets
- `/mobile-native/build.gradle.kts` - Configure iOS build

### KMP Integration Pattern
```swift
// Importing Kotlin module
import shared

// Using Kotlin classes
let apiClient = ApiClient(baseUrl: Environment.current.apiBaseUrl)
let listings = try await apiClient.getListings()
```

### Dependencies
- None (first iOS story)

### References
- [Reference: mobile-native/androidApp/] - Android implementation pattern
- [Package: three.two.bit.ppt.reality]
