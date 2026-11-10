# code-review-mobile-native-kmp-inquiries-response-contract

**Vector:** bug
**Score:** 3
**Source:** Phase 1.5 rotating expert review 2026-08-16 (mobile-native-kmp segment)
**Confidence:** high

## Hypothesis
The KMP `InquiriesResponse` (`inquiry/InquiryModels.kt:50-55`) declares `page_size` as a **required** field, but reality-server returns `limit` on `GET /api/v1/inquiries` and returns neither `page` nor `page_size` on `GET /api/v1/realtors/inquiries`. With `Json { ignoreUnknownKeys = true }` the incoming `limit` is silently dropped, and the missing required `page_size` throws `kotlinx.serialization.MissingFieldException` on every real response — so both flows fail with a `Result.failure(Exception)` in the shared Kotlin, not a parsable payload. Fixing the KMP model to name the field `limit` (and to make paging fields tolerant of the realtors shape) restores portal + inquiries listing without changing the backend contract.

## Evidence
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/inquiry/InquiryModels.kt:50-55` — `data class InquiriesResponse(val inquiries: List<Inquiry>, val total: Int, val page: Int, @SerialName("page_size") val pageSize: Int)`; no default on `pageSize` → serializer flags it required.
- `backend/servers/reality-server/src/routes/inquiries.rs:253-263` — `InquiryListResponse { inquiries, total, page, limit }` (field is `limit`, not `page_size`).
- `backend/servers/reality-server/src/routes/realtors.rs:39-43` — realtor endpoint `InquiriesResponse { inquiries, total }` (no `page`, no `limit`); KMP's `getRealtorInquiries()` reuses the same class, so this call is also broken.
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/inquiry/InquiryRepository.kt:61` — outbound query sends `parameter("page_size", pageSize)` but `InquiryListQuery` (inquiries.rs:241-249) reads `limit`, so custom page sizes silently fall back to the server default.

## Files
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/inquiry/InquiryModels.kt`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/inquiry/InquiryRepository.kt`
- `backend/servers/reality-server/src/routes/inquiries.rs`
- `backend/servers/reality-server/src/routes/realtors.rs`

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
1. On `dev`, hit `GET /api/v1/inquiries?page=1&page_size=5` against a live reality-server instance (or use the KMP `InquiryRepository.getInquiries(page = 1, pageSize = 5)` from Android/iOS): server responds with `{ inquiries, total, page, limit: 20 }`.
2. KMP receives it, `Json { ignoreUnknownKeys = true }` drops `limit`, then `MissingFieldException: Field 'page_size' is required` is thrown inside `response.body<InquiriesResponse>()` at `InquiryRepository.kt:66`. The `try/catch` converts it into `Result.failure(InquiryException("Failed to load inquiries: ..."))`. Expected: parsed response. Actual: exception.
3. Same repro for realtors: `GET /api/v1/realtors/inquiries` returns `{ inquiries, total }`; KMP requires both `page` AND `page_size` → same `MissingFieldException`.

## Suggested approach
1. Rename the KMP `InquiriesResponse` field from `pageSize` (with `@SerialName("page_size")`) to `val limit: Int` so it matches the wire contract from `inquiries.rs:262`. This is the safe direction (align KMP to backend, no server change needed).
2. Make `page` and `limit` tolerant of absent values (`val page: Int? = null`, `val limit: Int? = null`) so the realtor endpoint's `{ inquiries, total }` shape parses without a second DTO. Keep `total: Int` required.
3. Update `InquiryRepository.kt:61` outbound query to send `parameter("limit", pageSize)` instead of `page_size`, so caller-supplied paging is honored by `InquiryListQuery { page, limit }` in `inquiries.rs:241-249`.
4. Fix any consumers of the removed `pageSize` field name (grep `pageSize` under `mobile-native/shared/src/commonMain`); expose it via a computed `val pageSize: Int? get() = limit` for source-compat if any UI code reads it.
5. Do NOT touch the backend response shape — reality-server's `InquiryListResponse.limit` and realtor `InquiriesResponse` are the contract other clients rely on.

## Alternatives considered
- **Change backend to return `page_size` instead of `limit`** — rejected because reality-server is the source of truth for other clients (ppt-web/reality-web), and inquiries.rs:262 `limit` matches the query param name; renaming server-side would ripple through OpenAPI + JS clients.
- **Add a bespoke KMP DTO per endpoint (`InquiriesResponse` vs `RealtorInquiriesResponse`)** — rejected because a single DTO with two optional paging fields is smaller, matches the server's actual polymorphism, and keeps the shared repository code path simple.

## Root-cause trace
1. Symptom: `Result.failure(...)` on every `getInquiries()` / `getRealtorInquiries()` call from Android + iOS — inquiries list surfaces show an error state.
2. ← `response.body<InquiriesResponse>()` throws `MissingFieldException: page_size` at `InquiryRepository.kt:66`.
3. ← `InquiriesResponse.pageSize: Int` (no default) at `InquiryModels.kt:54` — kotlinx.serialization treats it as required.
4. ← Backend never emitted `page_size` — the wire field has always been `limit` (`inquiries.rs:262`), and the realtor variant is `{ inquiries, total }` only (`realtors.rs:39-43`).
5. Origin: the KMP DTO was written speculatively against a hypothetical shape rather than the shipped contract; there is no `getInquiries` regression test in `mobile-native/shared/src/commonTest/**`, so the drift has been latent since the file was first added.

## Test plan
- [ ] `mobile-native/shared/src/commonTest/kotlin/three/two/bit/ppt/reality/inquiry/InquiryRepositoryTest.kt` — new unit test using Ktor `MockEngine` that serves the *real* server shapes (`{inquiries, total, page, limit}` and `{inquiries, total}`) and asserts both `getInquiries()` and `getRealtorInquiries()` return `Result.success`.
- [ ] Regression assertion: outbound request to `/api/v1/inquiries` includes `limit=<N>` (not `page_size=<N>`) when `pageSize` is passed to the repository.
- [ ] Run: `cd mobile-native && ./gradlew :shared:allTests` (or the equivalent commonTest target).

## Out of scope
- No backend response-shape changes (reality-server contract is authoritative).
- No UI/i18n changes in the Android / iOS apps beyond the DTO field rename ripple.
- No OpenAPI / typespec churn — the wire contract is unchanged.

## After-merge
- Move this file to `plans/_archive/code-review-mobile-native-kmp-inquiries-response-contract.md`
- Mark the matching `backlog.json` row as `status: "done"`
