# code-review-ppt-web-ui-content-mod-unsafe-cast

**Vector:** bug
**Score:** 3
**Source:** ppt-web-ui segment review 2026-08-25 (Phase 1.5 code-review slice); ContentModerationPage.tsx:129-176
**Confidence:** high

## Hypothesis
`ContentModerationPage.tsx` drives two regulated write flows — moderation actions (`takeAction`) and appeal decisions (`decideAppeal`) — through `window.prompt`, then casts the free-text return value straight to a TypeScript union (`as 'remove' | 'restrict' | 'warn' | 'approve'`, `as 'uphold' | 'overturn'`) before POSTing to the API. The cast is a lie: TypeScript erases at runtime, so a typo like `aprove` or `Uphold` (any case) reaches the moderation-action / appeal-decision endpoint, and the compliance audit trail records a payload the server never validated shape-first. This is the exact anti-pattern PR #2829 removed from `AmlDashboardPage` (which now uses a typed `<select>` inside `ReviewAssessmentDialog` / `InitiateEddDialog`) — the same fix should land here.

## Evidence
- `frontend/apps/ppt-web/src/features/compliance/pages/ContentModerationPage.tsx:129` — `const actionType = window.prompt(t('moderation.prompts.actionType'), 'approve');`
- `frontend/apps/ppt-web/src/features/compliance/pages/ContentModerationPage.tsx:139` — `action_type: actionType as 'remove' | 'restrict' | 'warn' | 'approve',` (runtime-forged cast on free text).
- `frontend/apps/ppt-web/src/features/compliance/pages/ContentModerationPage.tsx:165` — `const decision = window.prompt(t('moderation.prompts.decision'), 'uphold');`
- `frontend/apps/ppt-web/src/features/compliance/pages/ContentModerationPage.tsx:175` — `decision: decision as 'uphold' | 'overturn',` (same anti-pattern on the appeal flow).
- Sibling reference (already fixed): `frontend/apps/ppt-web/src/features/compliance/components/ReviewAssessmentDialog.tsx:19` — `const AML_REVIEW_DECISIONS: readonly AmlReviewDecision[] = ['approve', 'reject', 'escalate'];` gates the AML decision through a `<select>` bound to the same API union. PR #2829 removed `window.prompt` / `alert` from `AmlDashboardPage` for exactly this reason; ContentModerationPage was not covered by that PR.

## Files
- `frontend/apps/ppt-web/src/features/compliance/pages/ContentModerationPage.tsx`
- `frontend/apps/ppt-web/src/features/compliance/components/index.ts`

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
1. Open the ContentModerationPage in the ppt-web app with at least one pending moderation case card visible (any seed with a `pending` case).
2. Click "Take Action" on a case card; when the first `window.prompt` appears, type any string that is **not** in `{'remove','restrict','warn','approve'}` — e.g. `aprove` (typo).
3. Type any non-empty string when the second prompt asks for a rationale.
4. Expected: the UI refuses to submit the invalid action_type (dialog stays open, or a typed `<select>` never accepts the value in the first place).
5. Actual (today): `takeAction.mutate` is invoked with `action_type: 'aprove'`. The API call goes out; the audit trail records a payload with a value the client's own TypeScript union claims is impossible. The same reproduction against "Decide Appeal" with a typoed `decision` (`Uphold`, `revert`, etc.) hits the same defect via `decideAppeal.mutate`.

## Suggested approach
1. Author `frontend/apps/ppt-web/src/features/compliance/components/TakeModerationActionDialog.tsx` — a modal dialog mirroring `ReviewAssessmentDialog`'s shape: `isOpen`, `isSubmitting?`, `onSubmit(action_type, rationale, notify_owner)`, `onClose`. Bind `action_type` to a `<select>` whose options are declared as `const MODERATION_ACTION_TYPES: readonly ModerationActionType[] = ['approve','warn','restrict','remove']` (import the union directly from `@ppt/api-client` so the client-server contract cannot drift). Require a non-empty rationale before enabling submit — matching the existing `if (!rationale) return;` guard. Expose a "notify owner" checkbox (currently hardcoded `true`).
2. Author `frontend/apps/ppt-web/src/features/compliance/components/DecideAppealDialog.tsx` in the same shape: `<select>` bound to `['uphold','overturn']` (again imported from the API client), required rationale.
3. Export both from `components/index.ts` (mirror the AML entries that PR #2829 added).
4. In `ContentModerationPage.tsx`:
   - Add `takeActionCaseId` and `decideAppealCaseId` state (nullable case-id strings), mirroring `eddAssessmentId` / `reviewAssessmentId` in AmlDashboardPage.
   - Replace `handleTakeAction` / `handleDecideAppeal` bodies: `setTakeActionCaseId(caseId)` / `setDecideAppealCaseId(caseId)` — no more `window.prompt`.
   - Render `<TakeModerationActionDialog key={takeActionCaseId ?? 'take-closed'} isOpen={takeActionCaseId !== null} …>` and `<DecideAppealDialog key={decideAppealCaseId ?? 'decide-closed'} isOpen={decideAppealCaseId !== null} …>`. The `key` binding is the same guard PR #2833 added to the AML dialogs (per-assessment remount) so per-case state cannot leak between cases.
   - Route the dialog's `onSubmit` into `takeAction.mutate({ caseId, request: { action_type, rationale, notify_owner } }, { onError })` and `decideAppeal.mutate({ caseId, request: { decision, rationale } }, { onError })`. Delete the two `window.prompt` blocks and the two `as '<union>'` casts.
   - Replace the `alert(t('moderation.prompts.actionError'))` / `alert(...appealError)` `onError` calls with an in-app error banner state (single `string | null`); alerts belong to the Phase-1 flow the AML fix already removed.
5. Add / extend i18n keys for the two dialogs' labels + option copy (submit / cancel / notify-owner / required-rationale-error) to `frontend/apps/ppt-web/messages/{cs,de,en,hu,pl,sk}.json`. Mirror the AML dialog namespaces (`moderation.dialogs.takeAction.*`, `moderation.dialogs.decideAppeal.*`).
6. Do **not** touch `handleAssignCase` (uses `assignCase.mutate` directly, no cast — no defect). Do **not** touch `handleShowOverdue` (tracked separately as `code-review-ppt-web-ui-content-mod-overdue-noop`).
7. Delete the two `// TODO(Phase-2): Replace window.prompt with modal form` comments in `ContentModerationPage.tsx` once the dialogs land — the TODOs are now the acceptance criteria, not an aspiration.

## Alternatives considered
- **Guard the cast inside `ContentModerationPage.tsx` (validate string against the union before calling `.mutate`)** — rejected because it keeps the `window.prompt` UX (poor accessibility, no cancel-vs-empty distinction, no keyboard tabbing between fields) and repeats the AML anti-pattern verbatim. The typed `<select>` variant is the only shape the audit trail can trust and the only shape consistent with the sibling AML fix.
- **Introduce a generic `<TypedPromptDialog options={...}>` reused across compliance flows** — rejected as scope creep; the AML flow ships two hand-written dialogs (`ReviewAssessmentDialog`, `InitiateEddDialog`) and picking that pattern up here keeps the diff diffable one-to-one against #2829. A generic dialog can land later if a third compliance surface needs the same shape.

## Root-cause trace
1. Symptom: moderation actions and appeal decisions accept arbitrary free-text (`aprove`, `Uphold`, `''` for typoed input from the second prompt slot after cancel) into regulated write endpoints; TypeScript's union is silently violated at runtime.
2. ← Immediate cause: `ContentModerationPage.tsx:139` and `:175` cast a `string | null` prompt return value with `as '<literal-union>'` — TypeScript erases the assertion at runtime, so any non-empty string reaches `takeAction.mutate` / `decideAppeal.mutate`.
3. ← Upstream cause: the Phase-1 `window.prompt` scaffolding shipped with a `TODO(Phase-2)` comment on both handlers (`:127` and `:163`) admitting the modal form was deferred. PR #2829 replaced the equivalent scaffolding on `AmlDashboardPage` (also Epic 90) but did not extend the fix to `ContentModerationPage`.
4. Origin: introduced with the initial Epic 67 / Epic 90 wiring of the moderation dashboard; the `as '<union>'` cast survived because the compiler cannot detect it and no runtime test asserted the payload shape after a prompt.

## Test plan
- [ ] `frontend/apps/ppt-web/src/features/compliance/pages/ContentModerationPage.test.tsx` — mount the page with one seeded pending case (mock `@ppt/api-client` the same way `AmlDashboardPage.test.tsx` does), click "Take Action" → assert a `<select>` is present with the four action options (`approve`,`warn`,`restrict`,`remove`) and that submitting with a non-listed option is impossible; assert the "Notify owner" checkbox defaults `true`.
- [ ] Same file — click "Decide Appeal" → assert a `<select>` with `uphold`,`overturn`; assert typed submit round-trips through the `decideAppeal.mutate` mock (spy on the request arg).
- [ ] Regression: reopening the "Take Action" dialog for a different case (via `openTakeActionDialog(1)` helper mirroring `openReviewDialog(cardIndex)` in `AmlDashboardPage.test.tsx`) — assert the rationale field is empty and the action `<select>` is reset to its default; matches the `key={id ?? '…-closed'}` guard PR #2833 introduced for AML.
- [ ] Command: `cd frontend && pnpm --filter @ppt/web vitest run src/features/compliance/pages/ContentModerationPage.test.tsx`.

## Out of scope
- `handleShowOverdue` no-op — tracked separately as `code-review-ppt-web-ui-content-mod-overdue-noop`.
- `handleViewContent` full-page-reload `window.location.href` (also has a `TODO(Phase-2)` for `useNavigate`) — orthogonal to the cast defect, not a compliance write.
- Localizing `DsaReportsPage` — tracked separately as `code-review-ppt-web-ui-dsa-reports-no-i18n`.
- Server-side hardening (the API should reject unknown enums too) — worth doing, but the client bug is real regardless; server-side validation is a defense-in-depth follow-up.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-ui-content-mod-unsafe-cast.md`
- Mark the matching `backlog.json` row as `status: "done"` (dispatcher reconciler usually does this automatically once the PR merges)
