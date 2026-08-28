# code-review-ppt-web-ui-registry-page-window-confirm-hardcoded-copy

**Vector:** bug
**Score:** 3
**Source:** ppt-web-ui segment review 2026-08-28
**Confidence:** high

## Hypothesis
`RegistryPage.tsx` still gates 4 destructive/approve actions on `window.confirm` with hardcoded English copy — the exact anti-pattern PRs #2829/#2855/#2849 removed from the compliance surface. The native dialog blocks the main thread, is inaccessible, un-styleable, and ships an untranslated string to every non-English locale. Replacing with the shared `ConfirmationDialog` component + `react-i18next` translations closes a consistent UX regression across the pets/vehicles registry.

## Evidence
- `frontend/apps/ppt-web/src/features/registry/pages/RegistryPage.tsx:100,114,157,171` — 4× `window.confirm(...)` for pet delete, pet approve, vehicle delete, vehicle approve
- Approve handlers already carry a `// Simple approval - in a real app, this would open a modal to approve/reject with reason` TODO left in-code (line 113 preamble)
- Precedent: `frontend/apps/ppt-web/src/features/compliance/pages/ContentModerationPage.tsx` and `AmlDashboardPage.tsx` migrated to `<ConfirmationDialog>` + `t(...)` in PR #2829 and #2849
- Precedent: `messages/{cs,de,en,hu,pl,sk}.json` locale patterns from PR #2849 give a copy-paste template

## Files
- `frontend/apps/ppt-web/src/features/registry/pages/RegistryPage.tsx`
- `frontend/apps/ppt-web/messages/en.json`
- `frontend/apps/ppt-web/messages/sk.json`
- `frontend/apps/ppt-web/messages/cs.json`
- `frontend/apps/ppt-web/messages/de.json`
- `frontend/apps/ppt-web/messages/hu.json`
- `frontend/apps/ppt-web/messages/pl.json`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. `pnpm -F @ppt/web dev` and sign in as a property manager with pending pet + vehicle registrations
2. Open Registry → Pets tab; click "Delete" on a pending registration. Expected: styled `ConfirmationDialog` in the active locale. Actual: browser `window.confirm(...)` with English text `"Are you sure you want to delete this pet registration?"`
3. Repeat for pet approve, vehicle delete, vehicle approve — all four surfaces show the native dialog with hardcoded English
4. In Safari/Firefox, tick "Prevent this page from creating additional dialogs" after the first confirm; every subsequent action silently no-ops with no user feedback (same failure mode as `PrivacySettingsPage` GDPR flow — see sibling plan)

## Suggested approach
1. Import `ConfirmationDialog` (shared component used by `ContentModerationPage.tsx`) and `useTranslation` from `react-i18next` in `RegistryPage.tsx`.
2. Add per-action local state (`pendingDeletePetId`, `pendingApprovePetId`, `pendingDeleteVehicleId`, `pendingApproveVehicleId`) instead of gating the mutation inside a `window.confirm`. Open the dialog with the id + action; the dialog's confirm handler triggers the mutation.
3. For the approve actions, extend the dialog with an optional multi-line "reason" textarea (fulfilling the in-code TODO). Pass `reason` in the mutation payload (`reviewPetMutation.mutateAsync({ id, data: { approve: true, reason }})`) if the backend accepts it, otherwise land the UI change and file a follow-up on the API contract.
4. Extract all UI copy into i18n keys under a new `registry.dialogs.*` namespace in `messages/en.json`; mirror to `sk/cs/de/hu/pl` (use `en.json` as a fallback template — trust the existing translation contributor pattern from PR #2849).
5. Sweep the file for the two remaining `console.error('Failed to ...', err)` calls at L108 and L165 — replace with the shared toast/logging path used by `useAmlDashboard` (or leave alone if the sibling `code-review-ppt-web-ui-compliance-console-error-in-onerror-handlers` item lands first — keep this plan scoped to the confirm-dialog swap so the two don't collide).
6. Snapshot the new dialog markup in `RegistryPage.i18n.snapshot.test.tsx` (mirror the pattern from `VerificationBadge.i18n.snapshot.test.tsx` added in #2850). Assert that a `window.confirm` spy is NEVER called from any of the 4 handlers.
7. `pnpm -F @ppt/web check && pnpm -F @ppt/web test -- RegistryPage` should be green.

## Alternatives considered
- **Leave native confirm, only i18n the strings via `t(...)`** — rejected because the a11y regression (main-thread blocking, screen-reader focus loss) survives, and the silent-drop behavior when the user ticks Firefox/Safari dialog-suppression is unchanged. Copy is only half the defect.
- **Custom per-action modals instead of the shared ConfirmationDialog** — rejected because that duplicates styling and locale copy across 4 sites (and future ones), invites drift, and rejects the compliance-page precedent that just landed.

## Root-cause trace
1. Symptom: 4 destructive/approve actions gate on native `window.confirm` with hardcoded English (registry pet delete + approve; vehicle delete + approve)
2. ← Immediate cause at `RegistryPage.tsx:100,114,157,171` — handlers call `window.confirm(...)` inline instead of dispatching a dialog
3. ← Upstream cause: the compliance-page migration to `ConfirmationDialog` (PR #2829) did not sweep the registry surface. The pattern was documented but not applied
4. Origin: initial RegistryPage.tsx scaffolding (pre-dates the compliance migration); the file has been read-only since the compliance sweep

## Test plan
- [x] Add `RegistryPage.i18n.snapshot.test.tsx` — snapshot the pet-delete dialog in the 6 supported locales; assert `jest.spyOn(window, 'confirm')` is never called
- [x] Extend the existing (or new) `RegistryPage.test.tsx` — regression: click delete → dialog opens; click confirm → `deletePetMutation.mutateAsync` fires with the correct id; click cancel → mutation NOT fired
- [x] `pnpm -F @ppt/web test -- --filter RegistryPage` and `pnpm -F @ppt/web check`

## Out of scope
- Backend contract for the approve "reason" field — if the reviewPetMutation body doesn't accept `reason` yet, ship the UI with a TODO and open a follow-up
- The `console.error` sweep across compliance/registry — handled by `code-review-ppt-web-ui-compliance-console-error-in-onerror-handlers`
- Non-confirm dialogs (create-pet / edit-pet modals) — not affected

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-ui-registry-page-window-confirm-hardcoded-copy.md`
- Mark the matching `backlog.json` row as `status: "done"`
