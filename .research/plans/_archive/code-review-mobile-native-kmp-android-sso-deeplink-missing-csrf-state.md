# code-review-mobile-native-kmp-android-sso-deeplink-missing-csrf-state

**Vector:** security
**Score:** 3
**Source:** Phase 1.5 rotating expert review 2026-07-27 (mobile-native-kmp segment)
**Confidence:** high

## Hypothesis
The Reality Portal Android app accepts an SSO deep-link callback (`reality://sso?token=...`) without verifying a per-flow CSRF `state` nonce, while the iOS app matches state via `AuthManager.consumeSsoState()`. Any external actor (phishing link, notification, another app) can hand the app an attacker-controlled session token, causing the victim to be silently logged into the attacker's account (classic session fixation). Add the missing `state` field to the shared deep-link contract, mint + verify a nonce on Android to match iOS, and reject callbacks whose `state` does not match a pending flow — resolving the platform-parity gap with a small, contained change.

## Evidence
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/MainActivity.kt:88-91` — `handleDeepLink` unconditionally calls `ssoService.validateAndLogin(target.token)` for any `reality://sso?token=...` intent, with no state check.
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/navigation/DeepLinkRouter.kt:22,126` — `DeepLinkTarget.Sso` carries only `token`; there is no `state` field for either the parser or the intent-target to compare against.
- `mobile-native/androidApp/src/main/AndroidManifest.xml:24-41` — the SSO scheme intent-filter is exported (`android:exported="true"`), so any caller can deliver the intent (browser, notification tap, another app on the device).
- `mobile-native/iosApp/iosApp/Core/AuthManager.swift:161-180` + `mobile-native/iosApp/iosApp/App/RealityPortalApp.swift:192` — iOS mints a `state` nonce with `beginSsoFlow()` and calls `consumeSsoState(state)` in the URL handler; Android has no equivalent — this is a platform-parity regression, not a general design gap.

## Files
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/MainActivity.kt`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/navigation/DeepLinkRouter.kt`
- `mobile-native/androidApp/src/main/AndroidManifest.xml`
- `mobile-native/iosApp/iosApp/Core/AuthManager.swift`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [x] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [x] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**
- If C4 or C5 is ticked → `local` (implementer must run on the user's Mac)
- Otherwise → `cloud-ok` (can run as a claude.ai routine via `ppt-bridge` MCP)

Mode: local-only (reason: C5 — Android SSO callback must be exercised on a device/emulator via ADB deep-link injection).

## Repro steps
1. Fresh Android install of the Reality Portal app (`three.two.bit.ppt.reality`), no active session.
2. From ADB: `adb shell am start -a android.intent.action.VIEW -d "reality://sso?token=<any-valid-refresh-token>"` — where the token was minted for account A (attacker's account).
3. Expected (after fix): app rejects the callback, no session is created, and a log/toast records the state mismatch. Actual (today): `MainActivity.handleDeepLink` routes into `ssoService.validateAndLogin(token)`, the app is signed into account A, and any subsequent user action (save listing, send inquiry) is written to A's profile.

## Suggested approach
1. Add a `state: String?` field to `DeepLinkTarget.Sso` in `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/navigation/DeepLinkRouter.kt`; parse it from the incoming URI's `state` query parameter alongside `token`.
2. Introduce a shared `SsoStateStore` (commonMain) that mints + persists a random nonce (e.g. 32-byte URL-safe) when the SSO flow starts, mirroring iOS `AuthManager.beginSsoFlow()` semantics; single-slot storage is sufficient — a new flow overwrites the pending one.
3. Before starting the SSO browser hop on Android (wherever the outbound SSO URL is built), call `SsoStateStore.mint()` and append `&state=<nonce>` to the SSO URL; store the pending nonce.
4. In `MainActivity.handleDeepLink` (`mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/MainActivity.kt:88-91`), branch on `DeepLinkTarget.Sso`: reject (log + `Toast` or silent drop) when `SsoStateStore.consume(target.state)` returns false; only call `ssoService.validateAndLogin(target.token)` on a match.
5. Keep iOS unchanged — its equivalent already exists (`consumeSsoState`) — but move the shared token-verify entry into `SsoService` if it isn't already, so both platforms funnel through the same rejection path.
6. Add an instrumented test (`androidApp/src/androidTest/…/SsoDeepLinkStateTest.kt`) that dispatches an SSO deep-link intent with a mismatched `state` and asserts `ssoService.validateAndLogin` was not called (mock the service) — see *Test plan*.
7. Update the SSO section of `mobile-native/CLAUDE.md` (or the deep-link section of `docs/repo-map.md`, whichever documents the mobile deep-link contract) to state that both platforms require `state`.

## Alternatives considered
- **Refuse all SSO deep-links unless launched from the app's own browser custom-tab session** — rejected because the SSO round-trip legitimately re-enters the app via the OS deep-link handler; there is no in-app trust marker to distinguish the app-initiated launch from an external one without a nonce, so the check reduces to the same state-nonce comparison.
- **Server-side one-shot token binding** (single-use token bound to a device id at issue time) — rejected as a scope-blowout: it requires backend `sso_service` changes and a device-registration layer; the state-nonce fix is a same-file, mobile-only change that achieves parity with iOS today.

## Root-cause trace
1. Symptom: An attacker's `reality://sso?token=<A>` deep-link, tapped by victim, logs the victim into attacker's account A silently.
2. ← Immediate cause at `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/MainActivity.kt:88-91` — `handleDeepLink` calls `ssoService.validateAndLogin(target.token)` with no state check.
3. ← Upstream cause at `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/navigation/DeepLinkRouter.kt:22,126` — `DeepLinkTarget.Sso` never modeled a `state` field, so even a defensive callsite has nothing to compare against.
4. Origin: The mobile-native SSO plumbing landed asymmetrically — iOS `AuthManager.beginSsoFlow`/`consumeSsoState` was added, but the Kotlin side never got a shared `SsoStateStore` or the state field in the shared deep-link model. (Trace the commit history of `DeepLinkRouter.kt` and `AuthManager.swift` to identify the exact PR the asymmetry landed in.)

## Test plan
- [ ] Android instrumented test `mobile-native/androidApp/src/androidTest/java/three/two/bit/ppt/reality/SsoDeepLinkStateTest.kt` — dispatch an `Intent(ACTION_VIEW, Uri.parse("reality://sso?token=X&state=WRONG"))` after mint(); assert `SsoService.validateAndLogin` was NOT called and the session remains anonymous. This test fails on `dev`.
- [ ] Unit test `mobile-native/shared/src/commonTest/kotlin/three/two/bit/ppt/reality/navigation/DeepLinkRouterTest.kt` — asserts `parse("reality://sso?token=X&state=Y")` returns `DeepLinkTarget.Sso(token=X, state=Y)`; and that a missing state parses to `state=null`. Fails on `dev` (field doesn't exist yet).
- [ ] Manual smoke: adb-inject a matching-state deep-link → login succeeds; adb-inject a mismatched-state deep-link → login rejected. Command: `adb shell am start -a android.intent.action.VIEW -d "reality://sso?token=<T>&state=<S>"`.
- [ ] Run: `cd mobile-native && ./gradlew :shared:allTests` and `./gradlew :androidApp:connectedDebugAndroidTest` — both must be green.

## Out of scope
- Server-side one-shot token binding (see *Alternatives*).
- Any change to iOS `AuthManager.beginSsoFlow` / `consumeSsoState` — the fix is achieving Android parity with the existing iOS implementation, not redesigning it.
- Widening the state-nonce to cover other deep-link targets (`reality://listing/...`, `reality://saved/...`) — those don't hand out a session token and are out of scope for this plan.

## After-merge
- Move this file to `plans/_archive/code-review-mobile-native-kmp-android-sso-deeplink-missing-csrf-state.md`
- Mark the matching `backlog.json` row (`code-review-mobile-native-kmp-android-sso-deeplink-missing-csrf-state`) as `status: "done"`
