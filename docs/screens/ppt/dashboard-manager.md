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
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:392`.

## Agent Log
- 2026-05-18 — agent: created stub for unmapped route.
