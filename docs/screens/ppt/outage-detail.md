---
id: ppt/outage-detail
name: Outage Detail
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/outages/:outageId"
    component: ViewOutagePage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: stub
endpoints: []
relatedScreens:
  - id: ppt/outages
    rel: parent
sharedComponents: []
diagrams: []
useCases: []
epics: []
designSources: []
owner: pm-frontend
---

# Outage Detail

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

## Notes

### Specific (recent)
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:468`.

## Agent Log
- 2026-05-18 — agent: created stub for unmapped route.
