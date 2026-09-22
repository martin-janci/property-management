# code-review-reality-web-agency-branding-save-silent-fail

**Vector:** bug
**Score:** 3
**Source:** ppt-dev-review 2026-09-22 (reality-web segment)
**Confidence:** high

## Hypothesis
`AgencyBranding.handleSave` awaits `updateBranding.mutateAsync(...)` with no try/catch and unconditionally clears local state (`setHasChanges(false); setLogoFile(null); setCoverFile(null)`) on the next lines. `useUpdateBranding` in `frontend/packages/reality-api-client/src/agency/hooks.ts:258-288` defines only `onSuccess` — there is no `onError` fallback anywhere. When the multipart PUT to `/api/v1/agencies/{id}/branding` throws (network drop, 4xx from the backend, or `ApiError` on non-2xx), the promise rejection bubbles out of the handler as an unhandled rejection AND the just-selected `File` objects are dropped from React state, the "unsaved changes" indicator disappears, and the Save button reverts to disabled — the agency admin thinks their new logo/cover files landed when they did not. Smallest change: wrap the call in try/catch, keep `hasChanges` + the File refs when the mutation throws, and render `updateBranding.error` (or a toast) inline. Mirrors the pattern already in `InviteModal.handleSubmit` (`RealtorManagement.tsx:568-581`) and matches the fix shape landed by PR #2967 (`SyncSchedule.handleSave`) and PR #2969 (`import` create wizards).

## Evidence
- `frontend/apps/reality-web/src/components/agency/AgencyBranding.tsx:78-95` — the entire `handleSave` body: `await updateBranding.mutateAsync(...); setHasChanges(false); setLogoFile(null); setCoverFile(null);` with no error path.
- `frontend/packages/reality-api-client/src/agency/hooks.ts:258-288` — `useUpdateBranding` defines only `mutationFn` + `onSuccess`. `onError` is absent, so the mutation surfaces no toast/i18n message.
- Sibling `InviteModal.handleSubmit` at `frontend/apps/reality-web/src/components/agency/RealtorManagement.tsx:568-581` already demonstrates the correct pattern: try/catch, `setInviteError(t('inviteError'))`, keep the modal open on failure; the rendered alert lives at L621-625 in the same file.
- Class of bug already landed on other reality-web silent-fail sites: PR #2967 `code-review-reality-web-syncschedule-save-silent-fail`, PR #2968 `code-review-reality-web-feedcard-mutation-silent-fail`, PR #2969 `code-review-reality-web-import-create-mutation-silent-fail`.

## Files
- `frontend/apps/reality-web/src/components/agency/AgencyBranding.tsx`
- `frontend/packages/reality-api-client/src/agency/hooks.ts`

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
1. Log in to reality-web as an agency admin who owns at least one agency.
2. Navigate to the agency branding page and pick a new logo File + a new cover File via the file inputs.
3. Force the PUT to fail — either use DevTools Network → block `PUT **/api/v1/agencies/*/branding`, or throttle offline; alternatively point at a local `reality-server` returning 500 for `PUT /branding`.
4. Click "Save".
5. Expected: the wizard stays put, an error is visible ("Failed to save branding" or similar), the File selections and "unsaved changes" indicator are preserved so the user can retry.
6. Actual (today): the handler rethrows as an unhandled promise rejection; the Save button becomes disabled, the File pickers reset, and the user sees no error UI — they believe the save succeeded.

## Suggested approach
1. In `frontend/apps/reality-web/src/components/agency/AgencyBranding.tsx`, add local `saveError` state (`useState<string | null>(null)`).
2. Rewrite `handleSave` to clear `saveError` up front, wrap the `mutateAsync` in try/catch, only clear `hasChanges` / `logoFile` / `coverFile` inside the try branch after the await resolves, and set `saveError` (i18n-scoped, e.g. `t('brandingSaveFailed')`) in the catch branch. Log to `console.error` for parity with the `InviteModal` pattern.
3. Render `{saveError && <div className="branding-error" role="alert" aria-live="assertive">{saveError}</div>}` above the Save button (or wherever the section's form summary lives). Reuse an existing error class from `RealtorManagement.tsx` (`.invite-error` / equivalent) so no new CSS is required.
4. Add an i18n key `agency.brandingSaveFailed` to each locale JSON under `frontend/apps/reality-web/messages/{en,sk,cs,de}.json`; use a short human-readable message (mirrors the tone of `agency.inviteError`).
5. Optional: add an `onError` in `useUpdateBranding` (`frontend/packages/reality-api-client/src/agency/hooks.ts:258-288`) that invalidates nothing but rethrows so callers keep control — leave out if the try/catch at the callsite is enough (this plan does not require it).
6. Add a Vitest test in `frontend/apps/reality-web/src/components/agency/AgencyBranding.test.tsx` (or extend an existing test module for that component) that stubs the mutation to reject, invokes `handleSave`, and asserts the error node is rendered and `logoFile` was preserved.
7. Run `pnpm --filter @ppt/reality-web test` and `pnpm --filter @ppt/reality-web lint` — CI runs Biome + typecheck + vitest.

## Alternatives considered
- **Add `onError` inside `useUpdateBranding` and surface via a global toast** — rejected because the reality-web codebase currently surfaces errors as inline `role="alert"` regions per-component (see `InviteModal.inviteError`, `syncschedule-save-silent-fail` fix in PR #2967). Adding a toast infrastructure would widen the diff without matching the existing convention.
- **Rethrow inside `handleSave` and let a top-level error boundary catch it** — rejected because that would unmount the whole branding form (route-level error boundary) and the user would lose all in-progress state; keeping the failure local preserves the retry path.

## Root-cause trace
1. Symptom: agency admin selects new logo + cover, clicks Save while the PUT fails; UI acts as if the save landed but the backend didn't receive the files.
2. ← `AgencyBranding.tsx:78-95` `handleSave` awaits `updateBranding.mutateAsync(...)` without try/catch, then unconditionally calls `setHasChanges(false)` / `setLogoFile(null)` / `setCoverFile(null)`.
3. ← `frontend/packages/reality-api-client/src/agency/hooks.ts:258-288` `useUpdateBranding` supplies no `onError`, so the mutation rejection surfaces nowhere.
4. Origin: introduced in the initial branding editor commit (`AgencyBranding.tsx` was added as part of the agency-branding feature; the silent-fail shape was there from day one — same anti-pattern as the CRM/Feed and SyncSchedule handlers already in the routine's backlog).

## Test plan
- [ ] Extend `frontend/apps/reality-web/src/components/agency/AgencyBranding.test.tsx` with a `renders error + preserves file selections when updateBranding rejects` case (mock `useUpdateBranding` to return a mutation whose `mutateAsync` throws; assert `saveError` node visible via `screen.getByRole('alert')` and that the file preview URL is still rendered).
- [ ] The test must fail on today's `main` (no try/catch → the promise rejects and the test's assertion for the alert would never resolve; use `expect(...).rejects.toThrow()` on the direct handler call as the failing-on-main IG3).
- [ ] Command: `pnpm --filter @ppt/reality-web test -- AgencyBranding` (add to CI via existing `frontend.yml`).

## Out of scope
- The `useUpdateBranding` hook signature / return shape (no API change).
- Backend behaviour on PUT `/api/v1/agencies/{id}/branding` — same handler, same contract.
- The other 4 mutation callsites in `AgencyBranding.tsx` that already error-path correctly (this plan touches only `handleSave`).
- Reworking the toast/error surface into a shared component (kept as inline `role="alert"` per current convention).

## After-merge
- Update `docs/screens/reality/agency-branding.md` Agent Log: `2026-09-22 — routine: surfaced save-failure error path`.
- Watch for regressions in `dependabot` bumps of `@tanstack/react-query` — this fix depends on `useMutation`'s error contract; a breaking upgrade would need parallel changes.
- Consider a follow-up plan to add the same error-path treatment to the remaining silent-fail sites the ppt-dev-review 2026-09-22 pass surfaced but this run did not promote (`code-review-reality-web-realtor-detail-modal-silent-fail` — promoted alongside — and `code-review-reality-web-realtor-resend-unhandled-rejection` — left in backlog at score 2 pending a stackable second signal).
