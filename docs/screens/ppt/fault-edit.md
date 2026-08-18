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
- 2026-08-18 — agent: screen-map-drift-pr-2647 — noted i18n update in PR #2647: `EditFaultPageRoute` in `frontend/apps/ppt-web/src/routes/groups/faults.tsx` now renders its missing-param fallback via `t('errors.faultNotFound', 'Fault not found')` (was hardcoded English); `useTranslation()` hook added before the early return to keep hook order stable; `errors.faultNotFound` key added to all locale bundles. No route or component change.
