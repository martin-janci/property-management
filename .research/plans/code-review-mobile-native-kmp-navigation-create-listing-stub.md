# code-review-mobile-native-kmp-navigation-create-listing-stub

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review (tier1d 2026-07-18)
**Confidence:** high

## Hypothesis
The Android nav host wires `CreateListingScreen.onSubmit` to a permanent `Result.failure(NotImplementedError("Wire to listing API"))`, and no `createListing()` method exists in `ListingRepository` or `ApiClient`. The full realtor create-listing form folds every submit into its "publish failed" error banner, so the feature is silently broken in production. The fix is small: add `createListing()` to the shared repository + API client, then wire the nav callback to it.

## Evidence
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt:476` — `onSubmit = { Result.failure(NotImplementedError("Wire to listing API")) }` hard-wired inside the `CreateListingScreen(` invocation.
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/realtor/CreateListingScreen.kt:120` — the form folds the failure into a "publish failed" banner via the shared error surface.
- `shared/` grep: only `createInquiry`, `createSavedSearch`, `createMobileToken` exist under the API-client surface — no `createListing`.

## Files
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt`
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/realtor/CreateListingScreen.kt`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [x] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: local-only (reason: C5 — Android APK must run on a device / emulator to prove the submit reaches reality-server)

## Repro steps
1. Build and install the Reality Portal Android app (`./gradlew :androidApp:installDebug`) against a reality-server instance.
2. Log in as a realtor.
3. Navigate MyListings → Create.
4. Fill the required fields and tap **Publish**.
5. Expected: the listing is created server-side and the app returns to MyListings with the new row visible. Actual: red "publish failed" banner; no request reaches reality-server; no listing is created.

## Suggested approach
1. Under `mobile-native/shared/`, add a `createListing(request: CreateListingRequest)` method to the API client (openapi-generated stubs already model `POST /realtor/listings` — regenerate with `./gradlew :shared:openApiGenerate` if a fresh spec is available; otherwise author the KMP Ktor call by hand mirroring `createInquiry`).
2. Add `createListing(...)` to `ListingRepository` in the shared module — same shape as `createInquiry`: `suspend fun createListing(req: CreateListingRequest): Result<Listing>` that funnels through `SafeCall`.
3. In `Navigation.kt:474`, replace the `onSubmit = { Result.failure(NotImplementedError(...)) }` lambda with `onSubmit = { req -> listingRepository.createListing(req) }`. Inject `ListingRepository` at the nav host the same way other realtor screens do.
4. Update the Compose form's success path to navigate back to MyListings with a snackbar or refresh trigger (the failure path already exists at `CreateListingScreen.kt:120`).
5. Add a unit test on the repository method (mocked Ktor engine returns 201) and a Compose UI test that a successful submit navigates away from the create screen.
6. Run `./gradlew :androidApp:testDebug :shared:allTests` before opening the PR.

## Alternatives considered
- **Hide the Create button** — rejected because the redesign specifically added realtor create-listing to the roadmap; the deliverable is the flow, not its absence.
- **Wire `onSubmit` to a REST call inline in `Navigation.kt`** — rejected because it bypasses the repository layer and duplicates auth/token handling every future call site would need.

## Root-cause trace
1. Symptom: realtor create-listing submit never reaches reality-server; UI shows "publish failed".
2. ← `Navigation.kt:476` returns `Result.failure(NotImplementedError(...))` unconditionally — never calls a repository.
3. ← `shared/` has no `createListing` on `ListingRepository` / `ApiClient` — the placeholder was left because the API method wasn't ported yet.
4. Origin: the realtor create-listing screen was added as UI-only (design-first landing) with the intent to wire the API later; no follow-up ticket closed the loop.

## Test plan
- [ ] `mobile-native/shared/src/commonTest/kotlin/.../ListingRepositoryTest.kt` — mocked Ktor engine: `createListing()` returns `Result.success(listing)` on a 201 payload; returns `Result.failure` with typed error on a 4xx.
- [ ] `mobile-native/androidApp/src/androidTest/.../CreateListingScreenTest.kt` — Compose test: fill fields, tap Publish, assert nav host advances (`Screen.MyListings`) instead of surfacing the error banner.
- [ ] Local command: `cd mobile-native && ./gradlew :shared:allTests :androidApp:testDebug`.

## Out of scope
- iOS parity for create-listing — the SwiftUI screen may have the same gap; scope this plan to Android and file iOS as a follow-up finding.
- Redesign of the create-listing form itself — this plan wires the existing form to the API only.

## After-merge
- Move this file to `plans/_archive/code-review-mobile-native-kmp-navigation-create-listing-stub.md`
- Mark backlog row `code-review-mobile-native-kmp-navigation-create-listing-stub` as `status: "done"`
