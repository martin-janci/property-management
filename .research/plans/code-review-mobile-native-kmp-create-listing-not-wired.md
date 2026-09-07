# code-review-mobile-native-kmp-create-listing-not-wired

**Vector:** bug
**Score:** 4
**Source:** rotating-expert-review 2026-08-30 (dispatcher Tier-1d mobile-native-kmp); reconfirmed 2026-08-31
**Confidence:** high

## Hypothesis
The Android realtor `CreateListingScreen` composable is wired in `Navigation.kt` with `onSubmit = { _ -> Result.failure(NotImplementedError("Wire to listing API")) }`. Any realtor pushing "Publish" from AgencyHub or MyListings sees the localized `realtor_create_validation_publish_failed` toast and their entered listing is silently discarded. The backend endpoint (`POST /api/v1/my/listings`) already exists in `portal_listings.rs` and `PortalListingsRepository` already talks to that route for reads; the fix is a KMP-only wiring change — add a `createListing()` method to the repository and hand it to the composable.

## Evidence
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt:493` — `onSubmit = { _ -> Result.failure(NotImplementedError("Wire to listing API")) }` reached from `AgencyHubScreen` and `MyListingsScreen` entries at lines 415 and 484.
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/realtor/CreateListingScreen.kt:41` — composable's `onSubmit: suspend (CreateListingInput) -> Result<Unit>` contract is exercised on Publish (line 109).
- `backend/servers/reality-server/src/routes/portal_listings.rs:24` route + `:207 create_listing` handler — `POST /api/v1/my/listings` already accepts new listings and is registered in `reality-server` main routing.
- Dispatcher Tier-1d review 2026-08-31 (`mobile-native-kmp`) reconfirmed the stub is still live on `origin/dev`; score bumped 2→4.

## Files
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt:491`
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/realtor/CreateListingScreen.kt:41`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepository.kt`
- `backend/servers/reality-server/src/routes/portal_listings.rs`

## Dependencies
- gh-issue-2652

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [x] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**
Mode: local-only (reason: C5 — KMP androidApp build + verification needs ADB device; cloud runner egress blocks AGP per issue #2652)

## Repro steps
1. Sign in on Android as a realtor role and open the AgencyHub tab.
2. Tap "Create listing", fill the form, tap Publish.
3. Expected: request lands at `POST /api/v1/my/listings`, listing appears in MyListings on refresh.
4. Actual: snackbar shows the localized `realtor_create_validation_publish_failed` string; no HTTP request is sent; MyListings unchanged.

## Suggested approach
1. Add `suspend fun createListing(input: CreateListingInput): Result<Unit>` to `PortalListingsRepository` that POSTs to `/api/v1/my/listings` and maps HTTP errors into a `Result.failure` with a domain error type (mirroring `listMyListings` in the same file).
2. In `Navigation.kt:491–493`, replace the `NotImplementedError` closure with an invocation that delegates to the injected `PortalListingsRepository.createListing(...)` and threads the `Result<Unit>` through the composable.
3. Make sure the composable's success path in `CreateListingScreen.kt:95–120` triggers navigation back to MyListings and invalidates the listings query so the new row shows without a manual refresh.
4. Add a KMP unit test in `mobile-native/shared/src/commonTest/.../realtor/PortalListingsRepositoryTest.kt` that stubs the Ktor engine, asserts `POST /api/v1/my/listings` is issued with the mapped body, and returns `Result.success(Unit)` on 201.
5. Add an androidApp UI test (or a small Compose test) that exercises the Publish button and asserts the injected repository is called.

## Alternatives considered
- **Ship a client-only queue that persists the payload for later replay** — rejected because it hides the failure from the realtor and there's no consumer of the queue; the backend already accepts the write, so there's nothing to defer.
- **Route CreateListing through the shared KMP `ListingsRepository` (readonly-side today)** — rejected because that repository is scoped to consumer-side reads (`GET` only) and mixing writer concerns would break its `commonMain` abstraction; `PortalListingsRepository` is the realtor-facing seam.

## Root-cause trace
1. Symptom: realtor pushes Publish on `CreateListingScreen`, sees `realtor_create_validation_publish_failed` toast, listing never appears.
2. ← `CreateListingScreen.kt:109` invokes `onSubmit(input)` and treats a `Result.failure(NotImplementedError)` as a validation failure surfaced to the user.
3. ← `Navigation.kt:491–493` provides `onSubmit = { _ -> Result.failure(NotImplementedError("Wire to listing API")) }` instead of calling the repository.
4. Origin: composable landed as a UI-only scaffold before `PortalListingsRepository` grew a write API; the stub was left in place as a placeholder and never revisited.

## Test plan
- [ ] `PortalListingsRepositoryTest.createListing_postsToMyListingsEndpoint` — new KMP unit test using a mocked Ktor engine
- [ ] Compose UI test on `CreateListingScreen` asserting `onSubmit` invocation calls the repository (mocked) and navigates back on success
- [ ] Command to run locally: `cd mobile-native && ./gradlew :shared:allTests :androidApp:testDebugUnitTest`

## Out of scope
- iOS SwiftUI equivalent of CreateListingScreen (separate wiring in `iosApp/`).
- Adding new fields to the create payload — this plan only wires the existing `CreateListingInput` to the existing backend contract.
- Any changes to `reality-server` `create_listing` handler behavior or its DB path.

## After-merge
- Move this file to `plans/_archive/code-review-mobile-native-kmp-create-listing-not-wired.md`
- Mark the matching `backlog.json` row as `status: "done"`
