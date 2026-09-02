# code-review-mobile-native-kmp-create-listing-not-wired

**Vector:** bug
**Score:** 4
**Source:** rotating-expert-review 2026-08-30 (dispatcher Tier-1d mobile-native-kmp)
**Confidence:** high

## Hypothesis
The KMP realtor `CreateListingScreen` in the Android app is wired to a `Result.failure(NotImplementedError("Wire to listing API"))` stub in `Navigation.kt:493`, so every publish attempt discards the form and shows `realtor_create_validation_publish_failed`. The backend endpoint (`POST /api/v1/my/listings` → `create_listing`) already exists and is stable; the fix is to add a `createListing(...)` method to `PortalListingsRepository` that calls that endpoint and to replace the stub `onSubmit` in `Navigation.kt` with a call to it. The `CreateListingInput` shape is missing three fields the backend requires (`propertyType`, `street`, `postalCode`), so the form must also collect those before the wiring is truly usable.

## Evidence
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt:493` — `onSubmit = { _ -> Result.failure(NotImplementedError("Wire to listing API")) }` reachable from `AgencyHubScreen` and `MyListingsScreen`.
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepository.kt:40` — `listMyListings(...)` exists; no `createListing(...)` method.
- `backend/servers/reality-server/src/routes/portal_listings.rs:24` route registers `POST /` → `:207 create_listing`; `CreatePortalListingRequest` (`:101`) requires `title, propertyType, transactionType, price, street, city, postalCode` (plus optionals).
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/realtor/CreateListingScreen.kt:392` — `CreateListingInput` currently exposes only `title, description, city, price, currency, transactionType` (form fields at :47–52). Missing `propertyType`, `street`, `postalCode`.

## Files
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt:493`
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/realtor/CreateListingScreen.kt:392`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepository.kt`
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
- If C4 or C5 is ticked → `local` (implementer must run on the user's Mac)
- Otherwise → `cloud-ok` (can run as a claude.ai routine via `ppt-bridge` MCP)

Mode: cloud-ok

## Repro steps
1. Sign in on the KMP Android app as a realtor.
2. From `AgencyHubScreen` or `MyListingsScreen`, tap "New listing" to open `CreateListingScreen`.
3. Fill in `title`, `description`, `city`, `price`, choose `transactionType`.
4. Tap "Publish listing".
5. Expected: the listing is created via `POST /api/v1/my/listings` and the screen pops back to `MyListingsScreen` with the new row visible.
   Actual: `NotImplementedError("Wire to listing API")` bubbles into `Result.failure`, the error banner shows `realtor_create_validation_publish_failed`, and no request is made.

## Suggested approach
1. Extend `CreateListingScreen`'s form to collect `propertyType` (dropdown against `ALLOWED_PROPERTY_TYPES`), `street`, and `postalCode`; add matching fields to `CreateListingInput` (line 392).
2. Add a `suspend fun createListing(input: CreateListingInput): Result<PortalListing>` on `PortalListingsRepository` that maps the input to a JSON body matching `CreatePortalListingRequest` (camelCase serialization already handled by the shared JSON config) and POSTs to `/api/v1/my/listings`. Follow the same `catch(CancellationException) { throw it } / catch(e: Exception)` shape used in `LayoutRepository.kt:65-70` — do not regress the cancellation-swallowed pattern under review in a sibling backlog item.
3. In `Navigation.kt:493`, replace the stubbed `onSubmit` with `{ input -> portalListingsRepository.createListing(input).map { } }` (thread the repository through the composable via the existing DI seam used by `MyListingsScreen`).
4. On success, `onCreated` already pops back to `MyListingsScreen`; verify the my-listings list refreshes (call `refresh()` on the caller-side ViewModel if needed).
5. Add a shared unit test for `PortalListingsRepository.createListing(...)` using the `MockEngine` pattern seen in other repository tests, asserting the request URL, method, body shape, and error mapping.
6. Add an Android instrumentation-lite test (Compose `createComposeRule`) for `CreateListingScreen` that submits a valid form, mocks `onSubmit` to return `Result.success(Unit)`, and asserts `onCreated` fires.

## Alternatives considered
- **Land the wiring against the current 6-field form and rely on backend 400s to surface missing fields** — rejected because `create_portal_listing` requires `property_type`, `street`, `postal_code` as non-null; the flow would appear "wired" while every submission still failed at the server, moving the failure from the composable into a 400 without solving the user problem.
- **Server-side make `propertyType`/`street`/`postalCode` optional to match the current KMP form** — rejected because those columns are stored non-null in the `portal_listings` table (migration 00049) and the ppt-web + reality-web create flows already require them; loosening the server contract would spread the gap into other clients.

## Root-cause trace
1. Symptom: publishing a new listing from KMP Android does nothing; error banner shows `realtor_create_validation_publish_failed`.
2. ← `NotImplementedError("Wire to listing API")` bubbled up from `mobile-native/androidApp/.../navigation/Navigation.kt:493` `onSubmit`.
3. ← `Navigation.kt:493` was intentionally left as a stub because `PortalListingsRepository` has no `createListing(...)` method (see `mobile-native/shared/.../PortalListingsRepository.kt:40` where only `listMyListings` exists).
4. Origin: `CreateListingScreen` composable + navigation wiring landed as UI-only scaffolding (UC-51.4) ahead of the shared-module create method — the backend endpoint has existed since migration 00186 but the KMP repository method was never added.

## Test plan
- [ ] Shared: `mobile-native/shared/src/commonTest/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepositoryTest.kt` — new test `createListing posts CreatePortalListingRequest and returns success` using `MockEngine`, asserting request path, method, and JSON body.
- [ ] Shared: same file — negative test `createListing maps 400 body to Result.failure with server message`.
- [ ] Android: `mobile-native/androidApp/src/androidTest/java/three/two/bit/ppt/reality/ui/realtor/CreateListingScreenTest.kt` — new Compose test that fills the form, taps publish, and asserts `onCreated` is invoked when the submit lambda returns `Result.success`.
- [ ] Regression: same test asserts the pre-fix stub (`NotImplementedError`) would fail — the new test would fail on `dev` today.
- [ ] Local command: `cd mobile-native && ./gradlew shared:allTests androidApp:testDebugUnitTest`.

## Out of scope
- Wizard-style step navigation (Type / Location / Details / Photos / Price) currently rendered as a single-step pane — the plan only adds the missing form fields; the multi-step flow is a separate UX task.
- Photo upload on the create flow.
- Draft persistence beyond the in-memory `remember { mutableStateOf(...) }` values.
- iOS parity (SwiftUI screen) — the shared-module `createListing` method is used by both platforms; the Android UI + navigation are the immediate blocker, iOS follow-up is separate.

## After-merge
- Move this file to `plans/_archive/code-review-mobile-native-kmp-create-listing-not-wired.md`
- Mark the matching `backlog.json` row as `status: "done"`
