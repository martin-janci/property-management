---
id: ppt/fault-new
name: Create Fault
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/faults/new"
    component: CreateFaultPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: stub
endpoints: []
relatedScreens:
  - id: ppt/faults-list
    rel: parent
sharedComponents: []
diagrams: []
useCases: []
epics: []
designSources: []
owner: pm-frontend
---

# Create Fault

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

Note: a `ppt/report-fault` screen-map already exists (mobile-oriented). This stub covers the web `/faults/new` route specifically.

## Notes

### Specific (recent)
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:498`.

## Agent Log
- 2026-05-18 — agent: created stub for unmapped route.
