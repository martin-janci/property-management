---
id: ppt/financial-invoices
name: Invoice Management
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/financial/invoices"
    component: InvoiceManagementPage
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
epics: []
designSources: []
owner: pm-frontend
---

# Invoice Management

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

## Notes

### Specific (recent)
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:526`.

## Agent Log
- 2026-05-18 — agent: created stub for unmapped route.
- 2026-06-03 — agent: confirmed InvoiceManagementPageRoute wired to listInvoices (status + pagination) + sendInvoice via TanStack Query (#975.2); apiStatus -> partial (no building_id/search params).
