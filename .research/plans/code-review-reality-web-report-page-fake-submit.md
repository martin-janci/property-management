# code-review-reality-web-report-page-fake-submit

**Vector:** bug
**Score:** 3
**Source:** Tier1d review 2026-08-12 (reality-web)
**Confidence:** medium

## Hypothesis
`reality-web`'s `/[locale]/report` "Report a listing" abuse-report form (`app/[locale]/report/page.tsx`) validates its inputs and then flips `setSubmitted(true)` without ever sending the report to a backend — `grep fetch|mutate|apiClient|.post` over the file returns 0 hits. The user sees a success confirmation while every abuse/problem report is silently dropped. Beyond user data loss, a public EU real-estate portal that captures nothing on its abuse-report surface has direct DSA notice-and-action compliance exposure. The smallest correct change is to POST the report to the reality-server (via `@ppt/reality-api-client`), thread loading/error state, and only flip `setSubmitted(true)` on success.

## Evidence
- `frontend/apps/reality-web/src/app/[locale]/report/page.tsx:37-41` — `handleSubmit` runs `e.preventDefault()`, guard-returns on invalid input, then calls `setSubmitted(true)` with no network call
- `frontend/apps/reality-web/src/app/[locale]/report/page.tsx:12` — `REPORT_PROBLEMS` categories imported from local `./_mock` (placeholder)
- `frontend/apps/reality-web/src/app/[locale]/report/page.tsx:3-4` — file header comment: "Report a listing — problem report form"
- Same fake-submit pattern class as the sibling backlog row `code-review-reality-web-sell-page-fake-submit` (on `[locale]/sell/page.tsx`), and the previously-fixed `code-review-reality-web-realtor-mgmt-untranslated` region — both were seller-lead / agent-contact flows on the same portal

## Files
- `frontend/apps/reality-web/src/app/[locale]/report/page.tsx:37`
- `frontend/apps/reality-web/src/app/[locale]/report/page.tsx:12`
- `frontend/apps/reality-web/src/app/[locale]/report/_mock.ts`
- `docs/api/typespec`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):**
- No C4 or C5 → `cloud-ok`

Mode: cloud-ok

## Repro steps
1. Serve `reality-web` (`pnpm dev:reality`). Visit `/en/report?ref=<any-listing-slug>`.
2. Pick a problem category, add description, tick GDPR, click Submit.
3. Observe: success screen renders; open the browser network tab — **no request is sent**. Query `reality-server` DB for any `listing_reports`-shaped row: empty. Expected: a POST fires to `/api/v1/listing-reports` (or the endpoint the API adds), a row is persisted, and the success screen renders only on 2xx.

## Suggested approach
1. Confirm the reality-server surface: grep `backend/servers/reality-server/src/routes/` for an existing listing-report endpoint (`listing_reports.rs`, `abuse.rs`, `moderation.rs`, `dsa.rs`); if it exists, use its OpenAPI-generated client method from `@ppt/reality-api-client`. If it does not exist, add the endpoint via `docs/api/typespec` first, regenerate the client, then wire the frontend to it — the endpoint contract is out-of-scope prose but the *implementation* is in-scope for this plan (do not ship a "fake-submit fixed by adding a fetch to a 404" outcome).
2. In `page.tsx`, replace the `handleSubmit` body with a TanStack Query `useMutation` (already imported elsewhere in the app) that calls the client method with `{ problem, listingRef, description, attachments, gdprAcceptedAt: new Date() }`.
3. Thread `isPending` / `error` state: disable the Submit button while pending; render a localized error banner (via `t('errors.reportFailed')`) on failure; only call `setSubmitted(true)` in the mutation's `onSuccess`.
4. Attachments: if the API accepts multipart, forward the `File[]` via `FormData`; if it accepts only URLs, out-of-scope for this pass — leave the attachment picker but note in Out of scope.
5. Move the `REPORT_PROBLEMS` list from `./_mock.ts` into `page.tsx` (or a shared const) so the file stops advertising a placeholder; keep the same shape.
6. Add a Playwright + msw regression covering: (a) fake-submit is impossible without a network call, (b) 5xx surfaces the error banner and does NOT set `submitted`, (c) 2xx flips `submitted`.
7. `pnpm --filter reality-web test` — expect green.

## Alternatives considered
- **Disable the form and add a placeholder "coming soon" banner** — rejected because it makes a live route regress in user value and does not resolve the DSA exposure (the surface still exists to a crawler, and disabling it visibly is a worse product outcome than making it work).
- **Log the report to an in-browser sink (localStorage / Sentry breadcrumb) as a stop-gap while the API is missing** — rejected because it does not persist the report, does not satisfy any compliance obligation, and buries the real defect behind a fake fix.

## Root-cause trace
1. Symptom: submitting the abuse-report form shows success; no report is persisted anywhere.
2. ← `page.tsx:37-41` `handleSubmit` calls only `setSubmitted(true)`; no fetch.
3. ← Categories come from `./_mock`, suggesting the whole route was scaffolded before the API surface existed.
4. ← There is no `getReportProblems` / `submitListingReport` fetcher in `@ppt/reality-api-client`; the wiring was never added.
5. Origin: the route was landed as a design/UX slice ahead of the backend contract; DSA follow-through never closed the loop.

## Test plan
- [ ] `pnpm --filter reality-web exec vitest run app/[locale]/report/page.test.tsx` — new tests: (T1) submit without network call is impossible (msw intercepts a POST — assert it was called); (T2) 5xx from server surfaces an error banner and `submitted === false`; (T3) 2xx flips `submitted` and clears form state.

## Out of scope
- Attachments upload transport if the API surface only accepts URLs — leave the picker inert and file a follow-up.
- Server-side DSA notice-and-action workflow (moderator queue, statement-of-reasons, appeal) — this plan lands the submission edge only.
- Sibling `code-review-reality-web-sell-page-fake-submit` fix — separate backlog row, promoted separately.

## After-merge
- Close backlog row `code-review-reality-web-report-page-fake-submit`.
- Grep the reality-web app for other `setSubmitted(true)` / `setMessageSent(true)` / `setSent(true)` handlers not preceded by an `await`; file follow-up backlog rows for each hit (the sibling `code-review-reality-web-sell-page-fake-submit` and `code-review-reality-web-realtor-contact-fake-submit` already exist).
- Update `docs/screens/reality/report-listing.md`'s `apiStatus`/`buildStatus` frontmatter and Agent Log.
