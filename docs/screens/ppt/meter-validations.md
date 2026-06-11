---
id: ppt/meter-validations
name: Pending Reading Validations
product: ppt
implementations:
  ppt-web:
    route: "/meters/validations"
    component: PendingValidationsPage
    buildStatus: in-progress
    redesignStatus: not-started
    apiStatus: complete
endpoints: []
relatedScreens: []
sharedComponents: []
diagrams: []
useCases: []
epics: []
designSources: []
owner: pm-frontend
---

# Pending Reading Validations

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
