# code-review-mobile-native-kmp-create-listing-not-wired

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review 2026-08-30 + 2026-08-31 (dispatcher Tier-1d mobile-native-kmp)
**Confidence:** high

## Hypothesis
KMP realtor `CreateListingScreen` is wired into Navigation with an `onSubmit` lambda that always returns `Result.failure(NotImplementedError("Wire to listing API"))`. Every realtor who fills the form and taps *Publish* sees the generic `realtor_create_validation_publish_failed` error and their listing is silently discarded. `PortalListingsRepository` already talks to the same reality-server portal API (`listMyListings` → `GET /api/v1/my/listings`), and the backend `POST /api/v1/my/listings` endpoint already exists and is registered — so the fix is KMP-only: add `createListing()` to the shared repository and pass a real lambda from `Navigation.kt`.

## Evidence
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt:493` — `onSubmit = { _ -> Result.failure(NotImplementedError("Wire to listing API")) }`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepository.kt:22` — repository class already wires `HttpClient` + auth; `listMyListings` at :40 uses `GET /api/v1/my/listings`; there is NO `createListing` method
- `backend/servers/reality-server/src/routes/portal_listings.rs:207` — `pub async fn create_listing(...)` handler with `POST /api/v1/my/listings` route already implemented, registered in `main.rs:205`
- Dispatcher Tier-1d 2026-08-31 mobile-native-kmp review re-confirmed the stub on the current head — score raised 2 → 4 → 3 after 2026-09-17 decay pass

## Files
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepository.kt`
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt`
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/realtor/CreateListingScreen.kt`
- `backend/servers/reality-server/src/routes/portal_listings.rs`

## Dependencies
<none>

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [x] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**
Mode: local-only (reason: C5 — mobile-native KMP touches Android composable navigation; ADB device needed to smoke-test the flow)

## Repro steps
1. Launch the KMP Android app as an authenticated realtor (`three.two.bit.ppt.reality` debug build).
2. From `MyListingsScreen`, tap the FAB / *Create listing* affordance so `Screen.CreateListing` route loads.
3. Fill the form (title, price, address — enough to satisfy client-side validation) and tap *Publish*.
4. Expected: server returns `201 Created`, the listing appears in `MyListings` after `popBackStack`.
5. Actual: toast shows `realtor_create_validation_publish_failed`, nothing is posted (`POST /api/v1/my/listings` never fires — verify with a proxy or backend log).

## Suggested approach
1. In `PortalListingsRepository.kt`, add `suspend fun createListing(input: CreatePortalListingRequest): Result<PortalListingResponse>` mirroring the existing `listMyListings` shape (auth header, base URL, error mapping via existing helpers).
2. Add the shared DTO types (`CreatePortalListingRequest`, response) under `mobile-native/shared/src/commonMain/kotlin/.../realtor/` if not already present — prefer regenerating from the OpenAPI spec via the KMP generator to keep field names + enums in sync with `backend/servers/reality-server/src/routes/portal_listings.rs`.
3. In `Navigation.kt:490-495`, obtain the repository instance the same way `MyListingsScreen` does (Koin/dependency lookup already in scope) and replace the `NotImplementedError` lambda with one that calls `repo.createListing(...)` and folds the result into `Result<Unit>` — surface HTTP errors through the existing `Result.failure` path so `CreateListingScreen` keeps rendering `realtor_create_validation_publish_failed` for 4xx / 5xx.
4. Confirm `CreateListingScreen.kt` calls `onCreated` after a successful `onSubmit`; today that path is unreachable, so verify it actually triggers `popBackStack(Screen.MyListings.route, false)`.
5. Add a KMP unit test (see *Test plan*) that fails on the current stub and passes after the wiring.
6. Smoke via ADB: install debug APK, log in as a realtor, create a listing, confirm it appears in *My Listings* and the reality-server logs show `POST /api/v1/my/listings 201`.
7. Update `docs/screens/` if a screen doc for CreateListing exists (spot-check `docs/screens/` — if absent, do not create one in this plan).

## Alternatives considered
- **Move the wiring into `CreateListingScreen` itself (inject the repository into the composable)** — rejected because it breaks the current pattern where Navigation owns dependency lookup and screens stay stateless w.r.t. data access; would require refactoring the whole realtor navigation module.
- **Stub `createListing` as a no-op success so users see "success" without a real POST** — rejected because it hides the failure behind fake UX and would corrupt analytics; the whole point is that the listing must actually be created server-side.

## Root-cause trace
1. Symptom: realtor taps *Publish*, form shows `realtor_create_validation_publish_failed`, nothing appears in *My Listings*.
2. ← `CreateListingScreen.kt` calls its `onSubmit` prop → the prop always returns `Result.failure(NotImplementedError(...))`.
3. ← `Navigation.kt:493` passes `{ _ -> Result.failure(NotImplementedError("Wire to listing API")) }` — a placeholder lambda left in during the initial realtor UI scaffolding.
4. ← `PortalListingsRepository.kt` has no `createListing()` method, so the composable never had a real dependency to call.
5. Origin: the realtor CreateListing composable + Navigation entry were landed as UI-only scaffolding (pre-2026-08-30 mobile-native-kmp review); the backend `POST /api/v1/my/listings` handler was added independently in reality-server and was never wired to KMP.

## Test plan
- [ ] KMP unit test at `mobile-native/shared/src/commonTest/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepositoryTest.kt` — mock HttpClient, assert `createListing` issues `POST /api/v1/my/listings` with the correct JSON body and returns the parsed response on 201
- [ ] KMP unit test at `mobile-native/androidApp/src/androidTest/.../CreateListingScreenTest.kt` (or the equivalent Robolectric harness in the repo) — assert that submitting a valid form invokes the repository lambda and calls `onCreated` on success
- [ ] Command: `cd mobile-native && ./gradlew :shared:allTests :androidApp:testDebugUnitTest`

## Out of scope
- iOS SwiftUI create-listing screen (if any) — this plan is Android + shared KMP only
- Backend changes to `create_listing` handler (already implemented)
- Refactoring the wider realtor navigation module (dependency-injection cleanup lives in a separate refactor plan)
- Adding a new screen-map doc for CreateListing (spot-check whether one exists; extending the screen-map is a separate concern)

## After-merge
- Move this file to `plans/_archive/<slug>.md`
- Mark the matching `backlog.json` row as `status: "done"`
