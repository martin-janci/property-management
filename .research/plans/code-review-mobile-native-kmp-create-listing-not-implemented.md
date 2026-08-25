# code-review-mobile-native-kmp-create-listing-not-implemented

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review — mobile-native-kmp dev-review 2026-08-24 (dispatcher tier1d)
**Confidence:** high

## Hypothesis
The Android app's `Screen.CreateListing` navigation node hard-wires its `onSubmit` callback to `Result.failure(NotImplementedError("Wire to listing API"))` (Navigation.kt:490-495). Every tap of the Publish CTA in `CreateListingScreen` immediately returns that error and the screen renders the generic 'network' / 'publish failed' banner. UC-51.4 (Realtor: publish a new listing) is not functionally deliverable in the Android app, yet the entry point is reachable in production from the realtor MyListingsScreen "Create" button — the feature ships broken, not gated. The smallest resolution is to add a `createListing` method to `PortalListingsRepository` that hits reality-server's `POST /api/v1/listings` route and swap the NotImplementedError lambda for a real dispatch; alternatively, hide the CreateListing entry until the wiring exists.

## Evidence
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt:490-495` — `Screen.CreateListing` composable wires `onSubmit = { _ -> Result.failure(NotImplementedError("Wire to listing API")) }`
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/realtor/CreateListingScreen.kt:120-125` — the failure banner path is unconditionally taken because of the NotImplementedError above
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt:484` — MyListingsScreen "Create" button navigates to `Screen.CreateListing.route` with no feature-flag / hidden state
- `grep -rn 'createListing\|CreateListing' mobile-native/shared` — zero hits; `PortalListingsRepository` exposes only `listMyListings` + `getListingAnalytics` + `getPortfolioAnalytics` — there is no POST binding on the shared layer
- Dispatcher tier1d dev-review 2026-08-24 flagged this as high-confidence bug (score_delta=3) — the sole score>=3 open item promotable this run

## Files
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt`
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/realtor/CreateListingScreen.kt`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepository.kt`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [x] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**
Mode: local-only (reason: C5 — mobile-native Android app change; needs a device/emulator to verify the Publish CTA end-to-end against reality-server)

## Repro steps
1. Launch the Android app (debug build) as a signed-in realtor user
2. From the realtor dashboard → MyListingsScreen, tap the "Create" button (routes to `Screen.CreateListing`)
3. Fill the CreateListingScreen form fields with any valid input and tap Publish
4. Expected: listing is created via reality-server and appears in MyListings. Actual: 'network' / 'publish failed' banner shows immediately with no network call in adb logcat, because `Result.failure(NotImplementedError("Wire to listing API"))` short-circuits in Navigation.kt

## Suggested approach
1. Add `suspend fun createListing(input: CreateListingRequest): Result<PortalListing>` to `PortalListingsRepository.kt` in `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepository.kt`, targeting reality-server's `POST /api/v1/listings` route
2. Introduce a `CreateListingRequest` data class in the same shared module mirroring the wire schema; regenerate the generated Ktor client if the repo uses one (check `mobile-native/shared/build.gradle.kts` for `openapi-generator` config)
3. Map the composable's `CreateListingInput` (defined near `CreateListingScreen.kt`) to `CreateListingRequest` at the callsite in `Navigation.kt:490-495`
4. Replace the NotImplementedError lambda with `{ input -> portalRepo.createListing(input.toRequest()) }`; keep the existing `Result<T>` shape so the CreateListingScreen error-banner path stays unchanged
5. In `CreateListingScreen.kt`, on `Result.success` navigate back to MyListings and trigger a `listMyListings()` refresh so the new listing appears immediately
6. Add a smoke unit test around `PortalListingsRepository.createListing` (see Test plan) that mocks the Ktor engine and asserts the request shape + happy-path deserialization
7. If reality-server's POST endpoint is not yet exposed / stable, gate the MyListings "Create" button behind a build config flag until step 1 lands — do NOT ship the reachable-but-broken CTA another release

## Alternatives considered
- **Hide the CreateListing entry from MyListingsScreen for now** — rejected because the shared Kotlin layer already exposes the read side (listMyListings / analytics) and the reality-server POST endpoint exists; hiding the CTA punts a shipped-but-broken UC-51.4 feature without solving it. Only acceptable as a stopgap if step 1 slips a release
- **Wire the callback directly to a raw HttpClient in Navigation.kt** — rejected because it bypasses the `PortalListingsRepository` seam that already owns portal-scoped listing calls, splits Ktor client construction between shared and Android modules, and would make the KMP iOS surface diverge (iOS never sees the shared method)

## Root-cause trace
1. Symptom: signed-in realtor taps Publish in CreateListingScreen → 'publish failed' banner appears immediately with no network activity
2. ← immediate cause at `mobile-native/androidApp/.../ui/realtor/CreateListingScreen.kt:120-125` — the composable renders the failure banner because `onSubmit(input).isFailure` returns true
3. ← upstream cause at `mobile-native/androidApp/.../navigation/Navigation.kt:490-495` — the `onSubmit` lambda hard-returns `Result.failure(NotImplementedError("Wire to listing API"))`, so `isFailure` is always true regardless of user input
4. Origin: the placeholder wiring landed with the CreateListingScreen scaffold (the surrounding composable + form fields + Publish CTA were shipped together with a "wire later" stub in Navigation.kt); no follow-up PR ever replaced the stub against `PortalListingsRepository`, and `grep -rn 'createListing\|CreateListing' mobile-native/shared` confirms zero uses of the intended shared method

## Test plan
- [ ] Unit test: `mobile-native/shared/src/commonTest/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepositoryTest.kt` — new test `createListing_happyPath_deserializesResponse` using a MockEngine that asserts the POST body shape and returns a canned `PortalListing`; a matching `createListing_httpError_returnsFailure` for the 4xx/5xx path
- [ ] Instrumented smoke: launch the debug app on an ADB-connected emulator, sign in as a realtor fixture user, navigate through the CreateListingScreen → Publish flow, assert MyListings shows the new listing after one round-trip. Capture adb logcat to prove a real network call went out
- [ ] Command to run locally: `cd mobile-native && ./gradlew :shared:allTests` for the unit tests; `./gradlew :androidApp:connectedAndroidTest` for the instrumented smoke (needs a device/emulator via C5)

## Out of scope
- iOS SwiftUI CreateListing wiring (the shared `createListing` method arrives with this plan, but the iOS composable / navigation call is a separate follow-up)
- Multi-image upload, address geocoding, or draft persistence on the CreateListing form (the current scaffold only surfaces the fields already rendered; extending them belongs to UC-51.4 iteration, not this fix)
- reality-server API changes to `POST /api/v1/listings` — this plan consumes the existing route as-is; if the route is missing required fields, raise a separate backend plan

## After-merge
- Move this file to `plans/_archive/code-review-mobile-native-kmp-create-listing-not-implemented.md`
- Mark the matching `backlog.json` row as `status: "done"`
