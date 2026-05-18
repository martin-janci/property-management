---
id: reality/realtor-detail
name: Realtor Detail (Public)
product: reality
sitemapRefs: {}
implementations:
  reality-web:
    route: "/realtor/[id]"
    component: RealtorDetailPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: stub
endpoints: []
relatedScreens:
  - id: reality/realtor
    rel: parent
sharedComponents: []
diagrams: []
useCases: []
epics: []
designSources: []
owner: reality-frontend
---

# Realtor Detail (Public)

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

Note: differs from `reality/agent-profile` (the realtor's own profile view) — this is a public-facing realtor profile page.

## Notes

### Specific (recent)
- 2026-05-18 — audit: stub created from `frontend/apps/reality-web/src/app/[locale]/realtor/[id]/page.tsx`.

## Agent Log
- 2026-05-18 — agent: created stub for unmapped route.
