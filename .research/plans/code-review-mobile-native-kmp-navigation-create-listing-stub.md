# code-review-mobile-native-kmp-navigation-create-listing-stub

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review (segment `mobile-native-kmp`, 2026-07-16 dispatcher tier1d)
**Confidence:** high

## Hypothesis
The Reality Portal Android app ships a fully built realtor `CreateListingScreen`, but its navigation-level `onSubmit` is hardwired to `Result.failure(NotImplementedError("Wire to listing API"))`, and no `createListing` method exists on `ListingRepository` / `ApiClient`. So every real user submission from `MyListings → Create → publish` folds the not-implemented failure into the generic "publish failed" banner, silently blocking realtors from creating listings on Android. The smallest resolving change is to add a `createListing(input): Result<Listing>` method to the shared repository and API client, then wire the navigation callback to it.

## Evidence
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt:476` — `onSubmit = { _ -> Result.failure(NotImplementedError("Wire to listing API")) }` (confirmed on `dev`).
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/realtor/CreateListingScreen.kt:120-126` — `result.fold(..., onFailure = { generalError = if (it.isNetworkError()) networkErrorMsg else publishFailedMsg })` — every failure surfaces as the generic banner, so the `NotImplementedError` is indistinguishable from a real network / server failure.
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/listing/ListingRepository.kt` — six public network methods (`searchListings`, `getFeaturedListings`, `getRecentListings`, `getSearchSuggestions`, `getListingsNearby`, `getListingDetail`), no `createListing`. `grep -rn "createListing" mobile-native/shared/src/` returns zero hits — the method is absent, not just untested.
- `.research/signals/2026-07-16-mobile-native-kmp-tier1d.json` — the tier1d review that surfaced this finding at `score_delta=3`, `confidence=high`, `candidate_vector=bug`.
- Backend API surface: `create_listing`-shaped endpoint on `reality-server` — the KMP `ListingRepository` already talks to portal endpoints (`getFeaturedListings`, `getListingsNearby`), so wiring `POST` alongside the existing GET calls is a linear extension of the same client.

## Files
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt:476`
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/realtor/CreateListingScreen.kt:120`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/listing/ListingRepository.kt`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug vector — confirm the API contract and error paths before wiring)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [x] C5 — ADB device (Android app change — the fix has to be exercised on a device / emulator so the submit path proves it lands a listing end-to-end)
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: local-only (reason: C5 — Android app change needs an ADB device to exercise `MyListings → Create → publish` on a real / emulated device)

## Repro steps
1. Build the Android app: `cd mobile-native && ./gradlew :androidApp:assembleDebug`.
2. Install on an emulator or physical device; log in as a realtor role.
3. Navigate `Bottom nav → MyListings → Create` (`Screen.CreateListing`).
4. Fill the form with valid title / description / city / price / currency / transactionType and tap `Publish`.
5. Expected: the listing is created and the app pops back to `MyListings` (`onCreated()`), and the new listing appears in the realtor's own list.
6. Actual: the "publish failed" banner appears; the listing is not created; no HTTP `POST` was issued (verified via Ktor logging plugin or the `mockEngine` in tests) — the failure originated inside the `NotImplementedError` at `Navigation.kt:476`, never reaching the network.

## Suggested approach
1. Add the API model + client method on `shared/`. In `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/listing/`, introduce `CreateListingRequest` (mirror `CreateListingInput` fields) and add `suspend fun createListing(request: CreateListingRequest): Listing` to `ApiClient` — `POST /api/v1/portal/listings` (verify the exact reality-server endpoint against `docs/api/typespec/` or `backend/servers/reality-server/src/routes/portal_listings.rs`). Serialize with the existing kotlinx-serialization models; keep the same JSON shape the backend already expects.
2. Add `suspend fun createListing(input: CreateListingInput): Result<Listing>` to `ListingRepository`, mapping non-2xx to `ListingException` and network errors to `Result.failure(...)` using the same pattern as `getListingsNearby`. Follow the `Result<T>` convention already used elsewhere in the repo.
3. Inject `ListingRepository` at the navigation composition root (`MainActivity.kt` already provides `portalListingsRepository` — extend the same DI-lite pattern). Update the `composable(Screen.CreateListing.route)` block in `Navigation.kt:473-479` to call `listingRepository.createListing(input)` from `onSubmit` instead of returning the hardcoded `NotImplementedError`.
4. On the iOS side, mirror the wiring: `iosApp/iosApp/Features/Realtor/CreateListingView.swift` (or the equivalent) must call the same shared repository method through the `DependencyContainer` so both platforms exercise the same code path — the `NotImplementedError` is Android-only today; do a parity check for iOS so we don't ship one platform ahead of the other.
5. Add `spotlessApply` before committing to keep ktfmt clean; the `spotlessKotlinCheck` gate skips build-android/build-ios when style fails and would hide the actual test signal (this is the exact class of miss called out in PR #2379's ktfmt fix commit).
6. Manually walk the flow on an ADB-connected device end-to-end (form → submit → success banner → `MyListings` shows the new row), and confirm a 4xx server response surfaces as the specific message from `ListingException`, not the generic `publishFailedMsg` fallback.

## Alternatives considered
- **Hide the `Create` entry point until the API is wired** — rejected because the screen and validation are already fully built and this leaves realtors with no way to publish from mobile. The fix is a linear wiring change, not a design gap.
- **Wire `onSubmit` directly to a raw Ktor call inside `Navigation.kt`** — rejected because it bypasses the repository / testable abstraction the rest of the shared module uses; every other network call goes through `ListingRepository` so its `MockEngine` tests can pin request bodies. Duplicating the client at the nav layer would be a regression from the KMP conventions PR #2371 just established for `PortalListingsRepository`.

## Root-cause trace
1. Symptom: `MyListings → Create → Publish` on Android surfaces the generic "publish failed" banner regardless of form input.
2. ← `CreateListingScreen.kt:122-125` folds the `Result.failure` into `generalError = publishFailedMsg`, masking the actual failure kind.
3. ← `Navigation.kt:476` returns `Result.failure(NotImplementedError("Wire to listing API"))` unconditionally — the failure never came from a network call.
4. ← `ListingRepository.kt` — no `createListing` method exists, so there is nothing for the navigation callback to bind to; the placeholder was intended as a temporary stub.
5. Origin: the KMP-native realtor Create flow (PR family that introduced `CreateListingScreen` + `MyListings`, mirrored by PR #2371 for the LIST + analytics path) landed the UI ahead of the shared repository / API-client work; the stub was never replaced. The lack of a running test that submits the form is what let it ship — see the tester test-plan below.

## Test plan
- [ ] `mobile-native/shared/src/commonTest/kotlin/three/two/bit/ppt/reality/listing/ListingRepositoryCreateTest.kt` — new `MockEngine`-based test asserting `createListing(input)` issues a `POST` to the expected path with the expected JSON body, returns `Result.success(listing)` on 201, and maps `422`/`5xx` to `Result.failure(ListingException(...))`. Mirrors `PortalListingsRepositoryTest` from PR #2371.
- [ ] `mobile-native/androidApp/src/androidTest/.../CreateListingScreenInstrumentedTest.kt` (or equivalent Compose UI test) — render the screen with a fake `onSubmit` returning `Result.success(...)` and `Result.failure(ListingException.Validation(...))`; assert the "publish failed" banner is shown only for the failure case and that `onCreated()` fires exactly once on success.
- [ ] Exact commands: `cd mobile-native && ./gradlew :shared:allTests` (unit); `cd mobile-native && ./gradlew :androidApp:connectedDebugAndroidTest` (needs ADB — Mode: local-only). Run `./gradlew spotlessCheck` before push (ktfmt gate).

## Out of scope
- iOS-specific UI polish beyond wiring the same `createListing` call (a full iOS parity pass on the realtor Create screen belongs to the follow-up iOS feature epic — this plan only needs the shared client to be reachable from iOS via `DependencyContainer`).
- Backend endpoint changes — the `create_listing` reality-server endpoint is out of scope unless verification reveals a request/response shape gap; if so, split into a follow-up backend plan rather than expanding this PR.
- Analytics / event tracking on submit — separate concern from the wiring bug.

## After-merge
- Move this file to `plans/_archive/code-review-mobile-native-kmp-navigation-create-listing-stub.md`
- Mark the matching `backlog.json` row (`code-review-mobile-native-kmp-navigation-create-listing-stub`) as `status: "done"` with an evidence line `"resolved: PR #<N> merged YYYY-MM-DD — <title>"`.
