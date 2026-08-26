# code-review-mobile-native-kmp-create-listing-not-implemented

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review 2026-08-24 (mobile-native-kmp)
**Confidence:** high

## Hypothesis
The Android Reality Portal wires `Screen.CreateListing` to a lambda that unconditionally returns `Result.failure(NotImplementedError("Wire to listing API"))` — every tap of the *Publish* button in `CreateListingScreen` fails with a generic error banner, even though the KMP shared module has no `POST /api/v1/my/listings` client method. UC-51.4 (Realtor: publish a new listing) is entry-point-reachable in production yet functionally non-deliverable in the Android app. The smallest fix adds `PortalListingsRepository.createListing()` calling the existing reality-server route, maps the composable input to the wire request, and replaces the stub lambda with a real dispatch.

## Evidence
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt:490-495` — `Screen.CreateListing` composable passes `onSubmit = { _ -> Result.failure(NotImplementedError("Wire to listing API")) }` to `CreateListingScreen`.
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt:484` — `MyListingsScreen`'s "Create" button is wired to `navController.navigate(Screen.CreateListing.route)`; the CreateListing entry is reachable in production without any feature gate.
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepository.kt` — exposes only `listMyListings` / `getListingAnalytics` / `getPortfolioAnalytics`; `grep -rn 'createListing\|CreateListing' mobile-native/shared` returns zero hits.
- `backend/servers/reality-server/src/routes/portal_listings.rs:24` — the target route already exists: `Router::new().route("/", post(create_listing))` under `/api/v1/my/listings`, `create_listing` handler at `:207`.

## Files
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt:490`
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/realtor/CreateListingScreen.kt`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepository.kt`
- `mobile-native/shared/src/commonTest/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepositoryTest.kt`
- `backend/servers/reality-server/src/routes/portal_listings.rs:207`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [x] C5 — ADB device (KMP Android surface — needs on-device smoke to confirm end-to-end publish flow)
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):**
Mode: local-only (reason: C5 — mobile-native Android build + on-device verification of the Publish flow)

## Repro steps
1. Build + install `mobile-native/androidApp` on an emulator or device signed in as a realtor account.
2. From the realtor dashboard tap "My listings" → "Create" → fill the CreateListingScreen form → tap **Publish**.
3. Expected: a new listing appears under "My listings" and the composable dismisses back to the list.
4. Actual: the form shows the generic "publish failed" / network-error banner (`CreateListingScreen.kt:120-125`) with no listing created; `logcat | grep NotImplementedError` shows the stub throwing.

## Suggested approach
1. Add `suspend fun createListing(input: CreateListingRequest): Result<PortalListing>` to `PortalListingsRepository` (shared), matching the reality-server contract at `backend/servers/reality-server/src/routes/portal_listings.rs:207` — reuse the existing `Result<T>` idiom already used by `listMyListings`.
2. Introduce a `CreateListingRequest` DTO in `mobile-native/shared/.../realtor/PortalListingsModels.kt` (or the nearest existing sibling models file) with `@Serializable` fields mirroring the server's `CreateListingBody` — start from the fields the composable already collects in `CreateListingScreen` state so the mapper stays local.
3. In `CreateListingScreen.kt` (or a small `CreateListingViewModel` if one already exists in the account/inquiries siblings), inject the repository and expose `onPublish(): suspend () -> Result<Unit>`; delegate to `repository.createListing(state.toRequest())` and map the `Result` back to the existing success/error UI branches.
4. Replace the stub in `Navigation.kt:490-495` with a real dispatch — either construct the ViewModel at the composable's DI seam or thread the repository through the existing `NavGraph` factory used by `MyListingsScreen`.
5. On success: `navController.popBackStack()` back to `MyListings` and trigger the list's existing refresh (mirroring the pattern from `EditProfileScreen` if present in the codebase, else a simple `LaunchedEffect` re-fetch).
6. On failure: surface the server error via the existing error banner path; do NOT swallow — the current banner shape already renders `state.errorMessage`.
7. Add / extend `PortalListingsRepositoryTest.createListing_happy_path` and one HTTP-error path using the KMP Ktor mock engine already used by sibling tests.

## Alternatives considered
- **Feature-gate `Screen.CreateListing` off (hide the button + guard the route)** — rejected because the route + form already ship to production; hiding the button ships the CTA to zero users but leaves dead composable + dead nav wiring around, and defers the actual UC-51.4 delivery further. The end state we want is a working publish, not a hidden stub.
- **Wrap the existing lambda in a `TODO()`/toast telling users the feature is disabled** — rejected because it converts a silent failure into a slightly less silent failure while adding a message users cannot act on; the backend route is ready today, so the wiring is the smaller change.

## Root-cause trace
1. Symptom: tapping Publish on Android surfaces a generic error banner and no listing is created (`mobile-native/androidApp/.../ui/realtor/CreateListingScreen.kt:120-125`).
2. ← Immediate cause: the `onSubmit` lambda passed to `CreateListingScreen` returns `Result.failure(NotImplementedError(...))` unconditionally (`mobile-native/androidApp/.../navigation/Navigation.kt:490-495`).
3. ← Upstream cause: the KMP shared surface has no `createListing()` method to wire the lambda to (`mobile-native/shared/.../realtor/PortalListingsRepository.kt` exposes only read methods).
4. Origin: composable + route were introduced without a paired shared-module call — the placeholder lambda was left as a `TODO: Wire to listing API` and never replaced.

## Test plan
- [ ] Add `PortalListingsRepositoryTest.createListing_returns_success_on_201` in `mobile-native/shared/src/commonTest/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepositoryTest.kt` using the Ktor `MockEngine` sibling tests already use.
- [ ] Add `PortalListingsRepositoryTest.createListing_maps_4xx_to_failure_result` covering a `400` server response — asserts the failure surfaces to the composable as an error result rather than an exception.
- [ ] Manual on-device regression: fresh install → sign in as realtor → complete CreateListing form → observe a new listing appears in `MyListingsScreen`.
- [ ] Command: `cd mobile-native && ./gradlew :shared:allTests` (green); then `./gradlew :androidApp:assembleDebug` (build proof).

## Out of scope
- iOS SwiftUI parity — the Android composable is the entry point that ships today; iOS wiring is a follow-up sized separately.
- New CreateListingRequest server-side field additions — this plan wires the existing contract, no server changes.
- Draft-save / autosave for the CreateListingScreen form — the current UC is publish-only.
- Fixing the sibling `Result.failure(NotImplementedError(...))` lambdas in `Navigation.kt` for other stubbed screens (if any) — narrow to the CreateListing path.

## After-merge
- Move this file to `plans/_archive/code-review-mobile-native-kmp-create-listing-not-implemented.md`
- Mark the matching `backlog.json` row as `status: "done"`
