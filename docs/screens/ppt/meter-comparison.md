---
id: ppt/meter-comparison
name: Reading Comparison
product: ppt
implementations:
  ppt-web:
    route: "/meters/comparison"
    component: ReadingComparisonPage
    buildStatus: in-progress
    redesignStatus: not-started
    apiStatus: complete
endpoints: []
relatedScreens: []
sharedComponents: []
diagrams: []
useCases: []
epics:
  - Epic-12
designSources: []
owner: pm-frontend
---

# Reading Comparison

Wired into ppt-web `AppRoutes.tsx` by PAP-20 (Epic 12). The feature dir was
already built but unrouted; this screen is now reachable on web.

## States

- **Empty**: rendered today — the `@ppt/api-client` Epic module is not yet
  generated (client-lag), so the route mounts with empty data.
- **Loading**: n/a until the api-client lands.
- **Error**: n/a until the api-client lands.

## Notes

### Specific (recent)
- 2026-06-08 — PAP-20: mounted route group; renders against the dev stack with
  stub/empty data pending the meters+leases api-client module.

## Agent Log
- 2026-06-08 — CTO: created on route mount (PAP-20, Epic 12).
- 2026-11-04 — agent: Compare now fetches `GET /meters/{id}/consumption?from&to` per selected meter and charts per-period consumption (overlay / side by side); empty range shows a no-data message, failures toast. Meters api-client now sends the bearer token.
- 2026-11-04 — agent: Reading count pluralised (`meters.readings_*`).
