# code-review-ppt-web-ui-registry-rules-save-silent-fail

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review (ppt-web-ui segment, 2026-10-01)
**Confidence:** high

## Hypothesis
`RegistryRulesPage.handleSubmit` catches a failed `updateRulesMutation.mutateAsync` and only logs to `console.error`; the UI never surfaces an error, and the success path is also silent (the inline comment acknowledges "could show a toast notification here"). From the user's perspective a save that failed looks identical to a save that succeeded — the form stays open, no toast fires, no error banner appears, and the user is invited to resubmit (potentially with stale state) or walk away believing their change landed. Routing both branches through a toast notification (and letting the mutation's `isError` surface a visible banner) collapses the three states to one visible outcome each.

## Evidence
- `frontend/apps/ppt-web/src/features/registry/pages/RegistryRulesPage.tsx:48-51` — `catch (err) { console.error('Failed to update registry rules:', err); }` is the entire failure branch. No `setError` state, no toast call, no re-throw.
- `frontend/apps/ppt-web/src/features/registry/pages/RegistryRulesPage.tsx:46-47` — Success branch is also silent: `// Success - could show a toast notification here` next to the awaited `mutateAsync`. The caller never knows.
- Phase 1.5 rotating-expert-review ppt-web-ui 2026-10-01 (frontend expert) flagged this as a +3 high-confidence bug; the sibling toast-i18n finding (`code-review-ppt-web-ui-workflow-automation-toasts-hardcoded-english`, score 2) shows that the ppt-web convention for a page-level save IS to raise a toast.
- Churn-adjacent: `features/registry/pages/RegistryRulesPage.tsx` was NOT in the 14-day churn hotspot set — the swallowed-error bug has been quietly in place since Story 57.7 landed. Low churn risk for the fix.

## Files
- `frontend/apps/ppt-web/src/features/registry/pages/RegistryRulesPage.tsx:48`
- `frontend/apps/ppt-web/src/features/registry/pages/RegistryRulesPage.tsx:46`

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
1. In Vitest, mount `<RegistryRulesPage />` wrapped in a QueryClientProvider + MemoryRouter at `/buildings/:buildingId/registry/rules` with `buildingId="bld-1"`.
2. Mock `registryHooks.useUpdateRegistryRules` so that `mutateAsync` rejects with `new Error('500')`.
3. Fill the form via `RegistryRulesForm` and click Save.
4. Assert the DOM contains a visible error banner / toast referencing the failure. **Current behaviour**: nothing user-visible — the test fails because no error message appears.
5. Change the mock so that `mutateAsync` resolves. Assert a success toast appears. **Current behaviour**: fails for the same reason.

## Suggested approach
1. Add a top-level toast hook — reuse whatever the automation-rules pages use (`AutomationRulesPage.tsx:66-114` demonstrates the pattern; prefer the shared `useToast()` helper if one exists, otherwise introduce a thin wrapper in `features/registry/pages/RegistryRulesPage.tsx`).
2. In `handleSubmit` success branch (`RegistryRulesPage.tsx:46-47`): emit a success toast `t('registry.rules.toast.saved')` with a short body.
3. In the catch branch (`RegistryRulesPage.tsx:48-51`): keep the `console.error`, add an error toast `t('registry.rules.toast.saveFailed')` with the server's error message surfaced via `err instanceof Error ? err.message : <fallback>`.
4. Add i18n keys to `frontend/apps/ppt-web/messages/{en,sk,cs,de,hu,pl}.json` under `registry.rules.toast.*` (`saved`, `saveFailed`, body copy for each). Keep English and Slovak at parity; the other four match English for the first pass.
5. Add a regression test `RegistryRulesPage.test.tsx` next to the component that covers the two Repro-steps assertions. Fails on `main`, passes with the fix (IG3).
6. Run `pnpm -F @ppt/web test`, `pnpm -F @ppt/web typecheck`, `pnpm check` (Biome lint + format).

## Alternatives considered
- **Inline error banner above the form** — rejected because it duplicates the `error || !buildingId` 500-banner UX already at `RegistryRulesPage.tsx:79-98` and would require a second error state the rest of ppt-web pages do not carry; the shared toast pattern is already proven across automation and compliance pages.
- **Re-throw from the catch so the mutation's `isError` surfaces** — rejected because `handleSubmit` is bound to a submit event, not to React Query's error boundary; re-throwing would hit `useAsyncError`-style uncaught promise handling and still not show the user a toast without further plumbing.

## Root-cause trace
1. Symptom: Build-manager clicks Save with a bad payload (or during a backend 500); the modal/button stays idle, the user gets no confirmation either way.
2. ← Immediate cause: `RegistryRulesPage.tsx:48-51` — the catch block only calls `console.error`, no user-visible side effect.
3. ← Upstream cause: `RegistryRulesPage.tsx:46-47` — the success branch is also commented out (`// Success - could show a toast notification here`), so there is no symmetric notification scaffold to lift the failure handling into.
4. Origin: Story 57.7 initial scaffold of `RegistryRulesPage.tsx` (feature-registry introduction PR). The TODO comment on the success branch is the fossil of the deferred toast integration.

## Test plan
- [ ] `frontend/apps/ppt-web/src/features/registry/pages/RegistryRulesPage.test.tsx` — new file with the two Repro assertions (error-toast on reject, success-toast on resolve).
- [ ] Regression scenario: unmount the page immediately after `mutateAsync` rejects to ensure the toast ref doesn't leak (prevents React warning about state update on unmounted component).
- [ ] Local command: `cd frontend && pnpm --filter @ppt/web test -- --run src/features/registry/pages/RegistryRulesPage.test.tsx`

## Out of scope
- Rebuilding the registry rules read-side `useRegistryRules` error UI (already handled at `RegistryRulesPage.tsx:79-98`).
- Centralising the toast helper across the ppt-web app (left to the `workflow-automation-toasts-hardcoded-english` plan which has broader scope).
- Backend validation of `UpdateRegistryRulesRequest` payloads.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-ui-registry-rules-save-silent-fail.md`
- Mark the matching `backlog.json` row as `status: "done"`
