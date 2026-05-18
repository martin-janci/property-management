---
id: ppt/forbidden
name: Forbidden (403)
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/forbidden"
    component: ForbiddenPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: n/a
endpoints: []
relatedScreens:
  - id: ppt/server-error
    rel: sibling
  - id: ppt/session-expired
    rel: sibling
sharedComponents: []
diagrams: []
useCases: []
epics: []
designSources: []
owner: pm-frontend
---

# Forbidden (403)

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

Note: chosen one-per-screen (over a combined error-pages.md) to match the existing convention in `docs/screens/_template.md`, which prefers a single screen per file.

## Notes

### Specific (recent)
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:542`.

## Agent Log
- 2026-05-18 — agent: created stub for unmapped route.
