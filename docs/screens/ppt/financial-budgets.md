---
id: ppt/financial-budgets
name: Budget Management
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/financial/budgets"
    component: BudgetManagementPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: stub
endpoints: []
relatedScreens:
  - id: ppt/financial
    rel: parent
sharedComponents: []
diagrams: []
useCases:
  - UC-40
epics: []
designSources: []
owner: pm-frontend
---

# Budget Management

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

## Notes

### Specific (recent)
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:534`.
- 2026-07-09 — linked UC-40 (Budget & Planning) to this screen; the use case covers annual budgets, budget-vs-actual, capex planning, budget approval voting, reporting, forecasting, reserve-fund management, and budget history — all served by the Budget Management page.

## Agent Log
- 2026-05-18 — agent: created stub for unmapped route.
- 2026-07-09 — agent: linked UC-40 to useCases frontmatter (gap-screens-link-uc-40).
