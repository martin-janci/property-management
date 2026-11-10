---
id: reality/realtor-analytics
name: Realtor Analytics
product: reality
sitemapRefs: {}
implementations:
  reality-web:
    route: "/realtor/analytics"
    component: RealtorAnalyticsPage
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

# Realtor Analytics

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

## Notes

### Specific (recent)
- 2026-05-18 — audit: stub created from `frontend/apps/reality-web/src/app/[locale]/realtor/analytics/page.tsx`.

## Agent Log
- 2026-11-10 — agent: audit round 8 — stats now come from the all-time counters on `GET /api/v1/realtors/profile` (the page called a non-existent `/realtors/me/stats`); period selector removed because the server has no per-period stats.
- 2026-05-18 — agent: created stub for unmapped route.
