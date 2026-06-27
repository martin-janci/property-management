# code-review-ppt-web-core-invoice-mgmt-no-err

**Vector:** bug
**Score:** 2
**Source:** code-review-2026-06-27 (Phase 1.5 ppt-web-core)
**Confidence:** high

## Hypothesis
`AccountingInvoiceManagementPage.tsx` defines `createMutation` and `deleteMutation` that omit `onError`; when invoice create or delete fails (network, validation 4xx, server 5xx), `isPending` drops back to `false` and nothing visible changes. The manager resubmits, thinking nothing happened. The same TanStack-Query pattern that bites `PaymentMatchingPage.tsx` is present here — both pages were authored without an error UX contract. The smallest fix wires `onError` (toast + inline error) on both mutations and asserts it under test.

## Evidence
- `frontend/apps/ppt-web/src/features/accounting/pages/AccountingInvoiceManagementPage.tsx:36-49` — `createMutation` and `deleteMutation` both omit `onError`.
- `frontend/apps/ppt-web/src/features/accounting/pages/AccountingInvoiceManagementPage.tsx:74,90` — submit and delete confirmation paths trigger mutations with no `isError` UI; result is silent failure.
- Rotating code review (Phase 1.5, segment `ppt-web-core`, 2026-06-27) — frontend expert finding.

## Files
- `frontend/apps/ppt-web/src/features/accounting/pages/AccountingInvoiceManagementPage.tsx`

## Dependencies
<!-- none -->

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Mode: cloud-ok** (unit-level React Testing Library coverage is sufficient — no browser/device required)

## Repro steps
1. Open `AccountingInvoiceManagementPage` in a manager session.
2. Stub the network so `POST /v1/accounting/invoices` returns 500 (or fail validation with a 422).
3. Submit the create form.
4. Expected: an error toast and/or inline form-level error names the failure; the submit button re-enables; the form stays open with the user's input intact.
5. Actual: button re-enables silently; no error UI; user re-clicks submit thinking nothing happened.
6. Repeat with delete: select an invoice, confirm delete; stub the DELETE to 500. Expected: failure UI. Actual: row appears un-deleted with no explanation.

## Suggested approach
1. Add `onError` to both `useMutation` configs in `AccountingInvoiceManagementPage.tsx:36-49` — invoke the project's toast helper with the parsed server error message (fallback to `'Create failed — please retry'` / `'Delete failed — please retry'`).
2. Render the mutation's `error.message` inline next to the submit button (line 74) and the delete confirm button (line 90) — small red text under the action so the failure is anchored to the action, not just a transient toast.
3. If a `useMutationWithToast` wrapper already exists in the accounting feature, use it for consistency. Otherwise mirror the local `onError` pattern.
4. Add vitest coverage that fakes each mutation to reject and asserts the inline + toast error renders.

## Alternatives considered
- **Wrap the entire page in an ErrorBoundary** — rejected because TanStack mutation errors don't propagate to React ErrorBoundary; they live on the mutation object until consumed. A boundary would catch render crashes, not mutation rejections.
- **Disable buttons until the user re-loads the page on error** — rejected because it punishes the user for a transient network failure and removes their ability to retry inline. Inline error + retry is the right UX.

## Root-cause trace
1. Symptom: invoice create/delete 4xx/5xx fails silently; manager re-submits believing the click did nothing.
2. ← `AccountingInvoiceManagementPage.tsx:36-49` — `useMutation({ onSuccess })` defined without `onError`; mutation's `error` state never reaches the UI.
3. ← TanStack Query treats missing `onError` as user opt-out: it still records `isError`/`error`, but the button-site code at lines 74/90 never inspects them.
4. Origin: page authored without an error UX contract — same pattern as `PaymentMatchingPage.tsx` (covered by the sibling plan); not tied to a specific PR.

## Test plan
- [ ] `frontend/apps/ppt-web/src/features/accounting/pages/AccountingInvoiceManagementPage.test.tsx` — fail-on-main: mock the create mutation to reject; submit the form; assert the toast and inline error appear with the server-provided message.
- [ ] Regression: mock the delete mutation to reject from the confirm dialog; assert error UI renders and the invoice row stays in place.
- [ ] Happy-path: keep the existing "create succeeds and invalidates the list" assertion passing.
- [ ] `pnpm --filter @ppt/ppt-web test AccountingInvoiceManagementPage`

## Out of scope
- Reworking the invoice API contract or error-payload format.
- Adding optimistic updates — the current pessimistic flow is fine; this plan only addresses error visibility.
- Touching `PaymentMatchingPage.tsx` (covered by the sibling `code-review-ppt-web-core-payment-matching-no-err` plan).

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-invoice-mgmt-no-err.md`
- Mark the matching `backlog.json` row as `status: "done"`
