---
name: ppt-mobile-native
description: Kotlin Multiplatform app under mobile-native/ — Gradle workflow, modules, ADB requirement.
when_to_use: The plan touches mobile-native/ — Kotlin shared module, Android app, iOS app, or any mobile-specific flow.
mode: local
capabilities: [C5, C6]
tags: [mobile]
---

# PPT Mobile Native

Kotlin Multiplatform project at `mobile-native/`. Gradle build, shared
Kotlin module, Android app (`androidApp/`), iOS shell (`iosApp/`). UI uses
Compose Multiplatform.

## When to invoke

Plan vector area is `mobile`, or *Suggested approach* references
`mobile-native/**`. Note: `frontend/apps/mobile` is a *separate*
React-Native/Expo app — see [`ppt-nuxt-frontend`](../ppt-nuxt-frontend/SKILL.md)
for that one.

## What it gives you

- Module layout
- `./gradlew` quick reference
- ADB requirement (C5, **local-only** — no cloud bridge equivalent in v1)

## Inputs

- A target module (`shared` or `androidApp`)
- An emulator or USB-connected device for UI flows

## Module layout

```
mobile-native/
├── shared/         # KMP shared module — business logic, networking
├── androidApp/     # Android entry point
├── iosApp/         # iOS entry point (Xcode-driven)
├── build.gradle.kts
├── settings.gradle.kts
└── gradle.properties
```

## Steps

1. **Build:**
   ```bash
   cd mobile-native && ./gradlew build                   # full
   cd mobile-native && ./gradlew :shared:build           # shared only
   cd mobile-native && ./gradlew :androidApp:assembleDebug
   ```
2. **Test:**
   ```bash
   cd mobile-native && ./gradlew test                              # all
   cd mobile-native && ./gradlew :shared:test                      # one module
   cd mobile-native && ./gradlew test --tests "*<filter>*"
   ```
3. **Install on a connected device/emulator:**
   ```bash
   cd mobile-native && ./gradlew :androidApp:installDebug
   ```
4. **UI flow verification (C5).** Use the `adb-app-control` skill —
   screenshot, dump UI hierarchy, tap/swipe/type. Requires `adb devices`
   to show at least one `device` (not `unauthorized` / `offline`).
5. **iOS** — open `mobile-native/iosApp` in Xcode. Gradle drives the shared
   framework; the iOS shell is built natively. Skip the iOS arm unless the
   plan explicitly requires it.

## Deterministic verification

```bash
# 1. gradle wrapper present + executable
test -x mobile-native/gradlew && echo OK
# expected: OK

# 2. JDK available
java -version 2>&1 | head -1
# expected: a JDK version line (Gradle needs 17+)

# 3. (only when plan needs UI) ADB present
command -v adb >/dev/null && echo OK
# expected: OK if you have Android Studio / platform-tools installed; otherwise plan
# requiring C5 cannot proceed on this host.

# 4. (only when plan needs UI) at least one device
adb devices 2>/dev/null | awk 'NR>1 && $2=="device"' | wc -l
# expected: >= 1 for any C5 step; 0 is fine for build/test-only changes
```

## Smoke check (single command)

```bash
cd mobile-native && ./gradlew help -q
```

> **Expected failure modes on this Mac that are normal:**
>
> - If JDK 17+ isn't installed, `gradlew help` fails. Install Temurin 17 via
>   Homebrew (`brew install --cask temurin@17`) or rely on the Android
>   Studio JDK.
> - `adb devices` returns no `device` lines if no emulator is running.
>   That's fine unless the plan needs C5.
>
> The verification harness counts a smoke fail as fail — that's by design.
> If the host can't build mobile-native, the implementer can't ship
> mobile-native plans from that host.

## After-task verification

```bash
cd mobile-native && ./gradlew check test
```

## Cross-references

- [`.research/implementer-prompt.md`](../../implementer-prompt.md) C5 — ADB
- `adb-app-control` skill (in `~/.claude/skills/`) — actual ADB driver
- [`ppt-nuxt-frontend`](../ppt-nuxt-frontend/SKILL.md) — separate RN app
