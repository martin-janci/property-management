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
relatedScreens:
  - id: ppt/financial-reports
    rel: child
sharedComponents: []
diagrams: []
useCases: []
epics:
  - Epic-11
designSources: []
owner: pm-frontend
---

# Financial Dashboard

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

## Notes

### Specific (recent)
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:522`.
- 2026-08-31 — realtime sync (PR #2889): the `financial` query root now auto-refetches on a `notification.created` frame with `category: financial`. Previously `WebSocketContext.eventToQueryKeys` keyed on dead `entity:*` names the api-server never emits (100% dead sync); PR #2889 added `categoryToQueryKeys.financial → ['financial']` and wired `App.tsx`'s `onEntityEvent`. REST wiring unchanged (apiStatus stays partial — no org-wide payments endpoint).
- 2026-11-05 — i18n (PR #3242): the dashboard, its filters and all financial sub-views (invoices, payments, budgets, reports) are fully localized via `financial.*` keys (sk/cs/de/pl/hu, plurals one/few/many/other for sk/cs/pl); money/dates respect `i18n.language` with EUR kept; enum statuses render via keys with a raw-value fallback.

## Agent Log

- 2026-11-05 — agent: screen-map-drift-pr-3242-ppt — reconcile i18n drift from PR #3242 (`fix(ppt-web): localize financial, buildings and my-unit pages`, merged 2026-11-04). `FinancialDashboardPage` and its financial components moved hardcoded English into `financial.*` i18n keys across en/sk/cs/de/pl/hu; money and dates now format via `i18n.language` + Intl (currency stays EUR); invoice/payment/budget status + payment-method enums map to keys with a raw-value fallback; `/financial/payments` date filters use real `startDate`/`endDate` keys (were raw `common.startDate`/`common.endDate`); added aria-labels to icon-only and unlabeled filter controls. No route/component/buildStatus/apiStatus change — docs-only.
- 2026-11-04 — agent: r3-nav-roles follow-up — quick-nav shows Invoices, Budgets and Reports only to `isManagerRole` users; residents see Payments only (audit r3 #14).
- 2026-08-31 — agent: screen-map-drift-pr-2889-ppt — reconcile drift from PR #2889 (realtime ws→query-invalidation fix). ppt-web `WebSocketContext` re-keyed cache invalidation to canonical `domain.action` events and its `notification.created` subscriber routes by `payload.category`, so `category=financial` now invalidates the `financial` root; `App.tsx` wires `onEntityEvent`. No route/component/endpoint/status change — frontmatter unchanged; docs-only.
- 2026-05-18 — agent: created stub for unmapped route.
- 2026-06-03 — agent: confirmed FinancialDashboardPageRoute wired to getARAgingReport/getOverdueInvoices/listInvoices via TanStack Query (#975.1); apiStatus -> partial (no org-wide payments endpoint).

## Agent Log

<!-- newest entries on top -->

- 2026-11-04 — agent: added quick-nav cards to invoices/payments/budgets/reports; recent payments wired to listPayments(limit 5); overdue View → /financial/invoices/:id; building/date filters hidden (no API filter params)
