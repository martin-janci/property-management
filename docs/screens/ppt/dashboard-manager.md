---
id: ppt/dashboard-manager
name: Manager Dashboard
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/dashboard/manager"
    component: ManagerDashboardPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: stub
endpoints: []
relatedScreens:
  - id: ppt/dashboard-resident
    rel: sibling
sharedComponents: []
diagrams: []
useCases: []
epics:
  - Epic-3
designSources: []
owner: pm-frontend
---

# Manager Dashboard

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

Note: existing `ppt/dashboard` screen-map is mobile-only — this is the web-only manager view.

## Notes

### Specific (recent)
- 2026-10-05 — `/dashboard/manager` is now wrapped in `<ProtectedRoute>` (authentication-only, no role gate) inside `dashboardRoutes()` in `frontend/apps/ppt-web/src/routes/groups/core.tsx` (PR #2994). `ManagerDashboardPage` does not self-guard (only reads `user?.role` for a "customize" link), so unauthenticated hits now redirect to `/login`; previously the dashboard shell rendered for anonymous users. No role gate on the route — bare `/dashboard` redirects here, so a manager gate would wrongly deny residents; role fan-out stays on the Home page's dashboard button. Route path / component / API contract unchanged.
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:392`.

## Agent Log

<!-- newest entries on top -->

- 2026-10-05 — agent: reconciled with PR #2994 (ppt-web dashboard route auth-guard). Recorded `<ProtectedRoute>` wrapper added to `/dashboard/manager`. No frontmatter change — buildStatus (shipped), apiStatus (stub), route and component are unaffected by an auth-only wrapper.
- 2026-05-18 — agent: created stub for unmapped route.
