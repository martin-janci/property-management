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
- 2026-10-07 — correction: an earlier note on this screen claimed the route had "no role gate". That is wrong on `origin/dev`. The `/dashboard/manager` route is role-gated to `MANAGER_ROLES` via `<ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>` in `dashboardRoutes()` (`frontend/apps/ppt-web/src/routes/groups/core.tsx`), landed in PR #2998. A resident navigating here directly gets the `ProtectedRoute` Access-Denied surface, not the manager shell. Residents are not wrongly denied because bare `/dashboard` fans out by role via `DashboardIndexRedirect` (`isManagerRole`) — managers to `/dashboard/manager`, everyone else to `/dashboard/resident` — the same single source of truth as the Home "Open dashboard" CTA. The note below is retained for history but superseded by this one.
- 2026-10-05 — `/dashboard/manager` was wrapped in `<ProtectedRoute>` inside `dashboardRoutes()` in `frontend/apps/ppt-web/src/routes/groups/core.tsx` (PR #2994). `ManagerDashboardPage` does not self-guard (only reads `user?.role` for a "customize" link), so unauthenticated hits redirect to `/login`; previously the dashboard shell rendered for anonymous users. (Subsequently, PR #2998 tightened this route to a `MANAGER_ROLES` role gate — see the 2026-10-07 correction above.) Route path / component / API contract unchanged.
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:392`.

## Agent Log

<!-- newest entries on top -->

- 2026-10-07 — agent: corrected a false "no role gate" note. On `origin/dev` the `/dashboard/manager` route is role-gated to `MANAGER_ROLES` (`<ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>`), landed in PR #2998. Fixed the Specific (recent) note accordingly; frontmatter and the resident doc are already correct and were left unchanged. Docs-only.
- 2026-10-05 — agent: reconciled with PR #2994 (ppt-web dashboard route auth-guard). Recorded `<ProtectedRoute>` wrapper added to `/dashboard/manager`. No frontmatter change — buildStatus (shipped), apiStatus (stub), route and component are unaffected by an auth-only wrapper.
- 2026-05-18 — agent: created stub for unmapped route.
