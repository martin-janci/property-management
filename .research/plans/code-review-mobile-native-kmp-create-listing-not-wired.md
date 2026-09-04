# code-review-mobile-native-kmp-create-listing-not-wired

**Vector:** bug
**Score:** 4
**Source:** rotating-expert-review 2026-08-30 (dispatcher Tier-1d mobile-native-kmp) + 2026-08-31 dispatcher Tier-1d follow-up
**Confidence:** high

## Hypothesis
Realtor `CreateListingScreen` on the KMP Android app looks functional but the composable's `onSubmit` is wired in `Navigation.kt:491-493` to a `NotImplementedError` stub, so every realtor create-listing attempt fails with a translated validation copy (`realtor_create_validation_publish_failed`) and the entered data is silently discarded. The reality-server already exposes `POST /api/v1/my/listings` (`portal_listings.rs:24` route + `create_listing` at `:207`, registered in `main.rs`), and `PortalListingsRepository` already talks to sibling endpoints on the same base (e.g. `GET /api/v1/my/listings` via `listMyListings`). The smallest fix is a KMP-only change: add a `createListing(input): Result<ListingSummary>` method on `PortalListingsRepository` and hand it to the composable instead of the stub. No backend work needed — issue #2652 (AGP cloud-egress blocker) was closed 2026-09-04T04:32:22Z, so this is now landable in the cloud runner.

## Evidence
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt:491-493` — `CreateListingScreen(... onSubmit = { _ -> Result.failure(NotImplementedError("Wire to listing API")) }, ...)` — reachable from `AgencyHubScreen` (:415) and `MyListingsScreen` (:484).
- `backend/servers/reality-server/src/routes/portal_listings.rs:24` registers `POST /` → `create_listing` (`:207`); route is mounted at `/api/v1/my/listings` from `main.rs`.
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepository.kt` already calls `GET /api/v1/my/listings` (`listMyListings`) but has NO `createListing` method — the fix is a new method on the same repository.
- `PortalListingModels.kt` in the same directory holds the shared request/response DTOs used by list/detail flows; the create-payload shape mirrors the backend `CreateListingRequest`.
- Issue #2652 (mobile-native / KMP tasks unlandable in cloud runner — AGP from `dl.google.com` blocked by egress 403) CLOSED 2026-09-04T04:32:22Z, removing the historical "unlandable in cloud" blocker for this plan.

## Files
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt`
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/realtor/CreateListingScreen.kt`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepository.kt`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/realtor/PortalListingModels.kt`
- `backend/servers/reality-server/src/routes/portal_listings.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

Rationale: KMP unit test on the shared source set + `./gradlew :shared:test` cover the fix. No ADB device or Chrome DOM inspection needed for the repository-level wiring change. Now that issue #2652 is closed, the AGP fetch that used to make `:shared:test` unlandable in the cloud runner should succeed.

## Repro steps
1. Boot the KMP Android app (or run the shared-module unit tests directly).
2. Sign in as a realtor account, navigate `AgencyHubScreen` → *Create listing* (or `MyListingsScreen` → +).
3. Fill any valid listing fields and tap **Publish**.
4. Expected: request to `POST /api/v1/my/listings` succeeds and the new listing appears in the realtor's list.
5. Actual: UI surfaces `realtor_create_validation_publish_failed`; no request is ever made — `onSubmit` returns `Result.failure(NotImplementedError(...))`.

## Suggested approach
1. Add `PortalListingsRepository.createListing(input: CreateListingRequest): Result<PortalListingSummary>` — HTTP POST to `/api/v1/my/listings` using the same `Ktor` client + `NetworkErrorMapper` pattern as `listMyListings`.
2. Add the matching `CreateListingRequest` / response DTOs to `PortalListingModels.kt`, mirroring the reality-server `CreateListingRequest` shape (`portal_listings.rs:~180`).
3. In `Navigation.kt:491-493`, replace the `NotImplementedError` stub with `onSubmit = { input -> portalListingsRepository.createListing(input).map {} }` (or the return-type the composable expects). Take the repository from the same DI wiring that already supplies `ssoService` etc.
4. If `CreateListingScreen.kt` currently constructs its own request DTO, either keep it and thread the repo through, or move the DTO construction into the caller so the composable's `onSubmit` signature matches the repo. Prefer the smaller of the two changes.
5. Add a KMP unit test in `shared/src/commonTest/kotlin/.../realtor/PortalListingsRepositoryTest.kt` (create the file if it doesn't exist) using the existing `MockEngine` pattern: assert the request goes to `POST /api/v1/my/listings` with the expected body, and that a `201` response deserializes to a `PortalListingSummary`.
6. Run `./gradlew :shared:test` to confirm the new test fails against `main` (before step 1's change) and passes after — this is the IG3 evidence.
7. Manually confirm the composable no longer surfaces the placeholder error copy (via existing UI-level test file if one exists, else document under *After-merge* for a follow-up).

## Alternatives considered
- **Fake the wiring in the composable directly (no repo method)** — rejected because the composable then depends on `HttpClient` and duplicates the URL/serialization logic that lives on the repo; every other sibling flow (`listMyListings`, `RealtorRepository`, etc.) goes through a repo, and skipping it here would drift from the KMP layering the reality-portal shared module enforces.
- **Move `create_listing` to a new backend endpoint (`/realtor/listings`)** — rejected because the endpoint already exists at `POST /api/v1/my/listings` and is registered in `main.rs`; adding a second URL for the same operation would create two write paths to the same table, both auth'd differently, for no benefit.

## Root-cause trace
1. Symptom: user taps *Publish* on `CreateListingScreen`; UI shows `realtor_create_validation_publish_failed`; no network call ever leaves the app.
2. ← `CreateListingScreen`'s form calls its provided `onSubmit(input)`; the returned `Result.failure` is what the composable renders.
3. ← `Navigation.kt:491-493` supplies a stub — `{ _ -> Result.failure(NotImplementedError("Wire to listing API")) }` — instead of a real repository call.
4. Origin: `Navigation.kt` was authored with the composable as a UI-only shell; the KMP repository method (`PortalListingsRepository.createListing`) was never added when the backend endpoint was introduced (see `portal_listings.rs:207 create_listing`), so the wiring line was left as a placeholder. Uncovered by the 2026-08-30 dispatcher Tier-1d mobile-native-kmp review.

## Test plan
- [ ] New KMP unit test `mobile-native/shared/src/commonTest/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepositoryTest.kt` — asserts `createListing` issues `POST /api/v1/my/listings` with the expected body and deserializes the `201` response.
- [ ] Regression: keep an existing `MyListingsScreen` / `AgencyHubScreen` UI test (if any) green; the `onSubmit` signature change on `CreateListingScreen` must remain source-compatible with its two callers in `Navigation.kt`.
- [ ] Command: `cd mobile-native && ./gradlew :shared:test` — must fail on the pre-fix code (IG3), pass after.
- [ ] Command: `cd mobile-native && ./gradlew :shared:compileKotlinAndroid :shared:compileKotlinIosSimulatorArm64` (or the workspace's `assemble` alias) to make sure the KMP repository change compiles for both Android and iOS targets.

## Out of scope
- No changes to the backend `create_listing` handler, RLS policy, or migrations.
- No redesign of `CreateListingScreen` UI or the input DTO shape beyond what wiring requires.
- No wiring of the iOS `CreateListingScreen` variant (if one exists) — this plan is Android-nav + shared-repo only. If iOS needs the same wiring, it goes in a follow-up plan referencing the same repo method.
- No fix for the `realtor_create_validation_publish_failed` copy itself — the copy will simply stop rendering once real errors flow through.

## After-merge
- Move this file to `plans/_archive/code-review-mobile-native-kmp-create-listing-not-wired.md`
- Mark the matching `backlog.json` row (`code-review-mobile-native-kmp-create-listing-not-wired`) as `status: "done"` and append the resolving PR number to `sources`.
- Follow-up: if the same stub pattern exists on iOS (`iosApp/…/CreateListingScreen`), file a companion plan referencing this repo method.
