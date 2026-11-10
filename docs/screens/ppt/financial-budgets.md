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
    apiStatus: partial
endpoints: []
relatedScreens:
  - id: ppt/financial
    rel: parent
sharedComponents: []
diagrams: []
useCases:
  - UC-40
epics:
  - Epic-11
designSources: []
owner: pm-frontend
---

# Budget Management

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

## Notes

### Specific (recent)
- 2026-11-04 — budgets wired end to end. New `@ppt/api-client` `budgets` module wraps
  `/api/v1/budgets` (list/get/create/update/delete, submit/approve/activate/close,
  summary, variance, items, item actuals, categories). The list route loads
  `listBudgets({ organizationId, fiscalYear, buildingId })` plus one
  `getBudgetSummary` per budget for actual spend ("Remaining" = budgeted − actual).
  `/financial/budgets/new` (BudgetForm: name, fiscal year, optional building, notes) and
  `/financial/budgets/:budgetId` (BudgetDetail: lines, totals, add/remove lines in draft
  incl. inline category creation, record actuals when active, lifecycle actions, delete
  draft) are registered manager-only in `routes/groups/financial-detail.tsx`. Strings
  live under `financial.budgetPages.*` in all six locales. `endpoints` stays empty: the
  budget operations are not in the generated OpenAPI / `@ppt/sitemap`, so listing them
  would fail validation. Not surfaced (backend exists, no UI yet): budget rename/notes
  edit, item edit, variance alerts + acknowledge, budget dashboard, capital plans,
  forecasts. apiStatus is `partial` for that reason.
- 2026-11-04 — create / detail navigation hidden. `/financial/budgets/new` and
  `/financial/budgets/:id` are not routed and `@ppt/api-client` has no budgets client
  (the api-server does expose `/api/v1/budgets`), so `onNavigateToCreate` /
  `onNavigateToDetail` are now optional on `BudgetManagementPage` and the route omits
  them — the "Create budget" buttons no longer lead to not-found. Wiring a budgets
  client + create/detail screens remains open.
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:534`.
- 2026-07-09 — linked UC-40 (Budget & Planning) to this screen; the use case covers annual budgets, budget-vs-actual, capex planning, budget approval voting, reporting, forecasting, reserve-fund management, and budget history — all served by the Budget Management page.

## Agent Log
- 2026-11-05 — agent: screen-map-drift-pr-3242-ppt — reconcile i18n drift from PR #3242 (`fix(ppt-web): localize financial, buildings and my-unit pages`, merged 2026-11-04). `BudgetManagementPage` + `BudgetOverview`/`BudgetCard`/`BudgetForm` moved their remaining hardcoded English into `financial.*` i18n keys across en/sk/cs/de/pl/hu; budget-status enum maps to keys with a raw-value fallback; money/dates format via `i18n.language`. No frontmatter change — docs-only.
- 2026-05-18 — agent: created stub for unmapped route.
- 2026-07-09 — agent: linked UC-40 to useCases frontmatter (gap-screens-link-uc-40).
- 2026-11-04 — agent: hid budget create/detail buttons (no route, no budgets api-client).
- 2026-11-04 — agent: added budgets api-client; wired list, create (/financial/budgets/new) and detail (/financial/budgets/:budgetId); apiStatus stub → partial.
