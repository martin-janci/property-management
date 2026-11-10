# code-review-mobile-native-kmp-create-listing-not-wired

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review 2026-08-30 (dispatcher Tier-1d mobile-native-kmp create-listing nav wiring)
**Confidence:** high

## Hypothesis
The KMP realtor "Create Listing" flow is a UI dead-end: Navigation.kt:493 wires `CreateListingScreen`'s `onSubmit` to `Result.failure(NotImplementedError("Wire to listing API"))`, so tapping Submit throws and discards all form data. The backend endpoint (`POST /api/v1/my/listings` at `backend/servers/reality-server/src/routes/portal_listings.rs:207` `create_listing`) is already implemented and validated. The fix is: add `PortalListingsRepository.createListing(...)` calling the endpoint, then replace the `NotImplementedError` lambda in `Navigation.kt:493` with a call into the repository.

## Evidence
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt:493` — `onSubmit = { _ -> Result.failure(NotImplementedError("Wire to listing API")) }`
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/realtor/CreateListingScreen.kt:41` — `onSubmit: suspend (CreateListingInput) -> Result<Unit>` — the Composable's contract already expects a working suspend function.
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepository.kt` — no `createListing`; only list / get / patch coverage.
- `backend/servers/reality-server/src/routes/portal_listings.rs:207` — `pub async fn create_listing` handler already wired at `POST /api/v1/my/listings`.

## Files
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt:493`
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/realtor/CreateListingScreen.kt`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepository.kt`
- `backend/servers/reality-server/src/routes/portal_listings.rs`

## Required capabilities
- [x] C1 — Systematic debugging (bug vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [x] C5 — ADB device (KMP Android instrumented smoke)
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception

Mode: local-only (reason: C5 — mobile-native-kmp requires AGP + Android device; cloud runner is egress-blocked on `dl.google.com` per issue #2652 / #2951).

## Repro steps
1. Launch the KMP Android app; sign in as a realtor account with a portal owner id.
2. Navigate: bottom nav → "My listings" → tap "Create listing".
3. Fill the create-listing form (title, description, price, city, postal_code, street) and tap Submit.
4. Expected: 201 from `POST /api/v1/my/listings`, screen pops back, new listing appears in the list.
5. Actual: `Result.failure(NotImplementedError("Wire to listing API"))` surfaces; form data is discarded; nothing hits the backend.

## Suggested approach
1. In `PortalListingsRepository.kt`, add a `suspend fun createListing(input: CreateListingInput): Result<PortalListing>` that POSTs to `/api/v1/my/listings` via the shared Ktor `HttpClient`. Model `CreateListingInput` as a `@Serializable` common-main data class mirroring the reality-server `CreatePortalListingRequest` shape (title, description, listing_type, listing_price, city, postal_code, street, plus the current optional set).
2. On 201, deserialize the response body into the existing `PortalListing` model. On non-2xx, map to `Result.failure(HttpException(status, body))` so the UI can show the backend's validation message (mirrors the pattern used by `updateListing`).
3. In `Navigation.kt:493`, replace the `NotImplementedError` lambda with `onSubmit = { input -> portalListingsRepository.createListing(input).map { navController.popBackStack(); Unit } }`. Inject `portalListingsRepository` the same way `Navigation.kt` already resolves other shared repositories (Koin / manual construction — mirror the ListingRepository wiring above the composable).
4. Keep `CreateListingScreen.kt:41`'s `onSubmit: suspend (CreateListingInput) -> Result<Unit>` signature unchanged so no call sites shift.
5. Add `shared/src/commonTest/kotlin/.../PortalListingsRepositoryTest.kt` — `createListing_returns201Body_and_deserializes()` using `MockEngine` to return a canonical 201 body; asserts the returned `PortalListing` matches the fixture.
6. Add `PortalListingsRepositoryTest.createListing_maps422_to_failure()` — asserts a 422 body is surfaced as `Result.failure` carrying the backend body.

## Alternatives considered
- **Wire the ViewModel to Ktor directly, skipping the repository** — rejected because it fragments network access away from the existing repository pattern used for list/get/patch; makes injection and MockEngine tests inconsistent with the rest of the shared module.
- **Ship a client-side placeholder that no-ops with a toast** — rejected because the backend endpoint is live and validated; a placeholder would leave the realtor create-listing feature broken while pretending success.

## Root-cause trace
1. Symptom: tapping Submit on `CreateListingScreen` throws `NotImplementedError("Wire to listing API")`.
2. ← `Navigation.kt:493` binds `onSubmit` to a `Result.failure(NotImplementedError(...))` lambda.
3. ← `PortalListingsRepository.kt` never grew a `createListing` method to point at.
4. Origin: `CreateListingScreen` shipped as a scaffold (screen implemented, wiring deferred) — the `NotImplementedError` placeholder was left in place across nav updates.

## Test plan
- [ ] `PortalListingsRepositoryTest.createListing_returns201Body_and_deserializes` (shared/commonTest)
- [ ] `PortalListingsRepositoryTest.createListing_maps422_to_failure` (shared/commonTest)
- [ ] Regression scenario: on a local Android device, filling and submitting the form creates the listing (backend log confirms 201).
- [ ] Command: `./gradlew :shared:allTests` (fast); optionally `./gradlew :androidApp:connectedDebugAndroidTest` for the smoke.

## Out of scope
- Editing / deleting portal listings from mobile.
- iOS `CreateListingView` parity — shared repository lands here; iOS screen wiring is a follow-up.
- Photo upload for the created listing (separate endpoint, separate flow).

## After-merge
- Move this file to `plans/_archive/code-review-mobile-native-kmp-create-listing-not-wired.md`.
- Set backlog row `code-review-mobile-native-kmp-create-listing-not-wired` to `status: "done"`.
