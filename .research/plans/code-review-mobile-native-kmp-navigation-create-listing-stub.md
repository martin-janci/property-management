# code-review-mobile-native-kmp-navigation-create-listing-stub

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review 2026-07-24 (mobile-native-kmp tier1d) — Kotlin expert
**Confidence:** high

## Hypothesis
The Android realtor "Create listing" flow is dead code in production: the CreateListing route's `onSubmit` is wired to `Result.failure(NotImplementedError("Wire to listing API"))`, and no `createListing()` method exists on the shared `ApiClient` or `ListingRepository`. Users on `MyListings → Create → Submit` see the "publish failed" banner unconditionally — the form is fully built but silently non-functional. The backend endpoint (`POST /api/v1/my/listings` in `reality-server`) is already live, so the fix is a straight KMP wiring: add `createListing()` to the shared client + repo, then route the composable callback to it.

## Evidence
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt:473-479` — CreateListing route wires `onSubmit = { _ -> Result.failure(NotImplementedError("Wire to listing API")) }`
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/realtor/CreateListingScreen.kt:120-126` — the failure gets folded into the "publish failed" banner path; no crash, but no success either
- `grep 'suspend fun create' mobile-native/shared/src/commonMain/kotlin/` returns `createInquiry`, `createSavedSearch`, `createMobileToken` — no `createListing`
- `backend/servers/reality-server/src/routes/portal_listings.rs:195-207` — `POST /api/v1/my/listings` (`create_listing`) is live and calls `portal_create_listing`
- Re-verified 2026-07-24 in this run: the stub is still present at the same line

## Files
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt:476`
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/realtor/CreateListingScreen.kt`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/api/ApiClient.kt`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/listing/ListingRepository.kt`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device (not needed — unit tests + shared-module compile is sufficient)
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):**

Mode: cloud-ok

## Repro steps
1. Boot the Android app as a realtor user with at least one property listed.
2. `MyListings → Create → fill the form → Submit`.
3. Expected: listing is persisted (backend `POST /api/v1/my/listings` returns 201) and the app pops back to `MyListings` with the new row visible.
4. Actual: the "publish failed" banner appears; no HTTP request is made; nothing lands in the DB.

## Suggested approach
1. Add a `CreateListingRequest` DTO to `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/listing/` matching the backend `portal_create_listing` payload shape (mirror what `CreateListingScreen`'s form already collects). Regenerate via the KMP OpenAPI client if that's the codebase pattern — check `mobile-native/gradle/libs.versions.toml` for the generator setup.
2. Add `suspend fun createListing(request: CreateListingRequest): Result<ListingDetail>` to `ApiClient.kt` — POST `$baseUrl/api/v1/my/listings`, auth-required (mirror the auth handling in the existing `createInquiry` path).
3. Add a passthrough `suspend fun createListing(request: CreateListingRequest): Result<ListingDetail>` on `ListingRepository.kt` that delegates to `ApiClient.createListing`.
4. In `Navigation.kt:476`, inject a `ListingRepository` (via `viewModel()` or the existing DI wiring — check how sibling `MyListings` composable resolves its dependencies) and replace the stubbed `onSubmit` with a lambda that maps the form input to `CreateListingRequest`, calls `repo.createListing(...)`, and returns the `Result` unmodified so the existing success/failure branching in `CreateListingScreen` continues to work.
5. Add a `LoggingListingEvent` or equivalent telemetry hook if `createInquiry` has one (parity check in `InquiryRepository:131`); otherwise skip.
6. Verify iOS parity: check `mobile-native/iosApp/**` — if iOS wires the same route through Compose Multiplatform / SwiftUI, it likely shares the shared-module fix; no separate iOS work needed. Note explicitly in the PR body if iOS is unaffected.

## Alternatives considered
- **Regenerate the whole KMP client from OpenAPI** — rejected because the mobile-native OpenAPI generator config is a targeted subset (only listings-search / auth flows are generated today); expanding the generator surface is a larger refactor and would balloon the diff.
- **Ship a UI change to hide/disable the "Create listing" CTA until the API is wired** — rejected because the CTA and the whole form are already user-visible on shipped Android builds; disabling them mid-flight would look like a regression to realtors and the backend endpoint is ready today.

## Root-cause trace
1. Symptom: `MyListings → Create → Submit` on Android always shows "publish failed"; no network request fires.
2. ← `CreateListingScreen.kt:120-126` folds the caller-supplied `onSubmit` failure into the banner path.
3. ← `Navigation.kt:476` supplies a stub `Result.failure(NotImplementedError("Wire to listing API"))` instead of a real repository call.
4. ← `ApiClient.kt` / `ListingRepository.kt` never grew a `createListing()` sibling to `getListings` / `createInquiry`; when `CreateListingScreen` was built the API layer wasn't extended and the nav wire-up left a "wire later" stub that never got wired.
5. Origin: cannot pin exactly without git blame, but the stub predates the current review window; grep of the string "Wire to listing API" returns only the one call site, so this is a single-point-of-truth omission (never a regression).

## Test plan
- [ ] Add a KMP `commonTest` for `ListingRepository.createListing()` using a mock/fake ApiClient — assert success maps to `ListingDetail`, error paths preserve the `Result.failure` shape (mirror the existing `InquiryRepositoryTest` pattern under `mobile-native/shared/src/commonTest/kotlin/three/two/bit/ppt/reality/inquiry/`).
- [ ] Add a Compose UI test in `mobile-native/androidApp/src/androidTest/` (or the closest existing UI-test root) that renders `CreateListingScreen` with a stub repo returning `Result.success(...)` and asserts the success navigation callback fires — this is the failing-on-main regression test (IG3) for the wiring.
- [ ] `cd mobile-native && ./gradlew :shared:allTests :androidApp:testDebugUnitTest` — must be green.

## Out of scope
- iOS-specific wiring beyond the shared module (if iOS is affected the fix should ride the same commit; if not, note it).
- Backend changes — `POST /api/v1/my/listings` is already live and unaffected.
- Any UI polish on `CreateListingScreen` — the form itself is already fully built.
- Analytics/telemetry expansion beyond parity with the existing `createInquiry` path.

## After-merge
- Move this file to `plans/_archive/code-review-mobile-native-kmp-navigation-create-listing-stub.md`
- Mark the matching `backlog.json` row as `status: "done"`
