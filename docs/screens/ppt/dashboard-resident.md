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
epics: []
designSources: []
owner: pm-frontend
---

# Resident Dashboard

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

Note: existing `ppt/dashboard` screen-map is mobile-only — this is the web-only resident view.

## Notes

### Specific (recent)
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:395`.

## Agent Log
- 2026-07-19 — agent: page now renders via resolved-layout section registry (defensive rendering, spec 2026-07-19-layout-content-manager-design)
- 2026-05-18 — agent: created stub for unmapped route.
