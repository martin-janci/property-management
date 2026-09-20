# code-review-ppt-web-core-ocr-fetch-no-auth

**Vector:** bug
**Score:** 3
**Source:** dispatcher Tier-1d rotating-expert-review 2026-09-18 (ppt-web-core meters/OCR hooks) — signal `code-review-ppt-web-core-ocr-fetch-no-auth`
**Confidence:** medium

## Hypothesis
`useOcrMeterReading.ts` calls the app's own api-server (`POST /api/v1/ai/ocr/meter-reading` and `POST /api/v1/ai/ocr/correction`) through a bare `fetch()` with no `Authorization` header and no `X-Tenant-ID` header — while ppt-web authenticates only via a localStorage Bearer JWT injected by the axios interceptor / `setTokenProvider` (`lib/api.ts`, `AuthContext.tsx`). There is no cookie session and no global `fetch` monkey-patch, so both OCR calls go out **unauthenticated** and 401, breaking Epic-128 (OCR meter-reading preview + correction-feedback training loop) end-to-end against a real backend. The fix mirrors the pattern already used by every other raw-fetch caller in this app (see `notification-analytics`, `accounting/paymentMatching`): route through `getApiClient()` from `lib/api.ts` (gains Bearer injection, 401 refresh-and-replay, `X-Tenant-ID`, and the `ErrorResponse → ApiError` transform) — or, if the multipart `FormData` upload must stay on raw fetch, attach `Authorization` + `X-Tenant-ID` manually the way `paymentMatching.uploadStatement` does.

## Evidence
- `frontend/apps/ppt-web/src/features/meters/hooks/useOcrMeterReading.ts:26-51` — `ocrFetch()` `const response = await fetch(url, options)`; the image call at `:44-49` passes only a `FormData` body with no `headers` at all.
- `frontend/apps/ppt-web/src/features/meters/hooks/useOcrMeterReading.ts:54-72` — the correction call sets only `Content-Type: application/json`; no `Authorization`, no `X-Tenant-ID`.
- `frontend/apps/ppt-web/src/lib/api.ts:217-229` and `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:361-390` — the axios instance is the SOLE Bearer-injection path; no global `fetch` interceptor exists (grep of `frontend/apps/ppt-web/src` for `window.fetch` / `globalThis.fetch` returns 0 hits).
- Sibling correct callers: `frontend/apps/ppt-web/src/features/notification-analytics/hooks/useNotificationAnalytics.ts:35-41` (`headers.set('Authorization', 'Bearer ' + token)`); `frontend/apps/ppt-web/src/features/accounting/api/paymentMatching.ts:64-73` (`headers.Authorization = 'Bearer ' + token; headers['X-Tenant-ID'] = org`).
- Same-file convention hint: `frontend/apps/ppt-web/src/features/sentiment/hooks/useSentiment.ts:17-21` and `.../predictive-maintenance/hooks/usePredictiveMaintenance.ts:8-11` carry explicit doc comments stating that routing through `getApiClient()` is what applies Bearer injection + 401 refresh + tenant scoping — `useOcrMeterReading` is the lone outlier.

## Files
- `frontend/apps/ppt-web/src/features/meters/hooks/useOcrMeterReading.ts`
- `frontend/apps/ppt-web/src/lib/api.ts`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Log in to ppt-web as any authenticated user in a fresh browser tab. Confirm `localStorage.getItem('token')` returns a valid JWT.
2. Navigate to Meters → OCR Preview. Upload a meter photo — this triggers `useOcrMeterReading.submit(image)` which POSTs to `/api/v1/ai/ocr/meter-reading`.
3. In the DevTools Network tab inspect the outbound request: **actual** — no `Authorization` header, no `X-Tenant-ID` header, response 401. **Expected** — `Authorization + Bearer <jwt>` and `X-Tenant-ID: <active-org-id>` on every request; response 200/422 (a real OCR outcome).
4. Equivalent hook test (no live backend): mock `fetch` (or `axios` after fix), call the hook via `renderHook`, and assert the recorded request headers contain both `Authorization` and `X-Tenant-ID`. Fails on main today.

## Suggested approach
1. Replace `ocrFetch()` inside `useOcrMeterReading.ts` with a call through `getApiClient()` — the axios instance already handles `Content-Type` for JSON and lets you pass a `FormData` body for multipart (axios auto-sets the boundary). This is the smallest change that gains Bearer + 401-replay + tenant + `ErrorResponse` transform for free.
2. If the raw-multipart path must stay (some CI proxies mangle axios' auto-boundary), instead: read the current Bearer via the same accessor `AuthContext` exposes (or the `setTokenProvider` value inside `lib/api.ts`) and set `Authorization + Bearer <t>` + `X-Tenant-ID: <org>` on each request — mirroring `features/accounting/api/paymentMatching.ts:64-73` exactly. Do NOT re-implement Bearer refresh; if the token is stale, surface the 401 upstream (correction path already handles error state) and let the next axios call refresh.
3. If neither approach is acceptable to the reviewer, add a small helper `frontend/apps/ppt-web/src/lib/authedFetch.ts` that wraps `fetch()` with the same header injection axios does — but the plan strongly prefers path (1) because it deletes drift, not adds a second convention.
4. Update the JSDoc atop `useOcrMeterReading.ts` to state the auth contract (mirrors the `useSentiment` / `usePredictiveMaintenance` comments) so the pattern is enforced in review.
5. Add a Vitest hook test at `frontend/apps/ppt-web/src/features/meters/hooks/useOcrMeterReading.test.tsx` that mocks the axios `api` module (or `fetch` in the raw-path variant) and asserts each request carries `Authorization + Bearer <mock-token>` and `X-Tenant-ID: <mock-org>`. This is the IG3 failing-on-main test.
6. Run `pnpm --filter @ppt/web typecheck && pnpm --filter @ppt/web test -- --run useOcrMeterReading` in the cloud runner (npm reachable). Confirm both new-test and typecheck pass.

## Alternatives considered
- **Add a global `fetch` monkey-patch that injects Bearer + tenant** — rejected because it would silently affect every third-party `fetch` call in the app (analytics vendors, CDN preloads) and diverges further from the codebase's axios-first convention.
- **Move OCR to a cookie-session dedicated OCR service** — rejected because it's an architectural pivot far beyond a single-file fix; the api-server already enforces Bearer JWT + tenant scoping, and no other feature relies on cookie sessions for its own endpoints.

## Root-cause trace
1. Symptom: OCR meter-reading preview and correction upload silently fail (401) against any real backend — Epic-128 is DoA for authenticated users.
2. ← `useOcrMeterReading.ts:26-51` — `ocrFetch()` uses `fetch()` directly and sets no auth headers.
3. ← `useOcrMeterReading.ts:54-72` — the correction path sets only `Content-Type`, no `Authorization` / `X-Tenant-ID`.
4. ← `lib/api.ts:217-229` — the app's Bearer injection is scoped to the axios instance (`getApiClient()`); there is no global `fetch` interceptor, so raw-fetch callers must attach headers themselves.
5. Origin: the OCR hook likely predates the current axios-based auth convention (or was authored against a mock that returned 200 for everything). No lint rule blocks raw `fetch()` in `frontend/apps/ppt-web/src`, so the convention drift went undetected — see also `dep-update-noise: eslint-plugin-no-fetch` in the DX backlog for the linter follow-up.

## Test plan
- [ ] `frontend/apps/ppt-web/src/features/meters/hooks/useOcrMeterReading.test.tsx` — new; asserts each outbound OCR request carries `Authorization + Bearer <t>` and `X-Tenant-ID: <org>`. Fails on main.
- [ ] `frontend/apps/ppt-web/src/features/meters/hooks/useOcrMeterReading.test.tsx` — add a 401→refresh→replay scenario in the axios-based variant (path 1) that asserts the second request lands with the refreshed token.
- [ ] Regression command: `pnpm --filter @ppt/web test -- --run useOcrMeterReading && pnpm --filter @ppt/web typecheck`

## Out of scope
- Adding an ESLint rule that forbids raw `fetch()` in `frontend/apps/ppt-web/src` — worth doing but a separate DX plan.
- Reviewing every other raw-fetch site in the app for the same defect — the two known-good callers (`notification-analytics`, `paymentMatching`) already attach headers; a full sweep + lint rule is a follow-up.
- Backend-side changes to `/api/v1/ai/ocr/*` handlers — the endpoints already enforce Bearer + tenant; no server-side changes are in this plan.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-ocr-fetch-no-auth.md`
- Mark `backlog.json` row `code-review-ppt-web-core-ocr-fetch-no-auth` as `status: "done"`
