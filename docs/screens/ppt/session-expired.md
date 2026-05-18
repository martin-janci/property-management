---
id: ppt/session-expired
name: Session Expired
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/session-expired"
    component: SessionExpiredPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: n/a
endpoints: []
relatedScreens:
  - id: ppt/forbidden
    rel: sibling
  - id: ppt/server-error
    rel: sibling
  - id: ppt/login
    rel: child
sharedComponents: []
diagrams: []
useCases: []
epics: []
designSources: []
owner: pm-frontend
---

# Session Expired

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

## Notes

### Specific (recent)
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:544`.

## Agent Log
- 2026-05-18 — agent: created stub for unmapped route.
