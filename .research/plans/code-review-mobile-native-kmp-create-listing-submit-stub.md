# code-review-mobile-native-kmp-create-listing-submit-stub

**Vector:** bug
**Score:** 8
**Source:** tier1d-dev-review segment=mobile-native-kmp (2026-08-02, 08-03, 08-08, 08-09)
**Confidence:** high

## Hypothesis
The Realtor **Create Listing** screen is fully reachable in mobile-native (Android/iOS) production navigation, but every submit returns a hardcoded `Result.failure(NotImplementedError("Wire to listing API"))`. Users fill in the form, tap submit, and always see an error. The `PortalListingsRepository` layer has no `create` method — it only exposes `listMyListings` / `getListingAnalytics` / `getPortfolioAnalytics`. Fix: add a `createListing(payload)` repository call against the reality-server API, wire it through the composable `onSubmit`, and remove the stub.

## Evidence
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt:493` — `CreateListingScreen(onSubmit = { _ -> Result.failure(NotImplementedError("Wire to listing API")) })`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepository.kt` — exposes only read methods (`listMyListings`, `getListingAnalytics`, `getPortfolioAnalytics`); no `create`/`submit`.
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/realtor/CreateListingScreen.kt` — the composable that receives the stub `onSubmit` lambda.
- Grep `fun create` / `fun submit` under `mobile-native/shared/src/commonMain` returns nothing — no other repository provides the seam either.
- reality-server exposes the create endpoint (see `backend/servers/reality-server/src/routes/realtor/*` listings routes) — this is the API to consume.

## Files
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepository.kt`
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/realtor/CreateListingScreen.kt`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [x] C5 — ADB device (mobile-touching) · **local-only**
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Mode: local-only** (reason: C5 — mobile-native Android/iOS app requires a device/emulator for verification of the submit flow)

Mode: local-only

## Repro steps
1. Open the Reality Portal Android app (mobile-native/androidApp) as a Realtor user.
2. Navigate MyListings → Create Listing → fill required fields → tap Submit.
3. **Expected:** listing is created and appears in "My Listings".
   **Actual:** an error toast fires immediately; nothing is persisted; server logs show no POST reached reality-server.

## Suggested approach
1. In `PortalListingsRepository.kt`, add `suspend fun createListing(payload: CreateListingPayload): Result<Listing>` that calls the generated `openapi` client's Realtor listings POST endpoint. Use the same error-mapping helpers as `listMyListings`.
2. Introduce a `CreateListingPayload` data class (or reuse the OpenAPI-generated request DTO) with the fields the form collects — title, price, area, rooms, description, media refs, etc.
3. In the ViewModel behind `CreateListingScreen`, expose a `submit()` suspend that calls the repository and returns the `Result<Listing>` to the composable's `onSubmit`.
4. In `Navigation.kt:493`, replace the stub lambda with a call to that ViewModel's `submit()` and route success to the newly-created listing's detail page (matching the flow declared in `docs/screens/reality/*.md` if a screen doc exists — otherwise emit a `screen-map-drift` follow-up).
5. Handle common failure modes explicitly in the composable — validation error (map field errors from the server), auth/session-expired (bubble to login), network failure (retry banner).
6. Add tests: (a) commonTest unit for `PortalListingsRepository.createListing` mapping success/failure; (b) androidTest instrumented for the submit happy path.

## Alternatives considered
- **Ship the form as read-only until the API is wired** — rejected because the button and form are already in production nav; the user's path is broken *now*, not blocked behind a feature flag.
- **Add only the repository seam and leave the composable stub** — rejected because the visible bug (button that fails) is what users hit; a seam without wiring doesn't restore any functionality.

## Root-cause trace
1. Symptom: Tap "Submit" on Create Listing → error toast; nothing created.
2. ← `CreateListingScreen(onSubmit = { _ -> Result.failure(NotImplementedError(...)) })` at `Navigation.kt:493` — the lambda is a placeholder.
3. ← `PortalListingsRepository` has no `createListing` method — the underlying data-layer seam was never built.
4. Origin: The Create Listing screen was scaffolded and merged into the Realtor flow before the repository/API integration; the placeholder lambda was left as a "wire later" TODO that never got wired.

## Test plan
- [ ] `mobile-native/shared/src/commonTest/kotlin/.../PortalListingsRepositoryTest.kt` — new `createListing_success_returnsListing` and `createListing_validationError_mapsFieldErrors`.
- [ ] Android instrumented test that mounts `CreateListingScreen`, fills fields, calls submit, and asserts navigation to detail.
- [ ] Regression: navigating back after submit does not re-submit (single-fire).
- [ ] Command: `cd mobile-native && ./gradlew :shared:allTests :androidApp:testDebugUnitTest`

## Out of scope
- iOS SwiftUI equivalent — file a follow-up if the KMP repo seam is not enough to cover the iOS composable.
- Redesign of the Create Listing form fields (validation UX, image picker) — this plan only closes the submit gap.
- Server-side changes — the reality-server endpoint already exists.

## After-merge
- Move this file to `plans/_archive/code-review-mobile-native-kmp-create-listing-submit-stub.md`
- Mark the matching `backlog.json` row as `status: "done"`
