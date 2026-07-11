# security-android-sso-missing-csrf-state

**Vector:** security
**Score:** 3
**Source:** Phase 1.5 code-review 2026-07-11 (mobile-native-kmp)
**Confidence:** high

## Hypothesis
The Android app's deep-link handler in `MainActivity` accepts `reality://sso?token=…` and calls `ssoService.validateAndLogin(target.token)` with no CSRF `state` nonce check. iOS enforces one via `AuthManager.beginSsoFlow`/`consumeSsoState` (state issued at flow start, verified on return). The shared `DeepLinkTarget.Sso(token)` doesn't carry a `state` field at all, so Android is structurally unable to verify it. Any process on the device that opens `reality://sso?token=<attacker-token>` (a malicious app, a webpage via intent, a QR code, a chat link) can silently sign the victim into the attacker's account — the classic session-fixation / forced-login. The smallest fix threads `state` through the shared model, has Android verify it against a store issued at flow-start (mirroring iOS's `consumeSsoState`), and rejects mismatches.

## Evidence
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/MainActivity.kt:90-91` — `is DeepLinkTarget.Sso -> lifecycleScope.launch { ssoService.validateAndLogin(target.token) }` (no state check).
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/navigation/DeepLinkRouter.kt:22` — `data class Sso(val token: String)` (no `state` field).
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/navigation/DeepLinkRouter.kt:126` — parser reads only `token`, discards `state=` if present.
- `mobile-native/iosApp/iosApp/Core/AuthManager.swift` — `beginSsoFlow` issues a state nonce; `consumeSsoState` verifies it on return (established pattern to mirror).

## Files
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/MainActivity.kt`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/navigation/DeepLinkRouter.kt`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/auth/SsoService.kt`
- `mobile-native/iosApp/iosApp/Core/AuthManager.swift`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (security bug — trace both platforms' flow)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [x] C5 — ADB device (mobile-touching plan — verify Android behavior on-device)
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: local-only (reason: C5 — Android SSO deep-link flow must be exercised on a physical device or emulator to prove state-mismatch rejection)

## Repro steps
1. Build and install the Reality Portal Android app on a device/emulator.
2. Ensure the user has never begun an SSO flow this session (no `state` pending).
3. From another process (adb shell), fire the deep link: `adb shell am start -a android.intent.action.VIEW -d "reality://sso?token=ATTACKER_TOKEN"`.
4. Expected (after fix): the app rejects the callback (log + toast: unrecognized SSO state) and does NOT invoke `validateAndLogin`.
5. Actual (today, main): `MainActivity.handleDeepLink` reaches the `Sso` branch and calls `ssoService.validateAndLogin("ATTACKER_TOKEN")` unconditionally.

## Suggested approach
1. Extend the shared model: `data class Sso(val token: String, val state: String? = null)` in `DeepLinkRouter.kt:22`, and update `parse(...)` (`DeepLinkRouter.kt:126`) to also read the `state` query param: `queryParam(queryPart, "token")?.let { tok -> DeepLinkTarget.Sso(tok, queryParam(queryPart, "state")) }`.
2. Port the `beginSsoFlow` / `consumeSsoState` pattern into shared `SsoService` (or a new common `SsoStateStore`) so Android and iOS share the state issue+consume logic. Persist the pending `state` — on Android via `EncryptedSharedPreferences` or `androidx.datastore`, on iOS keep the existing Keychain-backed store.
3. In `MainActivity.handleDeepLink` (`MainActivity.kt:88-96`), before calling `validateAndLogin`, call `ssoService.consumeSsoState(target.state)`; if it returns false (missing/mismatch/expired), abort with a user-visible message and do NOT touch `validateAndLogin`.
4. Update iOS `DeepLinkHandler` (if it currently constructs `Sso(token)` without state) to populate the new `state` field so both platforms flow the same value.
5. Add a shared unit test in `DeepLinkRouter` for `reality://sso?token=t&state=s` → `Sso("t", "s")` and for `reality://sso?token=t` → `Sso("t", null)`.
6. Add an Android instrumentation / JVM unit test that simulates a `reality://sso?token=X&state=Y` intent under three conditions: (a) no pending state → reject, (b) pending state != Y → reject, (c) pending state == Y → accept.
7. Grep `mobile-native/` for other `validateAndLogin` callers; verify only the deep-link path exists (or gate all of them the same way).

## Alternatives considered
- **Backend-only state enforcement** — rejected because Android happily hands `token` to `validateAndLogin` before the backend even sees the request; a stolen bearer can be validated by a legitimate backend, so the state check has to fire before the token leaves the device.
- **Detect + warn without blocking (log-only)** — rejected because a session-fixation forced-login is silent by design — the user has no signal to act on a log line; the app must refuse to complete the callback.

## Root-cause trace
1. Symptom: An external process fires `reality://sso?token=…` at the Android app → the victim is signed in as the attacker's account with no user confirmation.
2. ← Immediate cause at `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/MainActivity.kt:90-91` — `Sso` branch calls `validateAndLogin(target.token)` with no state check, unlike iOS.
3. ← Upstream cause at `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/navigation/DeepLinkRouter.kt:22` — shared `Sso` model has no `state` field; parser at line 126 discards `state=` query param — Android is structurally unable to verify state even if it wanted to.
4. Origin: Epic 122 (Push Notification Deep Links) + Epic 10A-SSO landed the `Sso` deep-link target on Android without threading state through the shared model. iOS added state enforcement in its own path (`AuthManager.beginSsoFlow`/`consumeSsoState`) that Android cannot use.

## Test plan
- [ ] `mobile-native/shared/src/commonTest/.../DeepLinkRouterTest.kt` — parser assertions for `Sso(token, state)` presence/absence.
- [ ] `mobile-native/androidApp/src/test/.../MainActivityDeepLinkTest.kt` (or instrumentation equivalent) — three cases: no pending state, mismatched state, matched state; assert `validateAndLogin` is invoked only in the matched case.
- [ ] `./gradlew :shared:allTests :androidApp:testDebugUnitTest` locally to confirm all three tests fail on `main` and pass after the fix.

## Out of scope
- Reworking iOS-side SSO — the existing `AuthManager` state contract is the reference; do not refactor it.
- Adding token persistence on Android (`code-review-mobile-native-kmp-android-no-session-persistence` — separate backlog row for that).
- Backend-side state store — the state issue/verify pair is deliberately client-owned in the current design; do not migrate.

## After-merge
- Move this file to `plans/_archive/security-android-sso-missing-csrf-state.md`
- Mark the matching `backlog.json` row as `status: "done"`
