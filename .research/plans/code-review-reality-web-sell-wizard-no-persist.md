# code-review-reality-web-sell-wizard-no-persist

**Vector:** bug
**Score:** 3
**Source:** code-review-reality-web-tier1d 2026-10-02 (dispatcher demand-driven refill; findings file `.research/signals/2026-10-02-reality-web-tier1d.json`)
**Confidence:** high

## Hypothesis
The public `/{locale}/sell` wizard in reality-web collects a full multi-step listing (address, price, area, rooms, description, contact, GDPR) but the Publish button only flips a local `submitted` flag and renders a success screen — it never POSTs the data to any API. A static trace of `sell/page.tsx` confirms zero network calls across the whole 848-line file. Every submission is silently discarded; no listing is created and no lead is captured. Fix: wire the Publish handler to the existing `createListing(data: ListingDraft)` export from `@/lib/realtor-api`, mirror the already-correct await + try/catch + error-state pattern from `account/listings/[id]/edit/page.tsx:195-217`, and only flip `submitted` on success.

## Evidence
- `frontend/apps/reality-web/src/app/[locale]/sell/page.tsx:723-746` — Publish onClick runs `const stepErrors = validateStep(step, form, …); setErrors(stepErrors); if (Object.keys(stepErrors).length === 0) setSubmitted(true);` (no mutation, no fetch, no await).
- `frontend/apps/reality-web/src/app/[locale]/sell/page.tsx:148-151` — the `(submitted)` branch early-returns a static "submission received" screen rendering `t('submittedTitle')` / `t('submittedBody')`.
- `grep -cE 'fetch\(|axios|mutate|useQuery|RealtorApi|@ppt/reality-api-client' frontend/apps/reality-web/src/app/[locale]/sell/page.tsx` → `0`.
- `frontend/apps/reality-web/src/lib/realtor-api.ts:117` — `export function createListing(data: ListingDraft): Promise<ListingResponse>` is already defined and ready to call.
- `frontend/apps/reality-web/src/app/[locale]/account/listings/[id]/edit/page.tsx:195-217` — reference pattern for the Save/error-state wiring to mirror.

## Files
- `frontend/apps/reality-web/src/app/[locale]/sell/page.tsx:723`
- `frontend/apps/reality-web/src/app/[locale]/account/listings/[id]/edit/page.tsx`
- `frontend/apps/reality-web/src/lib/realtor-api.ts`

## Dependencies
<none — reality-web frontend only; no cross-stack coupling.>

## Required capabilities
- [x] C1 — Systematic debugging (bug vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Navigate to `/{locale}/sell` in reality-web (any locale; the public sell wizard, no auth required).
2. Fill all required fields across every wizard step (address, property/transaction type, price, area, rooms, description, contact, GDPR acceptance).
3. On the final step, click Publish.
4. Expected: a `POST` to the create-listing endpoint fires; on success, a confirmation screen names the listing id or redirects to it. On failure, an error state surfaces the server message with a retry affordance.
5. Actual: no network request fires (verify via browser devtools Network tab or `grep -cE 'fetch|RealtorApi' …` returning 0); the component silently renders the success screen while nothing was persisted.

## Suggested approach
1. In `frontend/apps/reality-web/src/app/[locale]/sell/page.tsx`, import `createListing`, `ListingDraft`, and `RealtorApiError` from `@/lib/realtor-api` (mirror the edit page's import at line 15).
2. Add `const [publishing, setPublishing] = useState(false)` and `const [publishError, setPublishError] = useState<string | null>(null)` alongside the existing `submitted` state.
3. Replace the final-step Publish handler at line 723 (and the duplicate at line 730) with an `async` handler that: validates (keep existing `validateStep` call), maps `form` → a `ListingDraft` payload (mirror the edit page's field mapping at line 195-209, including number coercion for `price`/`area`/`rooms`), sets `publishing: true`, awaits `createListing(payload)`, flips `submitted: true` on success, and in `catch` sets `publishError` to `err instanceof RealtorApiError ? err.message : t('publishFailed')`, with `setPublishing(false)` in `finally`.
4. In the Publish button, disable while `publishing === true` and show a spinner/loading label.
5. Render a `publishError` banner (inline, above the Publish button) when non-null — do NOT early-return the submitted screen on error.
6. Add i18n keys `pages.sell.publishError`, `pages.sell.publishing` to all 6 reality-web locale JSONs (en, sk, cs, de, pl, hu) — mirror the account/listings/[id]/edit page's error-copy pattern.
7. In the `(submitted)` success-screen block (line 148-151), reference the listing id or route when the backend returns one.

## Alternatives considered
- **Introduce a new TanStack Query `useMutation` wrapper around `createListing`** — rejected because the sibling edit page uses the lib function directly with a bare `try/catch`, so adopting the same pattern keeps the diff local, consistent with existing reality-web convention, and avoids widening scope into shared mutation plumbing.
- **Keep the local `submitted` success screen without wiring Publish, and gate the whole page behind a "coming soon" feature flag** — rejected because the public URL is already live in production (confirmed by direct URL fetch) and silently appearing to accept submissions is worse than removing the entry point; the correct fix is to persist, not to disable.

## Root-cause trace
1. Symptom: a seller fills the sell wizard, clicks Publish, sees a success confirmation — but no listing exists and no lead is captured.
2. ← immediate cause at `frontend/apps/reality-web/src/app/[locale]/sell/page.tsx:730`: Publish's onClick runs `validateStep(...); setErrors(...); if (Object.keys(stepErrors).length === 0) setSubmitted(true);` — the Publish branch skips every network call.
3. ← upstream cause at `frontend/apps/reality-web/src/app/[locale]/sell/page.tsx:16`: the page imports `INITIAL_FORM_DATA`, `PROPERTY_TYPES`, `SELL_STEPS` from `./_mock` and never imports from `@/lib/realtor-api`, so there is nothing in scope to call — the page was shipped as a UI shell with the "wire the mutation later" step unfilled.
4. Origin: the sell page was introduced as a static shell before `createListing` landed in `realtor-api.ts:117`; nobody closed the loop. The persisted divergence is visible in the parallel edit page (`account/listings/[id]/edit/page.tsx`) having an await+try/catch pattern that the sell page does not.

## Test plan
- [ ] New Vitest file `frontend/apps/reality-web/src/app/[locale]/sell/page.publish.test.tsx` — mount the page under its providers, fill every required field, click Publish, assert `createListing` is called with the mapped payload and the success screen only renders when the mock resolves.
- [ ] Error-path case in the same test file — `createListing` rejects with `new RealtorApiError('Server down')`; assert the component stays on the Publish step, surfaces the error message, re-enables the Publish button, and does NOT flip to the success screen.
- [ ] Run locally: `cd frontend && pnpm --filter @ppt/reality-web test -- --run src/app/\[locale\]/sell/page.publish.test.tsx` (plus `pnpm --filter @ppt/reality-web typecheck` and `pnpm exec biome check` on the touched files).

## Out of scope
- Any backend/reality-server endpoint change (reality-server's create-listing endpoint already exists; this plan only wires the frontend to it).
- Introducing a shared mutation-wrapper abstraction across reality-web pages.
- Full page-level i18n overhaul (only the two new publish-error / publishing-label keys are added, mirroring the finding's minimum scope).
- The adjacent `/profile` and `/report` findings — tracked as separate backlog rows `code-review-reality-web-profile-mock-data` and `code-review-reality-web-report-page-no-submit`.

## After-merge
- Move this file to `plans/_archive/code-review-reality-web-sell-wizard-no-persist.md`
- Mark the matching `backlog.json` row (`code-review-reality-web-sell-wizard-no-persist`) as `status: "done"`
