# code-review-mobile-native-kmp-create-listing-not-wired

**Vector:** bug
**Score:** 7
**Source:** rotating-expert-review (dispatcher Tier-1d 2026-09-03 mobile-native-kmp create-listing stub)
**Confidence:** high

## Hypothesis
The Android Realtor "Create listing" screen (`CreateListingScreen`) is a complete UI that submits into a stubbed `onSubmit = { _ -> Result.failure(NotImplementedError("Wire to listing API")) }` in `Navigation.kt`. As a result UC-51.4 (Realtor publishes a new listing) is unusable on the shipped app — every submit surfaces the generic "publish failed" banner and the entered form data is dropped. The KMP shared repository `PortalListingsRepository` has no `createListing` method; grepping `shared/` for `createListing|POST.*listings|listings.*POST` returns zero hits. The fix is small and mechanical: add `PortalListingsRepository.createListing(input): Result<Unit>` that POSTs to the reality-server portal listings create endpoint, then swap the Navigation stub for a real repository call.

## Evidence
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt:493` — production `composable(Screen.CreateListing.route)` wires `onSubmit = { _ -> Result.failure(NotImplementedError("Wire to listing API")) }`; reachable from `MyListingsScreen` via `onCreateClick = { navController.navigate(Screen.CreateListing.route) }` (~L487) and `AgencyHubScreen` (~L415).
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/realtor/CreateListingScreen.kt:40-45` — screen declares `onSubmit: suspend (CreateListingInput) -> Result<Unit>` with full validation/state/submit UI; docstring L28-36 explicitly names UC-51.4.
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/realtor/CreateListingScreen.kt:122-125` — error branch renders `it.isNetworkError() ? networkErrorMsg : publishFailedMsg`, so the user always sees "publish failed".
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepository.kt:22-158` — repository exposes only `listMyListings`, `getListingAnalytics`, `getPortfolioAnalytics`; no `createListing` method exists in `shared/`.
- Backend counterpart to confirm the target endpoint: `backend/servers/reality-server/src/routes/portal_listings.rs` (create route + payload contract).

## Files
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt`
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/realtor/CreateListingScreen.kt`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepository.kt`
- `backend/servers/reality-server/src/routes/portal_listings.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [x] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception

Mode: local-only (reason: C5 — needs an Android emulator/device to smoke-test the wired create-listing flow end-to-end)

## Repro steps
1. Build the Android Realtor app on `main`, sign in as a realtor, navigate MyListings → tap "Create listing" (FAB).
2. Fill the form with a valid title/price/transactionType/currency and tap Publish.
3. Expected: 200 from the reality-server portal listings create endpoint; the new listing appears in MyListings after refresh.
4. Actual on `main`: publish button spins briefly, then the "publish failed" banner appears; nothing is sent to the backend (verify with `adb logcat`/Charles proxy — no `/api/v1/portal/listings` POST is observed).

## Suggested approach
1. In `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepository.kt`, add `suspend fun createListing(input: CreateListingInput): Result<Unit>` that POSTs to the reality-server portal listings create endpoint (mirror the existing `listMyListings` HttpClient use and error-mapping).
2. Confirm the payload contract against `backend/servers/reality-server/src/routes/portal_listings.rs` (field names, transaction type enum, currency, price shape); either reuse an existing `CreateListingRequest` DTO or introduce one under `shared/` next to `PortalListingsResponse`.
3. Wire the DI/factory that produces `PortalListingsRepository` so a live instance is available where `Navigation.kt` currently instantiates the stubbed lambda (mirror how the other Navigation lambdas hand off to shared repositories).
4. Replace the stub at `Navigation.kt:493` with `onSubmit = { input -> repository.createListing(input) }` (async-safely, matching how the composable receives a `suspend` lambda).
5. Ensure `CreateListingScreen` still surfaces `isNetworkError` vs generic failure via the existing `Throwable.isNetworkError()` helper; do not change UI copy.
6. Add a shared/unit test asserting `createListing` POSTs the expected JSON body against a mocked HttpClient, and an Android UI test (or instrumented test) asserting a successful create navigates back and shows the new item in `MyListings`.
7. Update `docs/screens/reality/*` if a screen-map entry references CreateListing status (build/api).

## Alternatives considered
- **Feature-flag hide the "Create listing" affordance until the API lands** — rejected because the shipped UI is already reachable from two entry points (MyListings and AgencyHubScreen) and users are already tapping a dead button; a stub-hidden button is worse UX than shipping the wired flow.
- **Wire directly from `Navigation.kt` to Ktor without going through `PortalListingsRepository`** — rejected because it duplicates the auth/base-url/error mapping the repository already centralises, and future analytics (`getListingAnalytics`) already lives in that repository; the create method belongs there for consistency.

## Root-cause trace
1. Symptom: submitting the Android CreateListing form always fails ("publish failed" banner); no HTTP POST to `/api/v1/portal/listings` is observed.
2. ← `CreateListingScreen` propagates the `Result.failure(NotImplementedError)` produced by the `onSubmit` lambda into its error branch.
3. ← `Navigation.kt:493` wires that lambda literally — the stub was never replaced when the screen shipped.
4. ← `PortalListingsRepository` has no `createListing` method to call, so wiring the lambda requires adding one first (a two-file change, not a one-liner).
5. Origin: the initial `CreateListingScreen` scaffold PR — implementation stopped at the UI + navigation and the shared repository work was never done.

## Test plan
- [ ] Shared/unit test in `mobile-native/shared/src/commonTest/kotlin/.../realtor/PortalListingsRepositoryTest.kt`: mock HttpClient, call `createListing(sampleInput)`, assert POST verb, endpoint path, request body JSON, and success/error mapping. Test must fail on `main` (method doesn't exist).
- [ ] Android instrumented test in `mobile-native/androidApp/src/androidTest/.../CreateListingScreenTest.kt` (or nearest existing suite): compose the screen with a repository stub that returns `Result.success(Unit)`; fill fields; assert `MyListings` receives the new item.
- [ ] Run: `./gradlew :shared:allTests` and `./gradlew :androidApp:testDebugUnitTest` from `mobile-native/`. When a device/emulator is attached, `./gradlew :androidApp:connectedDebugAndroidTest`.

## Out of scope
- Editing an existing listing (that's a separate screen).
- Photo upload flow (assume the create endpoint accepts a text-only payload; photo upload is a separate ticket).
- iOS `CreateListingScreen` — this ticket is Android-only until iOS parity is scheduled.

## After-merge
- Move this file to `plans/_archive/code-review-mobile-native-kmp-create-listing-not-wired.md`
- Mark the matching `backlog.json` row as `status: "done"`
