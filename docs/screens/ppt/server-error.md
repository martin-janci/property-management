---
id: ppt/server-error
name: Server Error (500)
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/server-error"
    component: ServerErrorPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: n/a
endpoints: []
relatedScreens:
  - id: ppt/forbidden
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

# Server Error (500)

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

## Notes

### Specific (recent)
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:543`.

## Agent Log

<!-- newest entries on top -->

- 2026-11-05 — agent: screen-map-drift-pr-3244-ppt — reconcile i18n drift from PR #3244 (`fix(ppt-web): localize faults, news, settings and error pages`). `ServerErrorPage` copy localized across all 6 locales — `errors.somethingWentWrong`, `errors.pages.serverError.description`, and actions `errors.pages.{tryAgain,backToDashboard}`. i18n only — still shipped; frontmatter unchanged.
- 2026-05-18 — agent: created stub for unmapped route.
