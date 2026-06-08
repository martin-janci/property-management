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
- 2026-06-03 — drift (PR #922, dev-review round 2): the `setReturnUrl` call into `@ppt/shared` now sanitizes via `sanitizeReturnUrl` on write — an off-origin / scheme / protocol-relative return URL captured at session-expiry is dropped rather than stored, so the eventual post-login redirect can't be hijacked.
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:544`.

## Agent Log

<!-- newest entries on top -->

- 2026-06-03 — agent: test-gap-screen-map-drift-pr-922-ppt — noted PR #922 return-URL sanitization in `@ppt/shared` `setReturnUrl` (open-redirect hardening). No frontmatter change (still shipped).
- 2026-05-24 — agent: gap-79-2 login-flow-wiring — removed local RETURN_URL_KEY constant; now imports setReturnUrl from @ppt/shared to align return-URL storage with LoginPage and prevent key drift
- 2026-05-18 — agent: created stub for unmapped route.
