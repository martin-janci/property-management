# code-review-mobile-native-kmp-create-listing-not-wired

**Vector:** bug
**Score:** 3
**Source:** code review (backlog `code-review-mobile-native-kmp-create-listing-not-wired`), commit eea70673
**Confidence:** high

## Hypothesis
The KMP realtor **Create Listing** flow renders and submits a form, but the Android `Navigation.kt` wires `onSubmit` to `Result.failure(NotImplementedError("Wire to listing API"))`. Whenever a portal user fills the form and taps *Create*, their data is discarded and the UI surfaces a generic failure. The reality-server already exposes `POST /api/v1/my/listings` (`portal_listings.rs`); the repository just needs a `createListing` method and the navigation call site needs to invoke it.

## Evidence
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt:493` — `onSubmit = { _ -> Result.failure(NotImplementedError("Wire to listing API")) }`
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/realtor/CreateListingScreen.kt:41` — `onSubmit: suspend (CreateListingInput) -> Result<Unit>` — screen contract already exists
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepository.kt` — no `createListing` / `create` method on the repository
- `backend/servers/reality-server/src/routes/portal_listings.rs:195-243` — `POST /api/v1/my/listings` handler already accepts a `CreateListingRequest` body

## Files
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt:493`
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/realtor/CreateListingScreen.kt`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepository.kt`
- `backend/servers/reality-server/src/routes/portal_listings.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [x] C5 — ADB device (KMP Android flow needs on-device verification)
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: local-only (reason: C5 — Android navigation + Compose UI needs on-device verification, and cloud verify gate for mobile-native is currently blocked by AGP / jest-expo egress — issue #2951)

## Repro steps
1. Log into the KMP Android app as a portal realtor.
2. From the realtor dashboard tap **My Listings → Create Listing**.
3. Fill every required field (title, price, address, postal_code, area, rooms, listing_type, property_type, description) and tap **Create**.
4. Expected: HTTP `POST /api/v1/my/listings` fires; a new listing appears in *My Listings*.
5. Actual: The submit button spins briefly, then a generic error surfaces; no request leaves the device (verify with logcat / mitmproxy). The typed data is discarded.

## Suggested approach
1. In `PortalListingsRepository.kt` add `suspend fun createListing(input: CreateListingInput): Result<Unit>` that POSTs to `/api/v1/my/listings` via the shared `ApiClient`. Serialize the input using a new `CreateListingRequest` DTO that mirrors the backend `CreateListingRequest` in `portal_listings.rs`.
2. Add a `CreateListingRequest` `@Serializable` data class in a new `PortalListingsModels.kt` (or extend the existing repository file) with fields matching the backend contract: `title`, `price`, `address`, `postal_code`, `area`, `rooms`, `listing_type`, `property_type`, `description` (align nullability with the Rust struct).
3. Map from `CreateListingScreen.kt`'s `CreateListingInput` (line 392) to `CreateListingRequest` inside the repository (keep the UI DTO decoupled from the wire format).
4. In `Navigation.kt:490-493`, obtain the shared `PortalListingsRepository` (same DI style used for `MyListingsScreen` — see composable `Screen.MyListings.route` block), then replace the `NotImplementedError` lambda with `onSubmit = { input -> repo.createListing(input) }`.
5. Surface backend validation failures: when the response body carries a per-field error, return `Result.failure(...)` with a typed exception the screen already renders (`CreateListingScreen` currently shows a toast on `Result.failure`).
6. On success (`Result.success(Unit)`), pop back to `MyListings` and trigger a refresh (`MyListingsViewModel.refresh()` if one exists, else navigate with `popUpTo(Screen.MyListings.route) { inclusive = false }` so the list re-fetches on resume).

## Alternatives considered
- **Do the wiring in the iOS SwiftUI layer only** — rejected because the bug is Android-side; the shared `commonMain` repository is where all platforms will eventually consume the endpoint, so the fix must land in `shared` too. iOS then reuses `PortalListingsRepository.createListing`.
- **Add the POST call directly inside `CreateListingScreen` composable** — rejected because it duplicates HTTP wiring in the UI layer and bypasses the KMP repository pattern used everywhere else in the module.

## Root-cause trace
1. Symptom: `Create Listing` submit silently fails; a `NotImplementedError` is thrown from the `onSubmit` lambda passed to `CreateListingScreen`.
2. ← Immediate cause at `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt:493` — the lambda returns `Result.failure(NotImplementedError(...))` unconditionally.
3. ← Upstream cause at `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepository.kt` — no `createListing` method exists on the repository, so `Navigation.kt` had nothing to call.
4. Origin: the initial KMP `CreateListingScreen` shipped with the screen wired to a stub while the backend endpoint was in progress; the backend landed (`portal_listings.rs:195` `POST /api/v1/my/listings`) but the Android wiring was never completed.

## Test plan
- [ ] Add `PortalListingsRepositoryTest.createListing_postsExpectedBody` in `mobile-native/shared/src/commonTest/kotlin/.../realtor/PortalListingsRepositoryTest.kt` using Ktor `MockEngine` to assert the request body matches the backend contract and that a `201` response yields `Result.success(Unit)`.
- [ ] Add an Android instrumentation regression: `NavigationCreateListingTest` in `mobile-native/androidApp/src/androidTest/...` that renders `Navigation` with a fake repository and verifies tapping submit produces a matching `createListing` call (no `NotImplementedError`).
- [ ] Manual smoke: run the app on a device against a dev reality-server; create a listing end-to-end; confirm the new listing renders in `MyListings` after refresh.
- [ ] Local command: `cd mobile-native && ./gradlew :shared:allTests :androidApp:testDebugUnitTest`.

## Out of scope
- iOS SwiftUI create-listing wiring (tracked separately once shared repo method lands — this plan only unblocks Android).
- Additional validation UX (client-side field validation beyond what the current screen already does).
- New backend endpoints or DTO changes on `portal_listings.rs`.

## After-merge
- Move this file to `plans/_archive/code-review-mobile-native-kmp-create-listing-not-wired.md`
- Mark the matching `backlog.json` row as `status: "done"`
