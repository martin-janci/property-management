# code-review-ppt-web-core-dispute-draft-persist

**Vector:** security
**Score:** 2
**Source:** rotating-expert-review ppt-web-core 2026-10-05
**Confidence:** high

## Hypothesis
The ppt-web dispute-filing form draft (`type`, `subject`, `description`, `unitId`) is persisted to `localStorage` under key `ppt-dispute-filing-draft`, but `AuthContext.logout` clears only the 4 auth keys via `tokenStorage.clear` and never touches the dispute draft. On a shared browser the next user who logs in and navigates to File Dispute reads the prior user's abandoned draft — including neighbor names, descriptions, and legal grievances. The smallest fix clears the dispute draft (and any other feature-owned localStorage drafts) at logout, mirroring the explicit allow-list pattern already used for TanStack Query roots in `AuthContext.logout`.

## Evidence
- `frontend/apps/ppt-web/src/features/disputes/hooks/useDraftStorage.ts:88-99` — `flush` writes `{values, savedAt}` to `window.localStorage.setItem(key, JSON.stringify(payload))` with no TTL.
- `frontend/apps/ppt-web/src/features/disputes/hooks/useDraftStorage.ts:107-125` — the only `removeItem(key)` sits inside the hook's own `clear()`, which is called by `FileDisputePage` solely on successful submit (`mutation.onSuccess`).
- `frontend/apps/ppt-web/src/features/disputes/pages/FileDisputePage.tsx:23,124` — `DISPUTE_DRAFT_KEY = 'ppt-dispute-filing-draft'` is passed into `useDraftStorage<Partial<DisputeFormValues>>` at page mount.
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:186-195` — `tokenStorage.clear()` calls `localStorage.removeItem` on only four keys (`ACCESS_TOKEN_KEY`, `REFRESH_TOKEN_KEY`, `USER_KEY`, `TENANTS_KEY`); the `ppt-dispute-filing-draft` key is not in the list.
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:614-638` — `logout()` runs `tokenStorage.clear()`, `setUser(null)`, loops `AUTHED_QUERY_KEY_ROOTS` for query cache purge, then best-effort server logout. No localStorage sweep touches feature-owned keys.

## Files
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:614`
- `frontend/apps/ppt-web/src/features/disputes/pages/FileDisputePage.tsx:23`
- `frontend/apps/ppt-web/src/features/disputes/hooks/useDraftStorage.ts`
- `frontend/apps/ppt-web/src/lib/queryKeys.ts`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode:** `Mode: cloud-ok` (frontend-only change; vitest + jsdom cover the localStorage contract — no browser DOM capture needed).

## Repro steps
1. Start the ppt-web dev server and sign in as **user A** (any role).
2. Navigate to `/disputes/file` (FileDisputePage). Pick a dispute type, fill `subject`/`description` with any identifying text (e.g. `"Noise complaint about neighbour <name>"`), close the page without submitting.
3. In DevTools: `localStorage.getItem('ppt-dispute-filing-draft')` returns the serialised draft with user A's fields.
4. Click the user menu → **Logout**. Confirm `localStorage.getItem('ppt_access_token')` is `null`.
5. In the same tab, log in as **user B** (different account). Navigate to `/disputes/file`.
6. Expected: form starts empty. Actual: user A's `subject` and `description` are prefilled from `localStorage['ppt-dispute-filing-draft']` and the "Draft saved …" indicator is rendered.

## Suggested approach
1. Introduce an exported constant `FEATURE_DRAFT_KEYS: readonly string[]` (co-located with `AUTHED_QUERY_KEY_ROOTS` in `frontend/apps/ppt-web/src/lib/queryKeys.ts` for discoverability) that enumerates every feature-owned localStorage draft key — initial entry `'ppt-dispute-filing-draft'`. Each new draft key the frontend adds must land in this allow-list; this mirrors the existing explicit `AUTHED_QUERY_KEY_ROOTS` discipline that `queryKeys.ts` already documents.
2. In `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:186-195`, extend `tokenStorage.clear` (or add a sibling `clearFeatureDrafts()` helper if semantic grouping is preferred) to iterate `FEATURE_DRAFT_KEYS` and `localStorage.removeItem(key)` each — wrapped in the same `try { … } catch {}` the current function uses for storage-unavailable browsers.
3. Ensure `logout` at `frontend/apps/ppt-web/src/contexts/AuthContext.tsx:618` invokes the extended cleanup (automatic if folded into `tokenStorage.clear`; explicit call if separate helper).
4. Export `DISPUTE_DRAFT_KEY` from `FileDisputePage.tsx` already does; add a comment at that export line referencing `FEATURE_DRAFT_KEYS` so future authors register new draft keys there.
5. Add a vitest unit test next to the existing `FileDisputePage.test.tsx` that: (a) seeds `localStorage['ppt-dispute-filing-draft']` with a payload, (b) calls `tokenStorage.clear()` (or the exported logout helper), (c) asserts `localStorage.getItem('ppt-dispute-filing-draft')` is `null`. Add a second case asserting the four existing auth keys are still cleared (no regression).
6. Add a vitest unit test for `AuthContext.logout` that mounts a minimal `<AuthProvider>`, seeds the draft, calls `logout()`, and asserts the draft key is gone.
7. Run `pnpm -F @ppt/web test` to verify both the new and existing dispute tests pass.

## Alternatives considered
- **Convert `useDraftStorage` to `sessionStorage` instead of `localStorage`** — rejected because the AC-4 design in `useDraftStorage.ts` explicitly chose `localStorage` so the draft survives an accidental tab close (not just page nav); switching to `sessionStorage` would regress that recovery contract and leaves the shared-browser leak unaddressed if the user never closes the tab between sessions.
- **Call `localStorage.clear()` in `logout`** — rejected because it would also wipe non-draft legitimate data (e.g. theme preference, i18n locale, DevTools-only keys) and makes the blast radius of future feature additions unpredictable. An explicit allow-list keeps the cleanup intentional and reviewable.

## Root-cause trace
1. Symptom: user A's dispute-form fields (subject/description) appear prefilled when user B navigates to `/disputes/file` on the same browser.
2. ← `FileDisputePage.tsx:124` calls `useDraftStorage<DisputeFormValues>(DISPUTE_DRAFT_KEY)` on mount; the hook reads `localStorage.getItem(DISPUTE_DRAFT_KEY)` and seeds `defaultValues` from it.
3. ← `useDraftStorage.ts:107-125` clears the key only when the consumer calls `.clear()`, which `FileDisputePage` fires only on successful submission (`mutation.onSuccess`). An abandoned draft is never cleared by the hook on unmount.
4. ← `AuthContext.tsx:186-195` `tokenStorage.clear` lists exactly 4 keys; the dispute draft is not among them, and `logout()` at `:614-638` performs no other localStorage sweep.
5. Origin: the feature draft contract was introduced alongside AC-4 (see `useDraftStorage.ts` docstring at line 1-10 and `FileDisputePage.tsx:23` export comment). The gap is the missing coupling between feature-owned draft keys and the auth logout path — no central registry exists for feature draft keys.

## Test plan
- [ ] New: `frontend/apps/ppt-web/src/features/disputes/pages/FileDisputePage.test.tsx` — add "logout clears abandoned dispute draft" case that seeds the draft, triggers logout (or `tokenStorage.clear`), and asserts the draft key is removed and the four auth keys still cleared.
- [ ] New: `frontend/apps/ppt-web/src/contexts/AuthContext.test.tsx` — add a case that mounts `<AuthProvider>`, seeds `localStorage['ppt-dispute-filing-draft']`, calls `logout()`, and asserts `localStorage.getItem('ppt-dispute-filing-draft') === null`.
- [ ] Regression: existing `FileDisputePage.test.tsx` draft-hydration test still passes (draft remains readable within a single session, pre-logout).
- [ ] Command: `cd frontend && pnpm -F @ppt/web test -- src/features/disputes src/contexts/AuthContext.test.tsx`.

## Out of scope
- Any server-side change (the leak is entirely client-side storage; the dispute POST endpoint is unaffected).
- Encryption/at-rest protection of drafts in localStorage (not required when the key is cleared at logout; a separate hardening plan if needed).
- Converting `useDraftStorage` to IndexedDB or any other storage layer.
- Clearing drafts other than `DISPUTE_DRAFT_KEY` — the allow-list is introduced empty-but-for-dispute; follow-ups add new keys as they are introduced.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-dispute-draft-persist.md`
- Mark the matching `backlog.json` row as `status: "done"`
