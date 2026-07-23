---
id: ppt/fault-detail
name: Fault Detail
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/faults/:faultId"
    component: FaultDetailPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: complete
endpoints: []
relatedScreens:
  - id: ppt/faults-list
    rel: parent
sharedComponents: []
diagrams: []
useCases: []
epics:
  - Epic-4
designSources: []
owner: pm-frontend
---

# Fault Detail

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

## Notes

### Specific (recent)
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:499`.

## Agent Log
- 2026-05-18 — agent: created stub for unmapped route.
- 2026-06-03 — agent: confirmed FaultDetailPageRoute wired to useFault + triage/resolve/confirm/reopen/comment/attachment hooks (#970.1); apiStatus -> complete.
