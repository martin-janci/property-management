# code-review-reality-web-inquiry-cancel-no-error

**Vector:** bug
**Score:** 4
**Source:** signals/2026-08-17-reality-web-tier1d-r1828.json + signals/2026-08-18-reality-web-tier1d.json (dispatcher tier1d, backlog id `code-review-reality-web-inquiry-cancel-no-error`)
**Confidence:** high

## Hypothesis
`InquiryCard.handleCancel` in `frontend/apps/reality-web/src/app/[locale]/inquiries/page.tsx` calls `cancelInquiry.mutate(inquiry.id)` fire-and-forget and immediately closes the confirm dialog (`setShowCancelConfirm(false)`). The `useCancelInquiry()` mutation from `@ppt/reality-api-client` has no `onError` handler, and nothing in the component reads `cancelInquiry.isError`, `cancelInquiry.error`, or `cancelInquiry.isPending`. When the cancel POST fails (network error, 409 already-cancelled by another tab, 403 permission), the confirm dialog closes, the card renders no feedback, and the inquiry silently stays pending — a user-facing false success on a state-changing action. The smallest safe change is to surface the mutation state: keep the dialog open until settled, show a spinner while pending, show an inline error message on error, and only close on `onSuccess`. The list-query path on the same page already handles `isLoading`/`error`; the mutation path is the sole gap.

## Evidence
- `frontend/apps/reality-web/src/app/[locale]/inquiries/page.tsx:41-50` (verified 2026-08-18) — `const cancelInquiry = useCancelInquiry(); const handleCancel = () => { cancelInquiry.mutate(inquiry.id); setShowCancelConfirm(false); };`
- Grep of the file (2026-08-18) — no references to `cancelInquiry.isError`, `.error`, `.isPending`, `.status`, no `onError` or `onSuccess` callback on the mutation, no toast/notification rendered on failure.
- Second-reviewer confirmation from `signals/2026-08-18-reality-web-tier1d.json` (`code-review-reality-web-inquiry-cancel-silent-fail`) — identical file, identical call sites, identical diagnosis; deduped into this canonical id and its +2 signal delta is why this row hit score 4.
- Contrast: the list query on the same page renders `isLoading` and `error` states correctly (lines around 320-340 use `if (isLoading)` / `if (error)`), so the pattern the mutation should follow already lives in-file.

## Files
- `frontend/apps/reality-web/src/app/[locale]/inquiries/page.tsx`
- `frontend/apps/reality-web/messages/en.json`
- `frontend/apps/reality-web/messages/sk.json`
- `frontend/apps/reality-web/messages/cs.json`
- `frontend/apps/reality-web/messages/de.json`

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
1. Sign in to reality-web (`pnpm dev:reality`), navigate to `/en/inquiries`, and create at least one pending inquiry.
2. In devtools, add a network throttle or a request block for `POST /api/v1/portal/inquiries/{id}/cancel` (return 500 or a network failure).
3. Open the inquiry's cancel-confirm dialog and click "Yes, cancel".
4. Observed today: dialog closes immediately, the inquiry card still renders as pending/responded with no error surface — the user believes the cancel succeeded.
5. Expected after fix: dialog stays open with a spinner while the mutation is pending; on failure the dialog shows an inline error and keeps the "Yes, cancel" button available; on success the dialog closes and the card refreshes (already handled by the list-query invalidation the mutation triggers).

## Suggested approach
1. In `InquiryCard`, replace `const handleCancel = () => { cancelInquiry.mutate(...); setShowCancelConfirm(false); }` with a mutation-aware handler:
   ```tsx
   const handleCancel = () => {
     cancelInquiry.mutate(inquiry.id, {
       onSuccess: () => setShowCancelConfirm(false),
     });
   };
   ```
2. Inside the confirm dialog block, disable the "Yes, cancel" button while `cancelInquiry.isPending` and swap its label for a translated "Cancelling…" (or a `<Spinner />`).
3. Below the button row, when `cancelInquiry.isError`, render an inline error paragraph with a translated message; `role="alert"` so screen readers announce it.
4. Add three new i18n keys to `messages/{en,sk,cs,de}.json` under the `inquiries.cancel.*` namespace: `cancelling`, `errorTitle`, `errorRetry`.
5. Add a Vitest RTL test at `frontend/apps/reality-web/src/app/[locale]/inquiries/__tests__/InquiryCard.cancel.test.tsx` that mocks `useCancelInquiry` to return an error state and asserts the dialog stays open + error message renders.
6. `pnpm -F reality-web check && pnpm -F reality-web typecheck && pnpm -F reality-web test`.

## Alternatives considered
- **Move to a global toast on error** — rejected because the failure is contextual to the specific inquiry card the user just clicked; a global toast disconnects the error from the affected row and doesn't keep the retry affordance visible. Inline error inside the still-open dialog keeps the mental model tight.
- **Only add an `onError` that re-opens the dialog after it closes** — rejected because the dialog-close-then-reopen flicker is worse UX than never-closing-until-settled, and it races the query invalidation callback that the mutation success path fires.

## Root-cause trace
1. Symptom: user clicks "Yes, cancel", dialog closes, inquiry stays pending, no error shown.
2. ← Immediate cause: `inquiries/page.tsx:47-50` `handleCancel` calls `.mutate(...)` without `onError`/`onSuccess`, then unconditionally closes the dialog.
3. ← Upstream cause: `useCancelInquiry` (generated by `@ppt/reality-api-client`) returns the standard `useMutation` shape with no baked-in error UI — the caller must surface the state. The caller pattern here treated the mutation as fire-and-forget, common in optimistic-update flows but wrong for a destructive action with no optimistic UI here.
4. Origin: the inquiries page landed with the fire-and-forget handler in place — see the initial mount of the reality-web inquiries feature (predates the tier1d discovery); no test covered the failure path.

## Test plan
- [ ] Add `frontend/apps/reality-web/src/app/[locale]/inquiries/__tests__/InquiryCard.cancel.test.tsx` — mock `useCancelInquiry` to return `{ mutate, isPending: false, isError: true, error: new Error("boom") }` and assert (a) the confirm dialog stays open, (b) an element with `role="alert"` renders the translated error, (c) the "Yes, cancel" button is still clickable for retry.
- [ ] Regression: extend the same test to cover the `isPending: true` state — assert the button is disabled and shows the "Cancelling…" label.
- [ ] Local run: `pnpm -F reality-web test -- InquiryCard.cancel`.

## Out of scope
- The broader `code-review-reality-web-locale-pages-hardcoded-i18n` refactor — this plan only wires the three new cancel-flow strings; the wider i18n gap on the page is tracked separately.
- Server-side changes to `POST /api/v1/portal/inquiries/{id}/cancel` — the contract stays the same; only the client's handling of its response changes.
- Optimistic UI for the cancel action — deliberately not adding one so the pending-state UX matches the destructive nature of the action.

## After-merge
- Move this file to `plans/_archive/code-review-reality-web-inquiry-cancel-no-error.md`
- Mark the matching `backlog.json` row as `status: "done"`
