# Reality Portal iOS Tests

Epic 80 - Story 80.6: Mobile Unit Tests (iOS)
Epic 82 - Story 82.5: Inquiries and Account (KeychainService, PushNotificationManager)

## Overview

This directory contains XCTest unit tests for the Reality Portal iOS application.

## Test Structure

```
iosAppTests/
├── README.md                    # This file
├── Info.plist                   # Test target configuration
└── RealityPortalTests.swift     # All test classes
```

## Test Classes

| Class | Purpose | Epic |
|-------|---------|------|
| `ConfigurationTests` | `Configuration` singleton, `Environment` enum, `keychainService` binding | 82.1 |
| `RouteTests` | `Route` enum equality / hashability (NavigationStack dedup) | 82.2 |
| `DeepLinkTests` | URL scheme parsing sanity checks | 82.2 |
| `DeepLinkHandlerTests` | Full `DeepLinkHandler` parse-table coverage (SSO, listing, search, favorites, inquiries, account, compare) | 82.2 |
| `NavigationStateRestorationServiceTests` | Encode/decode round-trips, save/restore, auth-gating, double-restore cycle regression | 82.2 |
| `AuthenticationTests` | `AuthManager` initial state | 82.3 |
| `InquiryStatusTests` | `InquiryStatus` display names (locale-agnostic), badge colours | 82.5 |
| `InquiryPreviewTests` | `InquiryPreview` model, `formattedDate`, sample data | 82.5 |
| `PushNotificationManagerTests` | Default preferences, UserDefaults persistence, device-token Keychain round-trip, hex conversion | 82.5 |
| `KeychainServiceTests` | Save/load/delete/overwrite/contains/deleteAll against an isolated Keychain service | 82.5 |
| `PerformanceTests` | `Configuration.apiBaseUrl` hot-path baseline | — |

## Running Tests

**From Xcode (macOS required):**

```
Cmd + U   # Run all tests
```

or

```bash
xcodebuild test \
  -project mobile-native/iosApp/iosApp.xcodeproj \
  -scheme iosApp \
  -destination 'platform=iOS Simulator,name=iPhone 15 Pro,OS=17.5'
```

**Note:** `KeychainServiceTests` and `PushNotificationManagerTests` interact with the
iOS Keychain API via `Security.framework`. They are isolated using a unique service
name per test run (UUID suffix) and clean up after themselves in `tearDown`.
They must run on a simulator or device — not on macOS (which has a different Keychain
API surface).

## Key Conventions

- Each test class uses isolated state: `UserDefaults(suiteName:)` for navigation tests;
  unique Keychain service names for security tests.
- `tearDown` clears all state so tests are order-independent.
- `InquiryStatusTests` uses locale-agnostic assertions (`!isEmpty`, `count == Set.count`)
  so tests pass on non-English CI runners.
- `DEBUG`-only code (sample data) is covered with `#if DEBUG` guards — those tests
  will pass in Release builds with no assertions (Swift `#if` omits the block entirely).

## Services Covered

### KeychainService (`Core/Services/KeychainService.swift`)

Secure token storage backed by `Security.framework`. Stores:

| Key (constant) | Value |
|----------------|-------|
| `KeychainService.Keys.accessToken` | JWT access token |
| `KeychainService.Keys.refreshToken` | Refresh token |
| `KeychainService.Keys.pushDeviceToken` | APNs device token (hex string) |

Access class: `kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly` — tokens survive
restarts but are NOT migrated to a new device via iCloud Backup.

### PushNotificationManager (`Core/Services/PushNotificationManager.swift`)

Manages APNs registration and per-category notification preferences.

- Device token stored in `KeychainService` (not `UserDefaults`).
- Preference booleans stored in `UserDefaults` (not secret, need no Keychain protection).
- `@Observable` — SwiftUI views observe `isAuthorized`, `isRequestingPermission`,
  `deviceToken`, `preferences` without any explicit `objectWillChange` calls.
- Created once in `RealityPortalApp` and injected via `.environment(pushNotificationManager)`.
