---
id: ppt/financial
name: Financial Dashboard
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/financial"
    component: FinancialDashboardPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: partial
endpoints: []
relatedScreens: []
sharedComponents: []
diagrams: []
useCases: []
epics: []
designSources: []
owner: pm-frontend
---

# Financial Dashboard

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

## Notes

### Specific (recent)
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:522`.

## Agent Log
- 2026-05-18 — agent: created stub for unmapped route.
- 2026-06-03 — agent: confirmed FinancialDashboardPageRoute wired to getARAgingReport/getOverdueInvoices/listInvoices via TanStack Query (#975.1); apiStatus -> partial (no org-wide payments endpoint).
