---
id: ppt/dashboard-resident
name: Resident Dashboard
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/dashboard/resident"
    component: ResidentDashboardPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: stub
endpoints: []
relatedScreens:
  - id: ppt/dashboard-manager
    rel: sibling
sharedComponents: []
diagrams: []
useCases: []
epics:
  - Epic-3
designSources: []
owner: pm-frontend
---

# Resident Dashboard

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

Note: existing `ppt/dashboard` screen-map is mobile-only — this is the web-only resident view.

## Notes

### Specific (recent)
- 2026-10-05 — `/dashboard/resident` is now the bare-`/dashboard` fan-out target for non-manager roles (PR #3002, closes #2998). The bare `/dashboard` redirect became role-aware via `DashboardIndexRedirect` (wrapped in an auth-only `<ProtectedRoute>`): managers → `/dashboard/manager`, everyone else → here. Residents typing the obvious `/dashboard` URL reach this screen instead of being caught by the now role-gated manager shell. Routing change only — `buildStatus`/`apiStatus` unchanged.
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:395`.

## Agent Log
- 2026-10-05 — agent: reconciled screen-map with PR #3002 — recorded that bare `/dashboard` now fans out non-manager roles to `/dashboard/resident`; routing only, no build/api status change.
- 2026-05-18 — agent: created stub for unmapped route.
