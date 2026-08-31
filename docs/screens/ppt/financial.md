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

## Agent Log
- 2026-08-31 — agent: screen-map-drift-pr-2889-ppt — reconcile drift from PR #2889 (realtime ws→query-invalidation fix). ppt-web `WebSocketContext` re-keyed cache invalidation to canonical `domain.action` events and its `notification.created` subscriber routes by `payload.category`, so `category=financial` now invalidates the `financial` root; `App.tsx` wires `onEntityEvent`. No route/component/endpoint/status change — frontmatter unchanged; docs-only.
- 2026-05-18 — agent: created stub for unmapped route.
- 2026-06-03 — agent: confirmed FinancialDashboardPageRoute wired to getARAgingReport/getOverdueInvoices/listInvoices via TanStack Query (#975.1); apiStatus -> partial (no org-wide payments endpoint).
