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
- 2026-11-26 — the five metric StatCards (total/active listings, views, inquiries, conversion rate) now render via the locale-aware `formatNumber(value, locale)` helper from `@/lib/format` (PR #3740); conversion rate uses `style: 'percent'`. A `page.i18n.test.tsx` regression renders under the `de` locale and asserts locale-correct grouping (`12.345`) and percent (`12,3 %`). No API/data change — all-time counters still come from `GET /api/v1/realtors/profile`.
- 2026-05-18 — audit: stub created from `frontend/apps/reality-web/src/app/[locale]/realtor/analytics/page.tsx`.

## Agent Log
- 2026-11-26 — agent: reconcile screen-map drift from PR #3740 (localize realtor analytics metric formatting) — metrics now route through `formatNumber` with the active `useLocale()`; added `page.i18n.test.tsx` regression. buildStatus/apiStatus unchanged (shipped/stub); presentation-only i18n fix, no endpoint change.
- 2026-11-10 — agent: audit round 8 — stats now come from the all-time counters on `GET /api/v1/realtors/profile` (the page called a non-existent `/realtors/me/stats`); period selector removed because the server has no per-period stats.
- 2026-05-18 — agent: created stub for unmapped route.
