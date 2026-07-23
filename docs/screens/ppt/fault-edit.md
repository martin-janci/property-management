---
id: ppt/fault-edit
name: Edit Fault
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/faults/:faultId/edit"
    component: EditFaultPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: complete
endpoints: []
relatedScreens:
  - id: ppt/fault-detail
    rel: parent
sharedComponents: []
diagrams: []
useCases: []
epics:
  - Epic-4
designSources: []
owner: pm-frontend
---

# Edit Fault

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

## Notes

### Specific (recent)
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:501`.

## Agent Log
- 2026-05-18 — agent: created stub for unmapped route.
- 2026-06-02 — agent (gap-sweep): wired EditFaultPageRoute to `useFault` (load initial data) + `useUpdateFault` + `useBuildings` in App.tsx (was initialData={{}} + no-op toast). apiStatus stub→complete.
