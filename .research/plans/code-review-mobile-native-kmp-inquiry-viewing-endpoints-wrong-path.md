# code-review-mobile-native-kmp-inquiry-viewing-endpoints-wrong-path

**Vector:** bug
**Score:** 3
**Source:** mobile-native-kmp segment review 2026-08-23
**Confidence:** high

## Hypothesis
The KMP `InquiryRepository` sends inquiries, replies, and viewing requests to URLs the
reality-server backend does not expose. `createInquiry` POSTs to `/api/v1/inquiries` (server wants
`/api/v1/inquiries/contact/{listing_id}`); `replyToInquiry` POSTs to
`/api/v1/inquiries/{id}/replies` (server wants `/api/v1/inquiries/{id}/respond`); the whole
viewings surface (`getViewings`, `scheduleViewing`, `cancelViewing`) hits `/api/v1/viewings*` —
there is no `/viewings` nest in `reality-server/src/main.rs` at all. Every one of these five KMP
call sites returns HTTP 404 against the real server: users cannot send an inquiry, cannot reply
to one, and cannot see, schedule, or cancel a viewing. Tests pass because they run against a
`MockEngine` that never checks the path against the real router. The fix rewrites the KMP calls to
match the actual reality-server routes and, for the `/viewings` GET/DELETE, either replaces them
with the existing `/api/v1/viewings/*` router that lives at `backend/servers/reality-server/src/routes/viewings.rs`
(if it exists) or adds the missing server routes when the mobile flow requires them.

## Evidence
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/inquiry/InquiryRepository.kt:134` — `client.post("$baseUrl/api/v1/inquiries") { setBody(request) }`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/inquiry/InquiryRepository.kt:153` — `client.post("$baseUrl/api/v1/inquiries/${inquiryId.asPathSegment()}/replies")`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/inquiry/InquiryRepository.kt:176,197,216` — `client.get/post/delete("$baseUrl/api/v1/viewings…")`
- `backend/servers/reality-server/src/routes/inquiries.rs:181-190` — router mounts `POST /contact/{listing_id}`, `POST /viewing/{listing_id}`, `GET /`, `GET /{id}`, `PUT /{id}/read`, `POST /{id}/respond`, all under the `/api/v1/inquiries` nest — no `/replies` sub-route, no bare `/inquiries` POST
- `grep -rn "nest.*viewings\|\.route.*viewings" backend/servers/reality-server/src/main.rs` returns 0 matches — the `/api/v1/viewings` prefix is not mounted

## Files
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/inquiry/InquiryRepository.kt`
- `backend/servers/reality-server/src/routes/inquiries.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

Mode: cloud-ok

## Repro steps
1. Point a debug KMP build at a real reality-server (a local `stack up pm-local` instance on
   `:8081`, or the ops team's shared dev host), signed in as a portal user.
2. From the shared code (or the Android/iOS inquiry UI), call
   `InquiryRepository(baseUrl, token).createInquiry(CreateInquiryRequest(listingId, message))`.
3. Expected: HTTP 201 with a `CreateInquiryResponse`. Actual: HTTP 404, the `Result` fails with
   `InquiryException("Failed to send inquiry: 404 Not Found")`, and the app surfaces the generic
   "could not send" error state. The same 404 repeats for `replyToInquiry`, `getViewings`,
   `scheduleViewing`, and `cancelViewing`.

## Suggested approach
1. In `InquiryRepository.kt` rewrite each URL to match `routes/inquiries.rs`:
   - `createInquiry(request)` → `POST /api/v1/inquiries/contact/${request.listingId.asPathSegment()}` with the message body (drop `listing_id` from the JSON body if the server derives it from the path).
   - `replyToInquiry(inquiryId, message)` → `POST /api/v1/inquiries/${inquiryId.asPathSegment()}/respond`.
   - `scheduleViewing(request)` → `POST /api/v1/inquiries/viewing/${request.listingId.asPathSegment()}`.
2. Decide whether reality-server needs a viewings LIST/DELETE surface at all. If yes, add
   `GET /api/v1/inquiries/viewings` and `DELETE /api/v1/inquiries/viewings/{id}` (or a dedicated
   `viewings` router nested from `main.rs`) with owner-scoped queries and update `getViewings` /
   `cancelViewing` accordingly. If no, delete both methods from `InquiryRepository` and remove the
   Android/iOS surfaces that depend on them (the "My Viewings" tab must not silently 404 either).
3. Regenerate any KMP models that shift (e.g. if `CreateInquiryRequest.listingId` moves out of the
   body): re-run the openapi-generator step per `mobile-native/CLAUDE.md § API Client Generation`.
4. Extend the MockEngine tests in
   `mobile-native/shared/src/commonTest/kotlin/three/two/bit/ppt/reality/inquiry/` so every method
   asserts `request.url.encodedPath` against the finalized real path (currently only body shape is
   checked, so this contract drift went silent).
5. Add `backend/servers/reality-server/tests/suites/inquiries_router_contract_tests.rs` with one
   test per KMP call that hits the real router and asserts the status is NOT 404 — fails on `main`
   today for all five paths.
6. Update the affected `docs/screens/reality/*.md` entries (`InquiryDetail`, `ScheduleViewing`,
   any "Send Inquiry" screen) per the screen-map protocol in `mobile-native/CLAUDE.md`.
7. Run `./gradlew :shared:allTests` and `cargo test -p reality-server inquiries` and confirm both
   suites are green.

## Alternatives considered
- **Add legacy POST `/inquiries` and `/{id}/replies` aliases to reality-server** — rejected because the current server routes are the canonical shape (matching `docs/api/typespec/reality-server.tsp`), aliasing on the server side doubles the surface area without a real caller and would drift again the next KMP change; the KMP client is the wrong shape and must adjust.
- **Bypass the KMP shared code and reimplement inquiries per-platform in `androidMain`/`iosMain`** — rejected because the whole point of the `:shared` module is the reality-server contract; per-platform copies would immediately drift and would triple the maintenance surface (Android + iOS + shared) for zero correctness gain.

## Root-cause trace
1. Symptom: sending an inquiry, replying to one, or any viewings action from the KMP client returns HTTP 404 against the real reality-server (`InquiryRepository.kt` failure branch on the `!isSuccess()` path).
2. ← Immediate cause at `InquiryRepository.kt:134,153,176,197,216` — the client hits paths that are not mounted on the reality-server axum router (`/api/v1/inquiries` POST, `/api/v1/inquiries/{id}/replies`, `/api/v1/viewings*`).
3. ← Upstream cause at `backend/servers/reality-server/src/routes/inquiries.rs:181-190` and `backend/servers/reality-server/src/main.rs:475-519` — the server exposes `POST /contact/{listing_id}`, `POST /viewing/{listing_id}`, `POST /{id}/respond` under `/api/v1/inquiries` and has no `/api/v1/viewings` nest; the KMP client was written to a different, unfinalized contract.
4. Origin: the KMP inquiries feature landed against an earlier draft of the reality-server routes; the routes were renamed/reshaped without updating the KMP client, and the MockEngine tests didn't catch the drift because they never asserted path-level equality against the router.

## Test plan
- [ ] Extend `mobile-native/shared/src/commonTest/kotlin/three/two/bit/ppt/reality/inquiry/InquiryRepositoryTest.kt` (or the equivalent existing test file) so each of the five methods asserts `request.url.encodedPath` against the finalized real path.
- [ ] Add `backend/servers/reality-server/tests/suites/inquiries_router_contract_tests.rs` with one test per KMP method: authed request → status is NOT 404. Fails on `main` for all five paths, passes after the KMP + router changes land together.
- [ ] `./gradlew :shared:allTests` — must be green.
- [ ] `cargo test -p reality-server inquiries` — must be green.

## Out of scope
- Rewriting the inquiry/viewing UI on Android or iOS beyond what the URL/model changes force.
- Reworking `HttpClientProvider` timeout/retry policy (tracked as `code-review-mobile-native-kmp-httpclient-no-timeout`).
- Migrating inquiries persistence off reality-server.

## After-merge
- Move this file to `plans/_archive/code-review-mobile-native-kmp-inquiry-viewing-endpoints-wrong-path.md`
- Mark the matching `backlog.json` row as `status: "done"`
