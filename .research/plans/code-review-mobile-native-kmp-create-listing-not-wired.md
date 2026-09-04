# code-review-mobile-native-kmp-create-listing-not-wired

**Vector:** bug
**Score:** 4
**Source:** rotating-expert-review 2026-08-30 + dispatcher Tier-1d mobile-native-kmp 2026-08-31
**Confidence:** high

## Hypothesis
The KMP realtor "Create listing" flow is wired to a stub: `Navigation.kt:493` passes `onSubmit = { _ -> Result.failure(NotImplementedError("Wire to listing API")) }` into `CreateListingScreen`, so every submission surfaces `realtor_create_validation_publish_failed` and the entered listing is silently discarded. The backend endpoint (`POST /api/v1/my/listings` at `backend/servers/reality-server/src/routes/portal_listings.rs:24` → `create_listing` at `:207`) is already live and registered in `main.rs:205`. The KMP shared repository `PortalListingsRepository` already talks to the sibling `GET /api/v1/my/listings` (`listMyListings`) but has no create method. The smallest change is KMP-only: add `createListing(input): Result<Unit>` to the repository (POST to the existing route) and pass it as the composable's `onSubmit`.

## Evidence
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt:493` — `onSubmit = { _ -> Result.failure(NotImplementedError("Wire to listing API")) }` reachable from AgencyHubScreen (`:415`) and MyListingsScreen (`:484`).
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/realtor/CreateListingScreen.kt` — the composable's failure branch surfaces the `realtor_create_validation_publish_failed` string; users see the error and the form data is dropped on the floor.
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepository.kt` — only `listMyListings()` today; no create method exists.
- `backend/servers/reality-server/src/routes/portal_listings.rs:24` (route) + `:207` (`create_listing` handler); registered in `backend/servers/reality-server/src/main.rs:205`. Endpoint is live and independently tested backend-side.
- Rotating expert review (dispatcher Tier-1d mobile-native-kmp 2026-08-31) re-confirmed the stub, upgrading confidence to `high` and score to 4.

## Files
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepository.kt`
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt`
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/realtor/CreateListingScreen.kt`
- `backend/servers/reality-server/src/routes/portal_listings.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [x] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

Mode: local-only (reason: C5 — KMP Android/iOS wiring change needs on-device build; issue #2652 tracks the cloud-runner AGP egress gap and was closed 2026-09-04 but the routine still schedules KMP work to a local implementer for verification)

## Repro steps
1. Build the KMP Android app on a device / emulator (`cd mobile-native && ./gradlew :androidApp:installDebug`); launch as a realtor user in the reality portal.
2. Navigate: AgencyHubScreen → MyListingsScreen → "Create listing" (or the direct route wired from `Navigation.kt:493`).
3. Fill in a valid listing form and tap Submit.
4. Expected: listing is POSTed and appears in `GET /api/v1/my/listings`.
5. Actual: the composable's error branch fires with `realtor_create_validation_publish_failed`; nothing is sent (verifiable by observing zero traffic on the network inspector or checking backend logs — no `POST /api/v1/my/listings` line).

## Suggested approach
1. In `PortalListingsRepository.kt`, add a `suspend fun createListing(input: CreateListingInput): Result<Unit>` that issues `httpClient.post("$baseUrl/api/v1/my/listings") { setBody(input) }` and returns `Result.success(Unit)` on `2xx`, `Result.failure(...)` on non-2xx (mirror the existing `listMyListings()` error handling). Wire the DTO to the existing `create_listing` request shape from `portal_listings.rs:207` — add a matching `@Serializable data class CreateListingInput(...)` in `mobile-native/shared/.../realtor/PortalListingsModels.kt` (or the existing models file) with only the fields the backend requires.
2. In `Navigation.kt:493`, replace the stub with `onSubmit = { input -> viewModelScope-launched call to portalListingsRepository.createListing(input) }` — inject the repository via the same DI/pattern the sibling `MyListingsScreen` uses (`:484`) to obtain `listMyListings`.
3. In `CreateListingScreen.kt`, keep the current success/failure UX (success returns the user to MyListingsScreen and triggers a refresh; failure raises the existing i18n key). Do NOT change the composable's public signature beyond swapping `Result<Unit>` semantics if needed.
4. Add IG3 test coverage in `mobile-native/shared/src/commonTest/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepositoryTest.kt` — a Ktor `MockEngine` case that verifies (a) `createListing` issues `POST /api/v1/my/listings` with the serialized body, and (b) a `201 Created` returns `Result.success(Unit)`, and (c) a `422` returns `Result.failure(...)`.
5. Do NOT touch the backend endpoint or its response shape — the contract is already correct and used by other clients.

## Alternatives considered
- **Move CreateListingScreen off KMP shared and implement in each platform's native code** — rejected because the shared repository is exactly the KMP boundary this codebase uses everywhere else (see `InquiryRepository`, `listMyListings`); duplicating the HTTP call per platform breaks the pattern and doubles the test surface.
- **Add the create method behind a feature flag and keep the stub as fallback** — rejected because there is no rollback story that leaves users with a broken submit button; the flag would just delay real usage without buying any safety.

## Root-cause trace
1. Symptom: realtors tap Submit on the KMP Create Listing form, see `realtor_create_validation_publish_failed`, and no listing is created.
2. ← Composable's `onSubmit` returns `Result.failure(NotImplementedError)` at `Navigation.kt:493`.
3. ← `PortalListingsRepository` has no `createListing` method (repository was authored with only the read path).
4. ← The screen was scaffolded ahead of the repository call and the TODO was never closed; no CI test would catch this because there is no unit/e2e test that exercises the submit path end-to-end on KMP.
5. Origin: the Create Listing feature landed in the KMP navigation graph as a UI-only slice; the backend `POST /api/v1/my/listings` handler shipped separately and no follow-up wired them together.

## Test plan
- [ ] `mobile-native/shared/src/commonTest/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepositoryTest.kt` — new unit test with Ktor `MockEngine`: `createListing(sampleInput)` posts to `/api/v1/my/listings`, body matches serialized DTO, `201` → `Result.success`, `422` → `Result.failure`.
- [ ] Manual regression (local, ADB): create a listing from AgencyHubScreen → MyListingsScreen; observe the new row appears in the list on refresh and in the backend DB / `GET /api/v1/my/listings`.
- [ ] Run: `cd mobile-native && ./gradlew :shared:allTests` (and `:androidApp:testDebugUnitTest` if there are Android-side stubs to update).

## Out of scope
- No backend endpoint or response-shape changes — `create_listing` in `portal_listings.rs:207` is authoritative.
- No iOS-specific work beyond whatever falls out of the shared repository (the iOS composable equivalent is already wired to the shared boundary in the same pattern).
- No dispatcher / cloud-runner AGP work — tracked separately in the (now-closed) issue #2652.

## After-merge
- Move this file to `plans/_archive/code-review-mobile-native-kmp-create-listing-not-wired.md`
- Mark the matching `backlog.json` row as `status: "done"`
