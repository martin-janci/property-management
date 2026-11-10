# code-review-reality-web-sell-wizard-no-persist

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review reality-web 2026-10-02 (dispatcher Tier-1d)
**Confidence:** high

## Hypothesis
The reality-web `/sell` multi-step listing wizard shows a success screen but never POSTs the collected data. On the final "Publish" step the onClick runs `validateStep` and, on empty errors, calls `setSubmitted(true)` — no network call anywhere in the 848-line file. Every seller completing the wizard is silently discarded. The smallest change is to wire the Publish handler to the existing `createListing` client (which already targets `POST /api/v1/listings`) and flip `submitted` only on success, mirroring the already-correct await+try/catch pattern used by the sibling `account/listings/[id]/edit/page.tsx`.

## Evidence
- `frontend/apps/reality-web/src/app/[locale]/sell/page.tsx:723-746` — Publish onClick body is only `const stepErrors = validateStep(...); setErrors(stepErrors); if (Object.keys(stepErrors).length === 0) setSubmitted(true);`. No `fetch` / mutation / API-client call appears anywhere in the file (`grep -c 'fetch\|mutate\|createListing\|RealtorApi\|await ' sell/page.tsx` returns 0).
- `frontend/apps/reality-web/src/app/[locale]/sell/page.tsx:151` — `if (submitted) return <SubmittedScreen ... />;` renders localized confirmation copy regardless of persistence.
- `frontend/apps/reality-web/src/lib/realtor-api.ts:117-122` — `createListing(data: ListingDraft): Promise<ListingResponse>` already exists and POSTs to `/api/v1/listings`.
- `backend/servers/reality-server/src/routes/portal_listings.rs:24,207` — server already serves `POST /api/v1/listings` → `create_listing`.
- `frontend/apps/reality-web/src/app/[locale]/account/listings/[id]/edit/page.tsx:193-217` — correct template: `await updateListing(...); setSaved(true);` with `catch (err) { setSaveError(err instanceof RealtorApiError ? err.message : 'Uloženie zlyhalo, skúste znova.'); }` and a `finally { setSaving(false); }`.

## Files
- `frontend/apps/reality-web/src/app/[locale]/sell/page.tsx`
- `frontend/apps/reality-web/src/lib/realtor-api.ts`

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
Mode: cloud-ok

## Repro steps
1. Open the reality-web app at `/<locale>/sell`, step through the 5 steps with valid data, accept GDPR+terms, click **Publish**.
2. Observe: success confirmation screen renders (`SubmittedScreen`) and no `POST /api/v1/listings` ever leaves the browser (verified via Network tab or the already-missing `createListing` call-site). On `main`, no listing is created on the server.
3. Expected: a `POST /api/v1/listings` is sent, a listing row is created server-side (`portal_create_listing`), and the UI either shows the success screen on 2xx or a submit-error branch (field-level or form-level) on 4xx/5xx.

## Suggested approach
1. Add a submit state tuple at the top of `SellPage` next to `submitted`: `const [saving, setSaving] = useState(false); const [submitError, setSubmitError] = useState<string | null>(null);`.
2. Extract the Publish handler into an async function `handlePublish` co-located with the component (not inside the JSX): validate first, then build the `ListingDraft` payload from `form` (map wizard fields: `propertyType`, `transactionType`, `price`, `currency`, `area`, `rooms`, `street=form.address`, `city`, `postalCode`, `country`, `description`), set `setSaving(true)`, `await createListing(payload)`, on success `setSubmitted(true)`, on `RealtorApiError` → `setSubmitError(err.message)`, else `setSubmitError(t('submitFailed'))`; `finally { setSaving(false); }`.
3. Wire the Publish `<button>` at `page.tsx:720-746`: `onClick={handlePublish}`, `disabled={!form.termsAccepted || saving}`, label swaps to `t('publishing')` while `saving`.
4. Render a non-blocking error banner above the step buttons when `submitError`; mirror the existing error styling used for per-field errors or the edit page's `saveError` block.
5. Keep `submitted` as the terminal state — only flip to true on a resolved success.
6. Add new i18n keys under `pages.sell.*` in each `messages/*.json`: `publishing`, `submitFailed`, `submitError`. (Hungarian + Polish keys default to Slovak until professionally translated — same pattern as prior PRs.)
7. Add a Vitest regression test (see Test plan) and run `pnpm -F reality-web test` + `pnpm -F reality-web typecheck` + `pnpm -F reality-web check`.

## Alternatives considered
- **Introduce a `useCreateListing` TanStack mutation hook** in `@ppt/reality-api-client` and consume it — rejected because the generated `reality-api-client` package doesn't yet expose a React-Query hook for this endpoint and the direct `createListing` function in `realtor-api.ts` is already the pattern the sibling edit page follows; adding a new generated hook widens the PR into client-generation territory.
- **Server-side form POST (progressive enhancement)** — rejected because the wizard already relies on client-side state across 5 steps, including file upload placeholders; converting to a Next.js server action would require re-architecting all step transitions and is well out of scope for a correctness fix.

## Root-cause trace
1. Symptom: user completes the sell wizard, sees "Published!" confirmation, but no listing appears on the server; no network request leaves the browser.
2. ← `frontend/apps/reality-web/src/app/[locale]/sell/page.tsx:730` — `setSubmitted(true)` is called without any `await createListing(...)` or any other network call.
3. ← The Publish button's inline onClick (`page.tsx:720-746`) was authored at wizard-scaffolding time as a UI-only prototype; the data layer wiring was never added when the server endpoint and `createListing` client function landed (`realtor-api.ts:117`, `portal_listings.rs:207`).
4. Origin: wizard scaffolding commit introducing the Publish button with a mock submit path (`SELL_STEPS` / `INITIAL_FORM_DATA` imported from `./_mock`) — predates the `portal_listings` route; the integration follow-up was never done.

## Test plan
- [ ] New `frontend/apps/reality-web/src/app/[locale]/sell/page.publish.test.tsx` — mock `createListing` and assert (a) clicking Publish with valid form state fires `createListing` once with the mapped payload, (b) a resolved mock flips to the success screen, (c) a rejected mock with `RealtorApiError('nope')` renders the error banner and keeps `submitted` false, (d) the Publish button is disabled while `saving`.
- [ ] Fails on `main` (handler never calls `createListing`), passes after the fix.
- [ ] `pnpm -F reality-web test` locally; `pnpm -F reality-web typecheck`; `pnpm -F reality-web check`.

## Out of scope
- Photo upload wiring — the wizard currently has a photos step with placeholder-only UI; a follow-up should route uploads through the presigned pipeline mirroring the mobile fault-attachment pattern. Not part of this fix.
- Address → geolocation lookup — the wizard collects city/street/postal without geocoding; current `ListingDraft` contract does not require lat/lon, so this fix stays within the existing contract.
- Rewrite of `_mock.ts` into a real options source — the `PROPERTY_TYPES` / `SELL_STEPS` are configuration-grade constants and don't need to be refactored to fix the persistence gap.

## After-merge
- Move this file to `plans/_archive/code-review-reality-web-sell-wizard-no-persist.md`
- Mark the matching `backlog.json` row as `status: "done"`
