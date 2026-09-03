# code-review-mobile-native-kmp-create-listing-not-wired

**Vector:** bug
**Score:** 7
**Source:** rotating-expert-review (dispatcher Tier-1d 2026-08-30 / 2026-08-31 / 2026-09-03 mobile-native-kmp); folded by routine Phase 1.5 2026-09-03
**Confidence:** high

## Hypothesis
The KMP realtor `CreateListingScreen` composable is wired in `Navigation.kt:493` with `onSubmit = { _ -> Result.failure(NotImplementedError("Wire to listing API")) }`. From the shipped Android app the "Create listing" flow is reachable via `AgencyHubScreen:415` and `MyListingsScreen:484`; users fill the form and receive `realtor_create_validation_publish_failed`, and the entered listing is never persisted. Backend endpoint `POST /api/v1/my/listings` already exists (`backend/servers/reality-server/src/routes/portal_listings.rs:207 create_listing`, registered at `main.rs:205`); only the KMP client side is missing. The fix is: add `createListing(input): Result<PortalListing>` to `PortalListingsRepository` (POST /api/v1/my/listings via the shared Ktor `HttpClient`) and pass it as the composable's `onSubmit`.

## Evidence
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt:493` — production `composable(Screen.CreateListing.route)` uses the stub `onSubmit`.
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/realtor/CreateListingScreen.kt` — the composable already computes a validated `CreateListingInput` and calls `onSubmit(input)` on the "Publish" tap; nothing else in the screen needs changing.
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepository.kt` — has `listMyListings()` (GET /api/v1/my/listings) but no `createListing`; the wrapping shape (`Result<T>` return + shared `HttpClient` install) is already in place.
- `backend/servers/reality-server/src/routes/portal_listings.rs:24` (route table) + `:207 create_listing` — POST endpoint exists, JWT-guarded, returns the created listing.
- Rotating expert reviews on 2026-08-30, 2026-08-31, and 2026-09-03 (mobile-native-kmp segment) all independently flagged the stub as HIGH — backlog row scored 4 → 7 over three folds.

## Files
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt`
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/realtor/CreateListingScreen.kt`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepository.kt`
- `backend/servers/reality-server/src/routes/portal_listings.rs`

## Dependencies
- pm-devops-unblock-mobile-native-cloud-builds

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [x] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**
- C5 ticked → `local`

Mode: local-only (reason: C5 — mobile-native/KMP build path; cloud runner AGP egress is gated by open infra issue #2652)

## Repro steps
1. Build and install the current `mobile-native/androidApp` on an emulator: `cd mobile-native && ./gradlew :androidApp:installDebug` (with a running emulator + `adb`).
2. Sign in as a realtor account (SSO through Reality Portal), navigate: Agency Hub → "My Listings" → "Create listing" (or the equivalent hub tile).
3. Fill the create-listing form with any valid inputs (title, price, area, description, category) and tap "Publish".
4. Expected: the screen navigates back and the new listing appears in `POST /api/v1/my/listings`'s response and in `GET /api/v1/my/listings` on next refresh.
5. Actual: the screen shows the toast/alert `realtor_create_validation_publish_failed`; server logs show no `POST /api/v1/my/listings` request; on `GET /api/v1/my/listings` the listing is absent.

## Suggested approach
1. In `PortalListingsRepository.kt` add `suspend fun createListing(input: CreateListingInput): Result<PortalListing>` that POSTs to `/api/v1/my/listings` via the shared `HttpClient`, deserializes the response into the existing `PortalListing` DTO, and follows the same `runCatching { … }` + non-swallowing `CancellationException` re-throw pattern used elsewhere in the file (or the pattern we settle on in `code-review-mobile-native-kmp-cancellation-swallowed`, whichever lands first).
2. Confirm the `CreateListingInput` DTO's JSON shape matches the backend `create_listing` handler's expected body (`backend/servers/reality-server/src/routes/portal_listings.rs:207` — inspect the `axum::extract::Json<...>` type + `sqlx::query!` inserts). If field names differ, add `@SerialName` on the KMP model, don't rename the backend contract.
3. In `Navigation.kt:493` replace the stub `onSubmit` with a lambda that captures a `PortalListingsRepository` instance (already resolved elsewhere in the composable graph — mirror what `MyListingsScreen` does) and calls `repo.createListing(input)`, mapping the returned `Result<PortalListing>` to `Result<Unit>` for the composable signature (drop the payload, or thread it through if the caller needs the created id for navigation).
4. Optional but recommended: on success, invalidate/refresh the "my listings" cache so `MyListingsScreen` shows the new row immediately (call sites already exist for the list flow).
5. Add IG3 KMP unit test (`mobile-native/shared/src/commonTest/kotlin/.../realtor/PortalListingsRepositoryTest.kt`) using the existing `MockEngine` pattern (see `LayoutRepositoryTest.kt` for the shape): POST /api/v1/my/listings returns a `PortalListing` JSON body → `createListing(input)` returns `Result.success(listing)`. Assert the request URL, method (POST), and body shape match.
6. Add a second test for the failure path: MockEngine returns HTTP 500 → `createListing(input)` returns `Result.failure` with the mapped domain error (whatever the repository's error-mapping helper produces elsewhere).
7. Verify no other composable expects the old `NotImplementedError` behavior (grep `NotImplementedError` under `mobile-native/`); update dead-stub tests if any.

## Alternatives considered
- **Wire the composable directly to `HttpClient` inside `CreateListingScreen`** — rejected because it bypasses the repository layer that every other CRUD flow in this app uses; makes the failure path untestable and violates the shared-module ownership boundary (compose/androidApp shouldn't talk HTTP directly).
- **Add a new `RealtorListingRepository` distinct from `PortalListingsRepository`** — rejected because `PortalListingsRepository` already owns `GET /api/v1/my/listings` and the create endpoint is under the same `/api/v1/my/listings` collection; splitting responsibilities across two repositories for the same resource would fragment cache invalidation and duplicate the shared `HttpClient` wiring.

## Root-cause trace
1. Symptom: user-visible `realtor_create_validation_publish_failed` toast; no HTTP request emitted from the app; created listing never persists.
2. ← `CreateListingScreen.kt` `onSubmit(input)` is invoked (composable behaves correctly) but returns `Result.failure(NotImplementedError(…))`.
3. ← `Navigation.kt:493` passes the stub `onSubmit = { _ -> Result.failure(NotImplementedError("Wire to listing API")) }` — the wiring gap is here.
4. ← `PortalListingsRepository.kt` never grew a `createListing` method; the stub was inserted as a "TODO to be wired when repo method exists" placeholder that shipped.
5. Origin: `CreateListingScreen` route was first introduced in the KMP realtor rewrite (git log the `Navigation.kt` composable block for the introducing commit) — the composable was landed as UI-only ahead of the API wiring, and the wiring commit never followed.

## Test plan
- [ ] `mobile-native/shared/src/commonTest/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepositoryTest.kt` — new `createListing_postsToMyListingsAndReturnsListing()` test using `MockEngine`; asserts POST + URL + body shape and success mapping.
- [ ] Same test file — new `createListing_returnsFailureOn500()` for the error path.
- [ ] Regression: verify no other route or composable regressed by running `./gradlew :shared:allTests :androidApp:testDebugUnitTest` locally.
- [ ] Exact commands to run locally (Mac, ADB present):
  - `cd mobile-native && ./gradlew :shared:allTests`
  - `cd mobile-native && ./gradlew :androidApp:testDebugUnitTest`
  - Manual smoke: `./gradlew :androidApp:installDebug` then follow *Repro steps* above; confirm the listing appears.

## Out of scope
- Any iOS-only changes (the same wiring lands via KMP shared code; iOS gets the fix for free once the shared repo method exists).
- Cancellation-exception swallowing across other KMP repositories (tracked separately as `code-review-mobile-native-kmp-cancellation-swallowed`; if that plan lands first, this plan inherits its pattern rather than duplicating the fix).
- Backend contract changes to `POST /api/v1/my/listings` — the endpoint is already implemented and JWT-guarded; this plan is a KMP client wiring only.
- Refactoring `PortalListingsRepository` to a different DI shape.

## After-merge
- Move this file to `plans/_archive/code-review-mobile-native-kmp-create-listing-not-wired.md`
- Mark the matching `backlog.json` row (`id: code-review-mobile-native-kmp-create-listing-not-wired`) as `status: "done"`
