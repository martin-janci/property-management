# Reality Portal - Mobile Native (Kotlin Multiplatform)

Reality Portal mobile app for Android and iOS using Kotlin Multiplatform.

## Requirements

- **JDK 17** (required - AGP 8.x does not support JDK 22+)
- Android SDK with:
  - Platform SDK 34+
  - Build Tools 34.0.0+
- Xcode 15+ (for iOS builds)

## Setup

### Android SDK

Create `local.properties` in this directory with your Android SDK path:

```properties
sdk.dir=/path/to/Android/sdk
```

Common locations:
- macOS: `~/Library/Android/sdk`
- Linux: `~/Android/Sdk`
- Windows: `C:\Users\<user>\AppData\Local\Android\Sdk`

### JDK

Set `JAVA_HOME` to JDK 17 before running Gradle:

```bash
# macOS
export JAVA_HOME=$(/usr/libexec/java_home -v 17)

# Linux
export JAVA_HOME=/usr/lib/jvm/java-17-openjdk

# Windows (PowerShell)
$env:JAVA_HOME = "C:\Program Files\Eclipse Adoptium\jdk-17"
```

## Build

```bash
# Android debug build (all flavors)
./gradlew assembleDebug

# Specific flavor
./gradlew assembleDevelopmentDebug
./gradlew assembleStagingDebug
./gradlew assembleProductionDebug

# Release build (requires signing config)
./gradlew assembleRelease
```

## Project Structure

```
mobile-native/
├── androidApp/          # Android app module (Compose UI)
├── shared/              # Shared Kotlin Multiplatform code
├── iosApp/              # iOS app (SwiftUI)
└── gradle/              # Gradle version catalog
```

## Troubleshooting

### JDK Version Error

If you see an error like `25.0.2` or a cryptic version number:

```
FAILURE: Build failed with an exception.
* What went wrong:
25.0.2
```

This means you're using an unsupported JDK version. Switch to JDK 17:

```bash
export JAVA_HOME=$(/usr/libexec/java_home -v 17)
./gradlew assembleDebug
```

### Android SDK Not Found

Ensure `local.properties` exists with the correct `sdk.dir` path, or set `ANDROID_HOME`:

```bash
export ANDROID_HOME=~/Library/Android/sdk
```
