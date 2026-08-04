---
id: ppt/financial-payments
name: Payment Management
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/financial/payments"
    component: PaymentManagementPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: partial
endpoints: []
relatedScreens:
  - id: ppt/financial
    rel: parent
sharedComponents: []
diagrams: []
useCases: []
epics:
  - Epic-11
designSources: []
owner: pm-frontend
---

# Payment Management

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

## Notes

### Specific (recent)
- 2026-08-04 — **money-movement mutations now surface failures via error toast, PR #2649.**
  The two reconciliation mutations in `PaymentManagementPageRoute`
  (`groups/financial.tsx`) — `matchMutation` (`allocatePayment`, the "Match"
  action) and `autoMatchMutation` (`autoMatchPayments`, "Auto-Match All") —
  previously had `onSuccess` (invalidate payments/invoices) but **no `onError`**,
  so a rejected reconciliation call produced zero user feedback (the click looked
  like a no-op). Both mutations now wire `onError` handlers that fire an `error`
  `useToast()` — titles `financial.payments.matchFailed` ("Failed to allocate
  payment") / `financial.payments.autoMatchFailed` ("Failed to auto-match
  payments") via the established `t(key, { defaultValue })` fallback (same UX as
  the sibling `sendInvoice`/`downloadInvoicePdf` toasts in
  `InvoiceManagementPageRoute`). Handlers only raise a toast — no query/form
  state is touched, so pending reconciliation input is preserved. Regression
  coverage: `frontend/apps/ppt-web/src/routes/groups/financial.payments-onerror.route.test.tsx`
  (mounts the production `/financial/payments` route, drives "Auto-Match All"
  against a rejecting `autoMatchPayments`, asserts the error toast). This is a
  failure-feedback UX addition — `buildStatus`/`apiStatus` unchanged.
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:530`.

## Agent Log
- 2026-08-04 — agent: screen-map-drift-pr-2649-ppt — reconciled this screen-map
  with PR #2649 (surface rentals/financial mutation failures via error toast).
  Documented the new `onError` error-toast wiring on the `allocatePayment` /
  `autoMatchPayments` money-movement mutations under Notes > Specific. Note: the
  merged #2649 diff only touched `groups/financial.tsx` (+ its route test) — the
  rentals `onError` handling referenced in the PR body is the #2648 auth-guard,
  already tracked on `ppt/rentals-dashboard`, so no rentals screen-map change was
  warranted. Docs-only reconcile; no frontmatter outcome changed.
- 2026-05-18 — agent: created stub for unmapped route.
- 2026-06-22 — agent: confirmed PaymentManagementPageRoute wired (#975.5) to listPayments + listUnallocatedPayments + listInvoices via TanStack Query, with allocatePayment (onMatch) and autoMatchPayments (onAutoMatch) mutations; apiStatus stub -> partial (no building_id filter param; buildings stays []).
