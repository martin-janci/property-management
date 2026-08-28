# code-review-ppt-web-ui-privacy-deletion-window-prompt-gdpr

**Vector:** bug
**Score:** 3
**Source:** ppt-web-ui segment review 2026-08-28
**Confidence:** high

## Hypothesis
`PrivacySettingsPage.tsx` gates the app's single most destructive account action — GDPR right-to-erasure — on `window.prompt` with hardcoded English copy. `window.prompt` is browser-suppressible: in Firefox/Safari, after repeated dialogs, the user gets a "Prevent this page from creating additional dialogs" checkbox that makes future prompts return `null` silently. A `null` return is indistinguishable from user-cancel, so the deletion request silently no-ops with zero UI feedback — a compliance-grade failure on a GDPR path. Replacing the prompt with a typed `AccountDeletionDialog` (email retype input + explicit confirm) + i18n on the whole page closes the destructive-flow drop and the un-translated-copy regression at once.

## Evidence
- `frontend/apps/ppt-web/src/features/privacy/pages/PrivacySettingsPage.tsx:120` — `window.prompt('This will schedule your account and all associated data for permanent deletion...')` gates the deletion mutation
- Same file L109 `setSuccess('Data export requested. You will receive an email when it is ready.')` and L129 `setError('The email you entered does not match your account email.')` — page-wide `t(...)` bypass; the whole feature is un-translated
- Firefox behavior documented: https://developer.mozilla.org/en-US/docs/Web/API/Window/prompt#return_value ("if the user cancels the prompt, or the prompt is denied because the site has been throttled…")
- Precedent: `frontend/apps/ppt-web/src/features/compliance/pages/AmlDashboardPage.tsx` migrated the same anti-pattern (`window.prompt` for AML decision reasons) to typed modals in PR #2829

## Files
- `frontend/apps/ppt-web/src/features/privacy/pages/PrivacySettingsPage.tsx`
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
1. `pnpm -F @ppt/web dev` and sign in in Firefox (or Safari) as any user with a valid session
2. Navigate to Privacy settings → click "Request account deletion"
3. First time: the `window.prompt(...)` appears with hardcoded English text; retype your email, expect success. Actual: success toast fires
4. Immediately click "Request account deletion" a second time. Firefox shows a checkbox "Prevent this page from creating additional dialogs"; tick it and cancel
5. Click "Request account deletion" a third time. Expected: some UI feedback (dialog, error toast, disabled state). Actual: `handleRequestDeletion` returns immediately at the `if (!confirmation) return;` guard with no user-visible signal — the GDPR request is silently swallowed
6. Also confirm any locale switch (`sk`, `cs`, `de`, `hu`, `pl`) leaves the prompt copy in English

## Suggested approach
1. Add a new `AccountDeletionDialog` component (or extend a shared `ConfirmationDialog` with an `emailConfirmInput` slot) that opens instead of `window.prompt`. State transitions: idle → dialog-open (form with retype-email input) → submitting (button disabled + spinner) → done or error.
2. Replace the `handleRequestDeletion` body: open the dialog instead of calling `window.prompt`. Do the email-match check inside the dialog's submit handler, so wrong input keeps the dialog open with an inline error rather than closing silently.
3. Wrap all user-facing strings on the page with `useTranslation` + `t(...)`. Add keys under `privacy.deletion.*`, `privacy.export.*`, `privacy.error.*` in `messages/en.json`; mirror to `sk/cs/de/hu/pl`.
4. Keep the success toast contract (`setSuccess(t('privacy.deletion.success'))`) but move it into the dialog's success path so the dialog can close before the toast reads (screen-reader ordering).
5. Add regression test — mock `RequestDataDeletion` mutation, open dialog, submit wrong email → assert dialog stays open with inline error; submit right email → assert mutation fired and dialog closed. Include a `window.prompt` spy assertion (`expect(promptSpy).not.toHaveBeenCalled()`).
6. Sweep any nearby `console.error` calls left in error handlers (`code-review-ppt-web-ui-compliance-console-error-in-onerror-handlers` covers compliance-page ones — Privacy has parallel calls at L107, L128; leave them for that item unless the fix is truly one-line here).
7. `pnpm -F @ppt/web check && pnpm -F @ppt/web test -- PrivacySettings`.

## Alternatives considered
- **Replace `window.prompt` with `window.confirm` after showing the email inline** — rejected because it still ships a native dialog that's a11y-hostile and browser-suppressible; solves only the email-typing UX, not the silent-drop.
- **Add a page-level Sentry breadcrumb when `window.prompt` returns null so the drop is at least observed** — rejected because it doesn't fix the user-visible defect and telegraphs to prod ops that the GDPR flow is fragile without repairing it.

## Root-cause trace
1. Symptom: GDPR account deletion silently drops when the browser suppresses further prompts; also entire page ships English copy in all locales
2. ← Immediate cause at `PrivacySettingsPage.tsx:120` — `handleRequestDeletion` calls `window.prompt(...)` and treats `null` return as user-cancel with no distinction between "user cancelled" and "browser blocked the dialog"
3. ← Upstream cause: initial Privacy scaffolding was written before the shared `ConfirmationDialog` pattern existed and was not migrated when the compliance surface swept its own prompts (PR #2829)
4. Origin: initial PrivacySettingsPage.tsx scaffolding — a search of `git log --oneline -- frontend/apps/ppt-web/src/features/privacy/pages/PrivacySettingsPage.tsx` should show the file has not been substantively edited since introduction; the implementer should include the commit sha in the final PR body

## Test plan
- [x] `PrivacySettingsPage.test.tsx` — regression scenario: open dialog, submit wrong-email → dialog stays open + inline error; submit right-email → `RequestDataDeletion` mutation fires with `{confirmation: <email>}`, dialog closes
- [x] `PrivacySettingsPage.test.tsx` — regression assertion: `jest.spyOn(window, 'prompt')` is never called from `handleRequestDeletion`
- [x] `PrivacySettingsPage.i18n.snapshot.test.tsx` — snapshot of the deletion dialog in each of 6 locales; asserts translation keys are wired (mirror the `VerificationBadge.i18n.snapshot.test.tsx` pattern from #2850)
- [x] `pnpm -F @ppt/web test -- --filter PrivacySettings` and `pnpm -F @ppt/web check`

## Out of scope
- Backend `RequestDataDeletion` contract — reuse as-is (accepts `{confirmation}` matching the account email)
- The data-export flow on the same page — only wrap its strings in `t(...)`, don't restructure
- Reworking the 30-day grace period language / policy copy — that's a legal/UX ask, not a bug fix
- The privacy page's `console.error` calls — will be swept by `code-review-ppt-web-ui-compliance-console-error-in-onerror-handlers` if it lands first

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-ui-privacy-deletion-window-prompt-gdpr.md`
- Mark the matching `backlog.json` row as `status: "done"`
