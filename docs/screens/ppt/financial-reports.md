---
id: ppt/financial-reports
name: Financial Reports
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/financial/reports"
    component: FinancialReportsPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: wired
endpoints:
  - "GET /api/v1/financial/reports/income-statement"
  - "GET /api/v1/financial/reports/balance-sheet"
  - "GET /api/v1/financial/reports/cash-flow"
  - "GET /api/v1/financial/reports/{report}/export"
relatedScreens:
  - id: ppt/financial
    rel: parent
sharedComponents: []
diagrams: []
useCases: []
epics:
  - "Epic 11 / Story 11.7 — Financial statement reports"
designSources: []
owner: pm-frontend
---

# Financial Reports

Tabbed financial statement viewer: Income Statement, Balance Sheet, and Cash Flow.
Income statement and cash flow use a from/to date range (default year-to-date);
the balance sheet uses an as-of date (default today). Each report renders as a
card/table and offers PDF + Excel (xlsx) export, which streams the backend blob
to a browser download.

## Notes

### Specific (recent)
- 2026-06-22 — BIT-222: page + IncomeStatementView / BalanceSheetView / CashFlowView
  components created; route `/financial/reports` wired in `routes/groups/financial.tsx`
  via TanStack Query (one query per active tab) against the Story 11.7 endpoints.

## Agent Log
- 2026-06-22 — FrontendEngineer: created screen + screen-map for Story 11.7 FE
  follow-up (BIT-222). Backend endpoints live on dev (PRs #1717 + #1723).
