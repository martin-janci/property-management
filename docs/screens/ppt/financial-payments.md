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
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:530`.

## Agent Log
- 2026-05-18 — agent: created stub for unmapped route.
- 2026-06-22 — agent: confirmed PaymentManagementPageRoute wired (#975.5) to listPayments + listUnallocatedPayments + listInvoices via TanStack Query, with allocatePayment (onMatch) and autoMatchPayments (onAutoMatch) mutations; apiStatus stub -> partial (no building_id filter param; buildings stays []).
