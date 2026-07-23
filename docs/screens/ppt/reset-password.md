---
id: ppt/reset-password
name: Reset Password
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/reset-password"
    component: ResetPasswordPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: stub
endpoints: []
relatedScreens:
  - id: ppt/forgot-password
    rel: sibling
sharedComponents: []
diagrams: []
useCases: []
epics:
  - Epic-1
designSources: []
owner: pm-frontend
---

# Reset Password

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

## Notes

### Specific (recent)
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:374`.

## Agent Log
- 2026-05-18 — agent: created stub for unmapped route.
