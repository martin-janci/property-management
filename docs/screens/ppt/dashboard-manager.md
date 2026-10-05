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
- 2026-10-05 — `/dashboard/manager` is now role-gated (PR #3002, closes #2998): wrapped in `<ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>` (single source of truth in `routes/shared.ts`, same as `isManagerRole()` and the Home "Open dashboard" CTA). An authenticated non-manager (resident/owner/tenant) now hits the `ProtectedRoute` Access-Denied surface instead of the manager shell; unauthenticated → `/login`. This tightens access on the existing route — `buildStatus`/`apiStatus` unchanged. The bare `/dashboard` redirect also became role-aware (`DashboardIndexRedirect`): managers → here, everyone else → `/dashboard/resident`.
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:392`.

## Agent Log
- 2026-10-05 — agent: reconciled screen-map with PR #3002 — `/dashboard/manager` role-gated to MANAGER_ROLES (resident lands on Access-Denied, not the manager shell); access-control only, no build/api status change.
- 2026-05-18 — agent: created stub for unmapped route.
