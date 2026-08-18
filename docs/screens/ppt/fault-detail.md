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
- 2026-08-18 — agent: screen-map-drift-pr-2647 — noted i18n update in PR #2647: `FaultDetailPageRoute` in `frontend/apps/ppt-web/src/routes/groups/faults.tsx` now renders its missing-param fallback via `t('errors.faultNotFound', 'Fault not found')` (was hardcoded English); `errors.faultNotFound` key added to all locale bundles. No route or component change.
