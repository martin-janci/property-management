# code-review-ppt-web-ui-workflow-automation-mutation-silent-fail

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review 2026-09-23 ppt-web-ui
**Confidence:** high

## Hypothesis
Three pages in the ppt-web workflow-automation feature call `mutateAsync()` without try/catch and without rendering `mutation.error`, so a failed rule delete, template use, or execution retry looks identical to success — dialogs close, buttons re-enable, and the user has no way to tell the operation actually failed. The correct pattern is already in-file at `MediationWorkspacePage.tsx:100-190` (try/catch + `onToastError`) and at `CreateRulePage.tsx:62` / `EditRulePage.tsx:113` (which render `mutation.error`); the fix is to wrap each failing handler in try/catch and surface the error via a toast, matching the pattern reality-web PRs #2967/#2968/#2969 landed last week.

## Evidence
- `frontend/apps/ppt-web/src/features/workflow-automation/pages/AutomationRulesPage.tsx:56-72` — confirmDelete / handleToggle / handleRun each `await mutation.mutateAsync(...)` with no try/catch and no `.error`/`.isError` branch anywhere in the file (only `.isPending` at :303/:306).
- `frontend/apps/ppt-web/src/features/workflow-automation/pages/TemplateLibraryPage.tsx:53-60` — handleUseTemplate does `const result = await createFromTemplate.mutateAsync(...); if (result?.id) navigate(...) else navigate('/automations/rules/new', ...);` — on rejection the unhandled promise propagates and neither navigate runs, but nothing tells the user.
- `frontend/apps/ppt-web/src/features/workflow-automation/pages/ExecutionMonitoringPage.tsx:74-77` — handleRetry does `await retryExecution.mutateAsync(log.id); setSelectedLog(null);` — a failed retry closes the details modal and hides the error state.
- `grep -rn MutationCache frontend/apps/ppt-web/src/` returns 0 hits — no global onError toast fallback exists to catch these either.
- Contrast: `frontend/apps/ppt-web/src/features/disputes/pages/MediationWorkspacePage.tsx:100-190` already uses the correct try/catch + `onToastError(t('...error'), err.message)` shape.

## Files
- `frontend/apps/ppt-web/src/features/workflow-automation/pages/AutomationRulesPage.tsx`
- `frontend/apps/ppt-web/src/features/workflow-automation/pages/TemplateLibraryPage.tsx`
- `frontend/apps/ppt-web/src/features/workflow-automation/pages/ExecutionMonitoringPage.tsx`
- `frontend/apps/ppt-web/src/features/disputes/pages/MediationWorkspacePage.tsx`

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
1. Start ppt-web dev server, log in as a manager with workflow-automation access.
2. Open Automations → Rules, click the trash icon on any rule, block the outgoing `DELETE /api/v1/automations/rules/:id` request in devtools (or force a 500 response via the bridge MCP).
3. Confirm the delete in the dialog.
4. Expected: an error toast surfaces "Failed to delete rule — <reason>" and the rule stays in the list.
5. Actual: the confirm dialog closes, the "Deleting..." spinner goes away, the rule remains in the list, and no user-facing feedback explains why the operation didn't take effect. Repeat with the toggle switch (PATCH /rules/:id `{isEnabled}`), Run button (POST /rules/:id/run), TemplateLibrary "Use template" (POST /rules/from-template), and ExecutionMonitoring retry (POST /executions/:id/retry) — same silent-failure shape on each.

## Suggested approach
1. In `AutomationRulesPage.tsx:56-72`, wrap `confirmDelete`, `handleToggle`, `handleRun` in try/catch that calls an `onToastError`-shaped helper (import from the shared toast provider, matching the shape at `MediationWorkspacePage.tsx:56`).
2. In `TemplateLibraryPage.tsx:53-60`, wrap `handleUseTemplate` similarly; on caught error, surface a toast with `t('automations.templates.useError')` (add the i18n key to `en.json` + all locales) and stay on the Template Library page.
3. In `ExecutionMonitoringPage.tsx:74-77`, wrap `handleRetry`; on error keep `selectedLog` set so the details modal stays open and render `retryExecution.error?.message` inline in the modal footer.
4. Add matching i18n keys to `frontend/apps/ppt-web/src/i18n/locales/{en,sk,cs,de}.json` (only `en` needed for a first landing; the reality-web PRs use the same discipline).
5. Add Vitest tests for each page that stub the mutation to reject and assert an error toast is shown / details modal stays open.
6. `pnpm --filter @ppt/web check && pnpm --filter @ppt/web typecheck && pnpm --filter @ppt/web test`.
7. Confirm the reality-web PRs #2967/#2968/#2969 have landed and use the exact same shape — align implementation to keep a single review-pattern across products.

## Alternatives considered
- **Add a global MutationCache `onError` toast in `query-provider.tsx`** — rejected because it would ship a catch-all that swallows explicit page-level handling; the disputes flow already has bespoke error copy per action and would double-toast.
- **Suppress the failure symptom by closing the dialog only in the `onSuccess` callback of each mutation** — rejected because it leaves the still-open dialog with no explanatory error state; users hit `retry` without knowing what went wrong. Wrapping with a toast keeps the dialog logic intact.

## Root-cause trace
1. Symptom: destructive workflow-automation actions (delete, toggle, run, use-template, retry) succeed to the eye even when the underlying mutation rejects.
2. ← Immediate cause at `AutomationRulesPage.tsx:58`, `TemplateLibraryPage.tsx:54`, `ExecutionMonitoringPage.tsx:75`: `await mutation.mutateAsync(...)` with no try/catch.
3. ← Upstream cause at `frontend/apps/ppt-web/src/lib/query-provider.tsx`: no MutationCache-level `onError` global toast fallback.
4. Origin: the workflow-automation feature was scaffolded without adopting the error-handling convention that `disputes/`, `CreateRulePage`, and `EditRulePage` had already established. Same class as the reality-web PRs #2967/#2968/#2969 (import/CRM+Feed wizards) that fixed this exact anti-pattern last week.

## Test plan
- [ ] `frontend/apps/ppt-web/src/features/workflow-automation/pages/__tests__/AutomationRulesPage.test.tsx` — stub `deleteRule` to reject, assert `onToastError` is called and the rule stays in the list; repeat for `updateRule` (toggle) and `runRule`.
- [ ] `frontend/apps/ppt-web/src/features/workflow-automation/pages/__tests__/TemplateLibraryPage.test.tsx` — stub `createFromTemplate` to reject, assert toast error and no navigation.
- [ ] `frontend/apps/ppt-web/src/features/workflow-automation/pages/__tests__/ExecutionMonitoringPage.test.tsx` — stub `retryExecution` to reject, assert the details modal stays open with the error surfaced.
- [ ] `pnpm --filter @ppt/web test` and `pnpm --filter @ppt/web typecheck` both green.

## Out of scope
- Do NOT add a global MutationCache `onError` handler — a separate design conversation.
- Do NOT refactor the workflow-automation feature's data-fetching hooks; scope is limited to the three handler bodies + i18n keys + tests.
- Reality-web mutation-silent-fail follow-ups (PRs #2967/#2968/#2969) — already landing on their own PRs; this plan is ppt-web-only.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-ui-workflow-automation-mutation-silent-fail.md`
- Mark the matching `backlog.json` row as `status: "done"`
