# code-review-mobile-native-kmp-path-inject-repo-sweep

**Vector:** security
**Score:** 3
**Source:** Phase 1.5 code-review 2026-07-11 (mobile-native-kmp segment) + issue #2195 + PR #2213 (draft)
**Confidence:** high

## Hypothesis
PR #2180 hardened `ListingRepository` (and previously `FavoritesRepository`) against path-injection by routing every raw id through a private `pathSegment()` helper wrapping `encodeURLPathPart()`. Four sibling shared-KMP repositories still splice untrusted ids straight into request URLs (nine call sites total). Because the shared module powers both Android Compose and iOS SwiftUI, one crafted deep-link value can smuggle `/`, `?`, `#`, `..`, or whitespace past the intended endpoint on both platforms. The smallest safe fix is to extract the two-line encoder once into `commonMain/api/UrlEncoding.kt`, delete the three copies, and rewrite every remaining `$id` interpolation to `${asPathSegment(id)}`.

## Evidence
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/agency/AgencyRepository.kt:43,61,79` — `/api/v1/agencies/$agencyId`, `.../by-slug/$slug`, `.../$agencyId/members` (documented as "used by deep-link handlers")
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/realtor/RealtorRepository.kt:49` — `/api/v1/realtors/$userId/profile`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/inquiry/InquiryRepository.kt:110,149,212` — `.../inquiries/$inquiryId`, `.../inquiries/$inquiryId/replies`, `.../viewings/$viewingId`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/notifications/NotificationRepository.kt:80,116` — `.../notifications/$notificationId/read`, `.../notifications/$notificationId`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/listing/ListingRepository.kt` — `private fun pathSegment(v)` (fixed shape from PR #2180); `favorites/FavoritesRepository.kt` and `api/ApiClient.kt` each carry a byte-identical duplicate

## Files
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/agency/AgencyRepository.kt`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/realtor/RealtorRepository.kt`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/inquiry/InquiryRepository.kt`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/notifications/NotificationRepository.kt`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/listing/ListingRepository.kt`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/favorites/FavoritesRepository.kt`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/api/ApiClient.kt`

## Dependencies
- gh-issue-2195

## Required capabilities
- [x] C1 — Systematic debugging (security bug class)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Build a MockEngine that captures the outgoing request URL.
2. Call `AgencyRepository.getAgencyBySlug("foo/../admin?x=1")` (mirroring an attacker-crafted deep-link).
3. Assert the captured URL path is `/api/v1/agencies/by-slug/foo%2F..%2Fadmin%3Fx%3D1` — i.e. a single percent-encoded segment. **Before the fix** the captured URL is `/api/v1/agencies/by-slug/foo/../admin?x=1`, escaping the resource path (regression test fails on `dev`).
4. Repeat per remaining call site: `Agency.getAgencyById` / `getAgencyMembers`, `Realtor.getRealtorProfile`, `Inquiry.getInquiry` / `replyToInquiry` / `getViewing`, `Notification.markRead` / `deleteNotification`.

## Suggested approach
1. Add `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/api/UrlEncoding.kt` with `internal fun String.asPathSegment(): String = encodeURLPathPart()`. One symbol, one line.
2. Delete the three private `pathSegment()` copies in `ListingRepository.kt`, `FavoritesRepository.kt`, and `ApiClient.kt`.
3. Rewrite `ListingRepository.kt` and `FavoritesRepository.kt` to call `.asPathSegment()` (keeps the existing regression tests green).
4. Wrap every `$id`/`$slug` bullet in *Evidence* with `${id.asPathSegment()}` in `AgencyRepository.kt`, `RealtorRepository.kt`, `InquiryRepository.kt`, `NotificationRepository.kt`.
5. Add one regression test per repository under `mobile-native/shared/src/commonTest/kotlin/three/two/bit/ppt/reality/<pkg>/` following the `Captured`/`repoCapturing` pattern from the PR #2180 test.
6. Run `./gradlew :shared:jvmTest` locally; ensure all new + existing tests pass; land on `dev`.
7. If PR #2213 has already merged the shared helper by the time this plan is claimed, skip steps 1–3 (helper already exists) and jump straight to steps 4–6.

## Alternatives considered
- **Regex-based URL sanitiser applied globally at the Ktor client interceptor layer** — rejected because it hides the encoding decision from the call site (silent behaviour change) and would encode legitimately-composed multi-segment paths.
- **Wait for PR #2213 to land unmodified** — rejected because #2213 has been open in draft for two days without a review round; a spec-grade plan lets the dispatcher fall back to a fresh implementation if #2213 gets abandoned, and clarifies scope (issue #2195's follow-up explicitly lists the extracted shared helper as a proposed improvement, not a landed one).

## Root-cause trace
1. Symptom: `AgencyRepository.getAgencyBySlug("foo/../admin?x=1")` produces `GET /api/v1/agencies/by-slug/foo/../admin?x=1` — escapes the intended endpoint.
2. ← `AgencyRepository.kt:61` interpolates `$slug` raw into the URL string (`client.get("$baseUrl/api/v1/agencies/by-slug/$slug")`).
3. ← Ktor's URL builder trusts the string as-is (no auto-encoding for template segments); the same class of bug fixed for `ListingRepository` in PR #2180 was never applied here.
4. Origin: repositories authored in the initial mobile-native scaffold (pre-PR #2180) — the encoding contract lived only in a per-file private helper, so new repos added later ended up without it and PR #2180 patched only the file it was routed at.

## Test plan
- [ ] `mobile-native/shared/src/commonTest/kotlin/three/two/bit/ppt/reality/agency/AgencyRepositoryTest.kt` (new) — one test per Agency call site asserting a `foo/../admin` id percent-encodes.
- [ ] `mobile-native/shared/src/commonTest/kotlin/three/two/bit/ppt/reality/realtor/RealtorRepositoryTest.kt` (new) — same shape for `getRealtorProfile`.
- [ ] `mobile-native/shared/src/commonTest/kotlin/three/two/bit/ppt/reality/inquiry/InquiryRepositoryTest.kt` (new) — one test per Inquiry/Viewing call site.
- [ ] `mobile-native/shared/src/commonTest/kotlin/three/two/bit/ppt/reality/notifications/NotificationRepositoryTest.kt` (new) — one test per Notification call site.
- [ ] Existing `ListingRepositoryTest`/`FavoritesRepositoryTest` continue to pass under the shared helper.
- [ ] Verify command: `cd mobile-native && ./gradlew :shared:jvmTest --tests '*Repository*'`

## Out of scope
- SwiftUI-only or Compose-only URL splicing (this plan covers only shared/commonMain — platform-specific splicing lives in per-platform code and requires a separate audit).
- Auth token handling in `SsoService` (covered by `code-review-mobile-native-kmp-repo-test-coverage-gap`, which is a separate backlog item).
- WebSocket URL construction (`ApiConfig.wsUrl`) — covered by `code-review-mobile-native-kmp-wsurl-broad-replace`.

## After-merge
- Close backlog item `code-review-mobile-native-kmp-path-inject-repo-sweep` (`status: done`, add merged PR # to `sources`).
- If PR #2213 landed independently, mark this plan file archived by moving to `.research/plans/_archive/`.
- Note in the next brief: PR #2180 was the first pass, this plan the sweep — future new KMP repos should call `asPathSegment()` by default.
