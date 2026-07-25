# code-review-mobile-native-kmp-realtor-create-listing-broken

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review 2026-07-25 (mobile-native-kmp segment) · confirms backend surface at `backend/servers/reality-server/src/routes/portal_listings.rs:207`
**Confidence:** high

## Hypothesis
Android realtor Create Listing (UC-51.4) is fully built end-to-end in the UI — CreateListingScreen renders the form (title/description/city/price/currency/transactionType) with validation and a sticky publish bar, and MyListingsScreen navigates to it — but Navigation.kt wires the composable's `onSubmit` callback to a lambda that unconditionally returns `Result.failure(NotImplementedError("Wire to listing API"))`. Every realtor tap on "Publish" therefore fails silently on Android with no repository call ever issued. The backend endpoint `POST /api/v1/my/listings` (create_listing) already exists and accepts the same fields (via `CreatePortalListingRequest`), and `PortalListingsRepository` in shared/ already handles auth + baseUrl for the sibling reads (`listMyListings`, `getPortfolioAnalytics`). The smallest safe change is to add a `createListing(input: CreateListingInput): Result<Unit>` suspend method on `PortalListingsRepository`, wire Navigation.kt:476 to call it, and add a repository-level Ktor MockEngine test asserting a `POST /api/v1/my/listings` fires with the expected JSON body.

## Evidence
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt:476` — `onSubmit = { _ -> Result.failure(NotImplementedError("Wire to listing API")) }`
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/realtor/CreateListingScreen.kt:39-41,392-399` — screen expects `suspend (CreateListingInput) -> Result<Unit>`; `CreateListingInput(title, description, city, price, currency, transactionType)` is already defined.
- `backend/servers/reality-server/src/routes/portal_listings.rs:99-116,207` — `POST /api/v1/my/listings` accepts `CreatePortalListingRequest { title, description, propertyType, transactionType, price, currency, street, city, postalCode, country, sizeSqm?, rooms?, floor?, totalFloors? }` (camelCase, gated by `PortalPrincipal`).
- No `CreateListingScreen*Test` / `NavigationTest` file exists anywhere under `mobile-native/` — the broken route has zero coverage. (`find mobile-native -name '*CreateListing*Test*'` → empty.)
- `PortalListingsRepository.kt:38-70` — the sibling `listMyListings` already carries the exact HTTP + auth pattern this fix reuses.

## Files
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepository.kt`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/realtor/PortalListingModels.kt`
- `mobile-native/shared/src/commonTest/kotlin/three/two/bit/ppt/reality/realtor/PortalListingsRepositoryTest.kt`
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt:473`
- `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/realtor/CreateListingScreen.kt:392`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode:** `Mode: cloud-ok` — Gradle test only for `shared/` (KMP), no Android device needed. The Kotlin JVM tests under `shared/src/commonTest/` run headless.

## Repro steps
1. Read `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt:473-479`. Observe the `Screen.CreateListing` route wires `onSubmit` to `{ _ -> Result.failure(NotImplementedError("Wire to listing API")) }`.
2. Read `mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/realtor/CreateListingScreen.kt:39-125`. Follow the code path from tapping "Publish": `onSubmit(CreateListingInput(...))` is invoked → wired lambda returns `Result.failure` → snackbar shows a generic error → no repository call ever fires. Expected: repository POSTs to `/api/v1/my/listings` and, on 201, `onCreated` pops back to MyListings. Actual: form is unusable on Android.

## Suggested approach
1. In `PortalListingModels.kt`, add `data class CreatePortalListingRequest(val title, val description, val propertyType, val transactionType, val price, val currency, val street, val city, val postalCode, val country, val sizeSqm, val rooms, val floor, val totalFloors)` matching the backend DTO (kotlinx-serialization, camelCase). Keep optional fields nullable to mirror `Option<T>`; use `String` (or a `BigDecimal`-analog via `@Serializable(with = ...)`) for `price` — server accepts a JSON number/string via `rust_decimal`.
2. In `PortalListingsRepository.kt`, add `suspend fun createListing(input: CreatePortalListingRequest): Result<PortalListing>` — POST `$baseUrl/api/v1/my/listings`, set `configureRequest()` for the bearer token, `contentType(ContentType.Application.Json)`, and `setBody(input)`. Map 201 → success, 401 → `PortalListingException("Please sign in ...")`, 400 → `PortalListingException("Validation error: ${response.bodyAsText()}")`, else generic failure — same shape as `listMyListings`.
3. In Navigation.kt (~line 473), replace the stub lambda: convert `CreateListingInput` → `CreatePortalListingRequest` (default `street="",postalCode=""` for now with a `TODO` note tracked separately if UI doesn't collect them, OR extend the UI form to collect them; see *Out of scope*), then `portalListingsRepository.createListing(request).map { Unit }`.
4. If the UI's `CreateListingInput` omits fields the backend requires (`street`, `postalCode`, `propertyType`), either (a) add those inputs to `CreateListingScreen.kt` (preferred — the backend rejects missing required fields), or (b) explicitly file a follow-up plan for the UI extension. Pick (a) for this plan since the fields are trivial `OutlinedTextField` additions.
5. Add a `commonTest` in `PortalListingsRepositoryTest.kt` using Ktor `MockEngine`: assert the POST hits `/api/v1/my/listings`, carries a bearer-token `Authorization` header, has a JSON body containing the input fields; mock a 201 with a `PortalListing` payload and assert the success branch surfaces it. Add a second case: mock 401 → assert the failure message matches `PortalListingException("Please sign in to view your listings")` pattern (or a create-specific variant if introduced).
6. `./gradlew :shared:allTests` should exercise the new repository test on JVM.
7. Manual verification (optional — requires ADB / a device, not part of `cloud-ok` gates): install debug APK, log in as a realtor, tap "Create Listing" → fill fields → tap Publish → verify 201 in server logs and the listing appears in MyListings.

## Alternatives considered
- **Delete Navigation.kt Screen.CreateListing and hide the entry point on MyListingsScreen** — rejected because CreateListingScreen.kt is a fully-designed 400-line form with validation and a sticky publish bar; deleting the wire, not the UI, would leave a dead-code footprint AND remove a shipped user story (UC-51.4 realtor listing creation). The stubbed lambda tells us the wiring — not the feature — is what was deferred.
- **Wire `onSubmit` directly to raw Ktor client in Navigation.kt without a repository method** — rejected because it duplicates auth + baseUrl plumbing that already lives in `PortalListingsRepository` (sibling methods `listMyListings`, `getPortfolioAnalytics` set the pattern) and would leave Android and iOS on divergent code paths; adding one repository method keeps KMP shared-first.

## Root-cause trace
1. Symptom: on Android, tapping "Publish" in the realtor CreateListingScreen surfaces a generic snackbar error and the listing is never created.
2. ← Navigation.kt:476 wires the composable's `onSubmit` to a lambda that unconditionally returns `Result.failure(NotImplementedError("Wire to listing API"))`.
3. ← `PortalListingsRepository` (shared) has no `createListing` method — only reads (`listMyListings`, `getPortfolioAnalytics`, listing detail). The write side of UC-51.4 was never implemented on the mobile side despite the backend `POST /api/v1/my/listings` shipping with reality-server.
4. Origin: `CreateListingScreen.kt` + `Screen.CreateListing` route landed as a UI-first slice with an explicit `NotImplementedError` TODO stub in place of the repository call. The stub sat past the "wire" phase without a follow-up. `git log --all -- mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/navigation/Navigation.kt` will name the introducing commit.

## Test plan
- [ ] Unit test: `PortalListingsRepositoryTest.createListing_postsExpectedBodyAndSurfacesSuccess()` — MockEngine asserts POST verb, path, Authorization header, JSON body; feeds 201 → asserts `Result.success` unwraps to a `PortalListing`.
- [ ] Unit test: `PortalListingsRepositoryTest.createListing_401MapsToAuthException()` — MockEngine returns 401 → asserts failure is a `PortalListingException` with the sign-in-required message.
- [ ] Regression: run the whole shared suite (`./gradlew :shared:allTests`) to confirm no other portal-listing test regresses when `CreatePortalListingRequest` model + serializer are introduced.
- [ ] IG3 gate: the two new tests would have been red on `main` (no method exists) → become green with the fix.
- [ ] Command: `cd mobile-native && ./gradlew :shared:jvmTest --tests three.two.bit.ppt.reality.realtor.PortalListingsRepositoryTest`

## Out of scope
- iOS wiring of the same route — file a sibling plan once the shared `createListing` method lands; iOS `MyListingsView` will need the equivalent hookup.
- Backfilling analytics events on realtor create-listing (`listing.created` for the reality-portal analytics bus added by PR #2541) — file separately once the wire is proven.
- Broader form validation UX (Slovak/Czech/German translations for new error messages beyond the existing generic string, richer error mapping from 400 validation-issue JSON) — the fix targets the wiring gap, not full UX polish.
- Extending `CreateListingInput` to collect `street`/`postalCode` if the form doesn't already — if this becomes needed, either add the two fields inline (trivial) or file a follow-up "extend form to backend-required address fields" plan; do NOT ship an incomplete DTO that the backend will 400 on.

## After-merge
- Move this file to `plans/_archive/code-review-mobile-native-kmp-realtor-create-listing-broken.md`
- Mark the matching `backlog.json` row as `status: "done"`
