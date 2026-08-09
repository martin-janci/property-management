# code-review-mobile-native-kmp-create-listing-submit-stub

**Vector:** bug
**Score:** 4
**Source:** Tier1d dev-review 2026-08-08 + 2026-08-09 (segment `mobile-native-kmp`, companion signals `code-review-mobile-native-kmp-create-listing-submit-stub` + `code-review-mobile-native-kmp-createlisting-notimpl-stub`)
**Confidence:** high

## Hypothesis
The realtor "Create listing" flow in the Reality Portal Android app is fully wired at the UI layer — `CreateListingScreen.kt` collects a complete `CreateListingInput`, `MyListingsScreen` navigates to it via `onCreateClick` — but the navigation composable in `Navigation.kt:490-499` injects a **hardcoded** `onSubmit = { _ -> Result.failure(NotImplementedError("Wire to listing API")) }`, so every submission fails unconditionally in production. The root cause is that `PortalListingsRepository` has no `create*` method — grep of the class returns only `listMyListings` / `getListingAnalytics` / `getPortfolioAnalytics`. The smallest correct change is to add `suspend fun createListing(input: CreateListingInput): Result<Listing>` to `PortalListingsRepository` (backed by `POST /api/v1/my/listings` via the existing `PortalApi` client) and wire it into `Navigation.kt:490` so realtors can actually create listings; alternatively, hide the `MyListingsScreen.onCreateClick` entry point behind a feature flag until the repository method lands so users don't hit a dead-end.

## Evidence
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt:490-499` — `CreateListing` composable wires the screen with `onSubmit = { _ -> Result.failure(NotImplementedError("Wire to listing API")) }`; entry point at `MyListingsScreen.onCreateClick` (line 484) navigates to it.
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/realtor/CreateListingScreen.kt` — 399-line implementation, `onSubmit: suspend (CreateListingInput) -> Result<Unit>` (line 41), collects the full `CreateListingInput` data class, renders `onSuccess -> onCreated` — all wasted because the injected callback always fails.
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepository.kt` — `grep "fun create"` returns nothing; the class exposes only `listMyListings`, `getListingAnalytics`, `getPortfolioAnalytics`. No POST-side method.
- Tier1d signal `code-review-mobile-native-kmp-create-listing-submit-stub` (2026-08-08) — original finding.
- Tier1d signal `code-review-mobile-native-kmp-createlisting-notimpl-stub` (2026-08-09) — independent second-pass confirmation, same file:line, same root cause. Two independent reviews landing on the identical finding raises confidence to high.

## Files
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepository.kt`
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

Mode: local-only (reason: C5 — needs ADB device to install the debug APK and drive the realtor Create-listing flow to green)

## Repro steps
1. `./gradlew :androidApp:installDebug` on a device authenticated as a realtor account.
2. Navigate: Realtor tab → "My listings" → tap the FAB / "Create listing" entry point (`MyListingsScreen.onCreateClick`, Navigation.kt:484).
3. Fill in the full `CreateListingInput` form and tap Submit.
4. Expected (with fix): a `POST /api/v1/my/listings` is issued, returns 201 with a listing id, screen navigates to the new listing detail.
5. Actual (today): submission fails with `NotImplementedError("Wire to listing API")` — screen shows an error toast; no request is issued.

## Suggested approach
1. Add `suspend fun createListing(input: CreateListingInput): Result<Listing>` to `PortalListingsRepository`, calling the existing `PortalApi` (the same Ktor client used by `listMyListings`). Map upstream 4xx/5xx to the standard `Result.failure` shape the rest of the repo uses.
2. Add the matching endpoint call to whichever generated / hand-written API client `PortalApi` provides. If the endpoint is not already in the generated openapi client, extend the OpenAPI spec (docs/api/typespec/) and regenerate — but only if it does not already exist server-side; verify against `backend/servers/reality-server/src/routes/listings.rs` (or the realtor-facing path) first.
3. In `Navigation.kt:490-499`, replace the hardcoded `NotImplementedError` with `onSubmit = { input -> portalListingsRepository.createListing(input).map { } }` (via the same DI shape the other composables use).
4. Confirm `CreateListingInput` → API-request-body mapping is 1:1 with the server contract (types, required fields, enums).
5. Add a shared-module unit test asserting `PortalListingsRepository.createListing` posts to the expected path and parses the success body.
6. Verify on device: complete a submission end-to-end, assert the listing appears in `listMyListings` on refresh and in the realtor's portfolio analytics.

## Alternatives considered
- **Hide `CreateListing` entry until the repo method lands** — rejected because the screen is a fully-built 399-line implementation and gating the entry point rather than wiring it wastes complete UI work; the missing piece is a single repo method + navigation callback, not a redesign.
- **Add a client-only mock POST that always succeeds and log a warning** — rejected because it would give realtors a false success signal and lose their form input; the failure mode today (visible error) is better than a silent lie.

## Root-cause trace
1. Symptom: every realtor "Create listing" submission fails with `NotImplementedError("Wire to listing API")`; no HTTP request is made.
2. ← `Navigation.kt:493` injects `onSubmit = { _ -> Result.failure(NotImplementedError(...)) }` as the callback.
3. ← `PortalListingsRepository` has no `createListing` method; the shipping repository is read-only for realtor portfolio queries.
4. Origin: `CreateListingScreen` and its navigation route landed as a UI-first slice with the repository method scheduled as a follow-up commit that never came. `git log --follow mobile-native/androidApp/.../Navigation.kt` on line 490-499 identifies the introducing commit — expect a commit with subject matching "CreateListing" whose diff adds the screen + route but no repo method.

## Test plan
- [ ] `mobile-native/shared/src/commonTest/kotlin/.../realtor/PortalListingsRepositoryTest.kt` — assert `createListing(sampleInput)` issues the expected POST and returns `Result.success(listing)` on a stubbed happy-path response.
- [ ] Regression scenario: submission via the on-device flow returns success, the new listing appears in `listMyListings` after refresh, and the realtor's portfolio-analytics view reflects the new item.
- [ ] Exact commands to run locally: `cd mobile-native && ./gradlew :shared:allTests :androidApp:testDebugUnitTest` (per-module unit tests) plus a manual on-device smoke of the flow.

## Out of scope
- iOS side of the same flow (SwiftUI wiring) — file a follow-up if the iOS `CreateListing` screen is similarly stubbed; this plan is Android-first because that's what the tier1d finding cited.
- Server-side `POST /api/v1/my/listings` behavior — assumed to exist and work; if it doesn't, a separate reality-server plan is needed and this plan blocks on it.

## After-merge
- Move this file to `plans/_archive/code-review-mobile-native-kmp-create-listing-submit-stub.md`
- Mark the matching `backlog.json` row as `status: "done"`
