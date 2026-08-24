# code-review-mobile-native-kmp-android-session-not-persisted

**Vector:** bug
**Score:** 3
**Source:** Phase 1.5 rotating expert review 2026-08-24 (mobile-native-kmp segment)
**Confidence:** high

## Hypothesis
The Android Reality Portal build scopes `SsoService` to the `MainActivity` instance and holds `sessionToken` in an in-memory field with no persistent store, so any Activity recreation (rotation, dark-mode toggle, locale/font-size change) or cold restart drops the signed-in session to `AuthState.Unauthenticated`. iOS calls `authManager.restoreSession()` at launch and keeps the token in the Keychain; Android has no equivalent path, so the mobile-native product regresses to the signed-out UI mid-session. Adding a KMP `SessionTokenStore` (EncryptedSharedPreferences on Android, reuse `KeychainService` on iOS), hoisting `SsoService` above the Activity, and calling `restoreSession(stored)` once at startup closes the gap.

## Evidence
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/MainActivity.kt:37` — `private val ssoService = SsoService()` scopes the auth session to the Activity instance; every recreation constructs a fresh service.
- `mobile-native/androidApp/src/main/AndroidManifest.xml:25` — the `.MainActivity` declaration has no `android:configChanges`, so the framework recreates the Activity on rotation, dark-mode toggle, and locale change.
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/auth/SsoService.kt:29` — `private val _authState = MutableStateFlow<AuthState>(AuthState.Unauthenticated)` and `private var sessionToken: String? = null` — plain in-memory fields, so a new instance always starts signed out.
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/auth/SsoService.kt:318` — `suspend fun restoreSession(token: String): Boolean` exists but `grep -rn 'restoreSession' mobile-native --include=*.kt` returns only the declaration; no Android caller. iOS calls it: `mobile-native/iosApp/iosApp/App/RealityPortalApp.swift:87`.
- No Android session-persistence primitive is used anywhere in the app: `grep -rn 'SharedPreferences|EncryptedSharedPreferences|DataStore' mobile-native/androidApp/src/main mobile-native/shared/src --include=*.kt` returns 0 hits.

## Files
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/MainActivity.kt`
- `mobile-native/androidApp/src/main/AndroidManifest.xml`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/auth/SsoService.kt`

## Dependencies
<none>

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**

Mode: cloud-ok

The change is a KMP shared-code addition (`expect/actual SessionTokenStore`), an Android-side `RealityPortalApplication` holder swap, and a startup-time `restoreSession` call. The failing-on-main test is a `commonTest` MockEngine case — no real device required. If the implementer wants to hand-verify on hardware they may tick C5 and run local, but the CI-level regression test does not need it.

## Repro steps
1. `./gradlew :androidApp:installDebug && adb shell am start -n three.two.bit.ppt.reality/.MainActivity`
2. Sign in on `LoginScreen` (email/password), or trigger the PM-app SSO deep link `reality://sso?token=<jwt>`. Confirm `AccountScreen` shows the authenticated user.
3. Rotate the device (or toggle system dark mode, or change system font size).
4. **Expected:** the AccountScreen keeps the signed-in user. **Actual:** the app returns to the signed-out UI (Login screen entry point), and each `remember(sessionToken)`-keyed repository in `MainActivity.kt` lines 134–150 (listing, favorites, inquiry, portalListings) rebuilds tokenless — Favorites, Inquiries, MyListings and Analytics all silently revert to their signed-out empty states. Cold-restarting the app has the same effect. Same login on the iOS build survives the equivalent lifecycle events via `KeychainService`.

## Suggested approach
1. Add `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/auth/SessionTokenStore.kt` as an `expect interface` with `suspend fun read(): String?`, `suspend fun write(token: String)`, `suspend fun clear()`.
2. Provide the Android actual as `mobile-native/shared/src/androidMain/…/SessionTokenStore.android.kt` backed by `EncryptedSharedPreferences` (`androidx.security:security-crypto`); provide the iOS actual by delegating to the existing `KeychainService` (`mobile-native/iosApp/iosApp/App/…/KeychainService.swift:` — mirror `AuthManager.swift:282`).
3. Inject the store into `SsoService` (constructor arg), then `write(token)` inside `validateAndLogin` and `loginWithPassword` after a successful `getSession()`, and `clear()` inside `logout()`.
4. Move `SsoService` construction out of `MainActivity.kt:37` into `RealityPortalApplication` (or a `ViewModel` scoped to the process). Pass it into `MainActivity` via the DI seam that already provides `HttpClientProvider`.
5. In the Application `onCreate` (or in a `LaunchedEffect(Unit)` at the composition root of MainActivity), call `runBlocking { store.read()?.let { ssoService.restoreSession(it) } }` once.
6. Add `android:configChanges="orientation|screenSize|screenLayout|uiMode|fontScale|locale"` to the `.MainActivity` entry in `AndroidManifest.xml:25` so the Activity survives orientation / theme / locale changes anyway (belt-and-braces — the token-store fix already covers the semantic bug, but avoiding needless Activity recreation preserves in-flight Compose state).

## Alternatives considered
- **Persist inside `MainActivity` via `SavedInstanceState`** — rejected because `Bundle` state is cleared on cold restart and after low-memory kills, so the "log in survives rotation" case works but the "resume after 20 minutes" case still fails. The token also has no business living in a `Parcelable` for the Compose tree; it belongs at the app boundary.
- **Ship the JWT to `SharedPreferences` (unencrypted)** — rejected because the JWT authorizes API calls against reality-server for the token TTL; leaking it via device backup or another app with `MODE_WORLD_READABLE` semantics would let a co-resident app impersonate the user. `EncryptedSharedPreferences` (or the platform Keystore) is a two-line difference and closes that class.

## Root-cause trace
1. Symptom: after Android device rotation (or cold restart), `AccountScreen` shows the signed-out entry point even though the user completed `LoginScreen`/`validateAndLogin` seconds before.
2. ← Immediate cause at `MainActivity.kt:130` — `val authState by ssoService.authState.collectAsState()` reads the fresh `SsoService`'s default `AuthState.Unauthenticated` because…
3. ← Upstream cause at `MainActivity.kt:37` — `private val ssoService = SsoService()`: the service is scoped to the Activity instance, which Android recreated. `SsoService.kt:29` holds `_authState` and `sessionToken` as plain in-memory fields; the new instance has neither.
4. ← Upstream cause at `MainActivity.kt` and `AndroidManifest.xml:25` — no `SsoService.restoreSession(stored)` is ever called on startup because there is no `stored` (no `SessionTokenStore` in the app), and no `android:configChanges` absorbs the recreate.
5. Origin: this scope choice has been in `MainActivity.kt` since the Android app landed. `SsoService.restoreSession` was added later when iOS needed launch-time restoration (`AuthManager.swift:282` writes to Keychain; `RealityPortalApp.swift:87` calls `restoreSession()`) — Android was never wired to it, so the code path exists in shared code but has no Android caller.

## Test plan
- [ ] `mobile-native/shared/src/commonTest/kotlin/three/two/bit/ppt/reality/auth/SessionPersistenceTest.kt` — new file, following the MockEngine pattern in `mobile-native/shared/src/commonTest/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepositoryTest.kt`: log in against a mock `POST /api/v1/users/login`, assert the token was written to an injected in-memory `SessionTokenStore`, construct a second `SsoService`, call `restoreSession(store.read()!!)` against a mock `GET /api/v1/sso/session`, assert `authState` settles on `AuthState.Authenticated`.
- [ ] Regression: with an empty store, `restoreSession` is not called and `authState` stays `Unauthenticated` (defensive).
- [ ] `cd mobile-native && ./gradlew :shared:allTests` — runs the commonTest suite; must pass. `./gradlew :shared:compileKotlinMetadata` (and, if available in the sandbox, `:androidApp:assembleDebug`) must build after the Android manifest change.

## Out of scope
- Any change to `AuthState`, the reality-server SSO endpoints, or the iOS `AuthManager` flow. iOS already works; the fix is purely Android-side plus one commonMain seam.
- Migrating other in-memory state fields inside `SsoService` (e.g. per-request retry counters). Only session persistence is in scope.
- Fixing the fabricated stats in `AgencyHubScreen` or the CreateListing stub — those are separate open backlog rows and each merit their own PR.

## After-merge
- Move this file to `plans/_archive/code-review-mobile-native-kmp-android-session-not-persisted.md`
- Mark the matching `backlog.json` row as `status: "done"`
