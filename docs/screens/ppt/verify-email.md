---
id: ppt/verify-email
name: Verify Email
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/verify-email"
    component: VerifyEmailPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: complete
endpoints: []
relatedScreens: []
sharedComponents: []
diagrams: []
useCases: []
epics:
  - Epic-1
designSources: []
owner: pm-frontend
---

# Verify Email

Landing page for the e-mailed verification link. Calls GET /api/v1/auth/verify-email once (single-use token) and offers POST /api/v1/auth/resend-verification on failure.

## Notes

### Specific (recent)
- 2026-11-06 — added in gap audit round 6 (P0).

## Agent Log
- 2026-11-06 — agent: created with the page.
