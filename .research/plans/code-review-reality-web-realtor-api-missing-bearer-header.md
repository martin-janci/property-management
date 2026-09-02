# code-review-reality-web-realtor-api-missing-bearer-header

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review Phase 1.5 (reality-web static review 2026-09-02)
**Confidence:** high

## Hypothesis
`reality-web/src/lib/realtor-api.ts` wraps every realtor CRUD write with a shared `request()` helper that sends only `credentials: 'include'` and never merges the bearer-token `Authorization` header returned by `getAuthHeader()` from `lib/auth-token.ts`. Under the shipping form-login flow (`/[locale]/auth/login` → localStorage bearer), reality-server sees no credentials on the write path and rejects every call with 401 — breaking the whole realtor onboarding + listing publish surface. The sibling `lib/auth-api.ts` already demonstrates the correct pattern (spread `...getAuthHeader()` into the merged headers); the smallest safe fix is to mirror that inside `request()`.

## Evidence
- `frontend/apps/reality-web/src/lib/realtor-api.ts:22-33` — `request()` composes headers as `{ 'Content-Type': 'application/json', ...(callerHeaders ?? {}) }` with no `getAuthHeader()` call anywhere in the module.
- `frontend/apps/reality-web/src/lib/auth-token.ts:84` — `export function getAuthHeader(): Record<string, string>` returns `{ Authorization: 'Bearer …' }` when the localStorage token is present.
- `frontend/apps/reality-web/src/lib/auth-api.ts:43` — the correct pattern already in use: `headers: { 'Content-Type': 'application/json', ...getAuthHeader() }`.
- `frontend/apps/reality-web/src/lib/auth-context.tsx:180-189` — comment records that the OAuth/SSO consent UI has not shipped; form-login is the only wired login path, so `credentials: 'include'` cannot compensate for the missing bearer header.
- Every mutation helper in `realtor-api.ts` (`createListing`, `updateListing`, `getMyListing`, `getMyRealtorProfile`, `updateMyRealtorProfile`, `getMyRealtorAnalytics`) is exercised by pages under `frontend/apps/reality-web/src/app/[locale]/realtor/**` and `.../account/listings/[id]/edit` — the whole realtor surface goes through this one helper.

## Files
- `frontend/apps/reality-web/src/lib/realtor-api.ts`

## Dependencies
<!-- none — self-contained one-file fix + one new test file -->

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps
1. In `frontend/`, run `pnpm --filter @ppt/reality-web test -- realtor-api` (or after this plan lands, `pnpm --filter @ppt/reality-web test -- realtor-api.test`).
2. Author the IG3 failing-on-main test in `frontend/apps/reality-web/src/lib/realtor-api.test.ts`: stub `getAuthHeader` (via `vi.mock('./auth-token', () => ({ getAuthHeader: () => ({ Authorization: 'Bearer test-token' }) }))`), stub `global.fetch` with a `vi.fn` returning `{ ok: true, status: 200, json: async () => ({}) }`, call `updateListing('id', { …minimal payload })`, and assert `fetch.mock.calls[0][1].headers.Authorization === 'Bearer test-token'`.
3. Expected on `main`: assertion fails — headers object contains only `Content-Type`, no `Authorization`.
4. After the fix: assertion passes.

## Suggested approach
1. Edit `frontend/apps/reality-web/src/lib/realtor-api.ts` at line ~27-33: import `getAuthHeader` from `./auth-token` and change the fetch call's `headers` initializer to `{ 'Content-Type': 'application/json', ...getAuthHeader(), ...(callerHeaders ?? {}) }` — spreading `callerHeaders` last so a caller override still wins.
2. Guard for the SSR path the way `auth-token.ts` already does — `getAuthHeader()` returns `{}` when `window` is undefined, so no extra guard is needed here.
3. Add `frontend/apps/reality-web/src/lib/realtor-api.test.ts` with at minimum three fetch-stub cases: (a) request sends the `Authorization` header with the stored bearer value when a token is present, (b) request omits `Authorization` when `getAuthHeader()` returns `{}` (unauthenticated + SSR path), (c) a caller-supplied `Authorization` header overrides `getAuthHeader()`.
4. Re-run `pnpm --filter @ppt/reality-web test -- realtor-api` and `pnpm --filter @ppt/reality-web typecheck`; then `pnpm biome check frontend/apps/reality-web/src/lib/realtor-api.ts frontend/apps/reality-web/src/lib/realtor-api.test.ts`.

## Alternatives considered
- **Move the helper wholesale to a shared axios-with-interceptor client** — rejected because reality-web has no such client today (`auth-api.ts` uses plain fetch too); the churn would sprawl this bug fix into an architectural refactor, and the sibling code-review-reality-web-realtor-api-no-tests item can pick up test-coverage separately.
- **Regenerate the reality-api SDK to expose realtor mutations and delete `realtor-api.ts` entirely** — rejected because the SDK doesn't yet expose them (docstring line 1-7 records this) and a SDK regeneration would drag in the typespec + generated-client pipeline; leave that to a follow-up refactor plan and unblock realtor onboarding now.

## Root-cause trace
1. Symptom: authenticated realtor hits `/realtor/onboarding` → `updateMyRealtorProfile` PATCH → reality-server returns 401 → UI shows generic failure → realtor cannot publish or update listings.
2. ← `request()` at `frontend/apps/reality-web/src/lib/realtor-api.ts:22-33` builds `headers` without `Authorization`.
3. ← `getAuthHeader()` is defined at `frontend/apps/reality-web/src/lib/auth-token.ts:84` and used by `auth-api.ts:43`, but never imported by `realtor-api.ts`.
4. Origin: `realtor-api.ts` was added as a stopgap "until the SDK is regenerated" (docstring) and only mirrored the SSR-safe `credentials: 'include'` shape from `env`/comparison fetches, not the form-login bearer pattern from `auth-api.ts`. `credentials: 'include'` was sufficient during the OAuth/SSO cookie prototype but became insufficient once form-login (bearer-only) shipped as the actual login path.

## Test plan
- [x] `frontend/apps/reality-web/src/lib/realtor-api.test.ts` — new file with three fetch-stub cases (see *Suggested approach* step 3). Fails on `main`; passes after the header spread lands.
- [x] `pnpm --filter @ppt/reality-web typecheck` — confirms the new `getAuthHeader` import does not regress types.

## Out of scope
- Regenerating `@ppt/reality-api-client` to expose the realtor mutations natively (separate refactor; docstring pledges it as a follow-up).
- Adding tests for the sibling read-only helpers in `realtor-api.ts` (covered by `code-review-reality-web-realtor-api-no-tests` as its own test-gap item).
- Fixing the sibling i18n regression in `CompareButton.tsx` (`code-review-reality-web-comparebutton-hardcoded-english`, score 2).

## After-merge
- Manually smoke-test `/en/auth/login` → `/en/realtor/onboarding` → update profile once against a dev reality-server; assert the request carries an `Authorization` bearer header in the browser network tab. (Not strictly required by verify-all; documented so the reviewer can spot-check.)
- Set `.research/backlog.json` `code-review-reality-web-realtor-api-missing-bearer-header.status = "done"` on merge; append `"resolved: PR #<N>"` to `evidence`.
