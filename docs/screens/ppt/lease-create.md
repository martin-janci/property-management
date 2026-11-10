---
id: ppt/lease-create
name: Create Lease
product: ppt
implementations:
  ppt-web:
    route: "/leases/new"
    component: CreateLeasePage
    buildStatus: in-progress
    redesignStatus: not-started
    apiStatus: complete
endpoints: []
relatedScreens: []
sharedComponents: []
diagrams: []
useCases: []
epics:
  - Epic-19
designSources: []
owner: pm-frontend
---

# Create Lease

Wired into ppt-web `AppRoutes.tsx` by PAP-20 (Epic 19). The feature dir was
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
- 2026-06-08 — CTO: created on route mount (PAP-20, Epic 19).
- 2026-11-04 — agent: unit picker now lists units of every org building (fan-out `GET /buildings/{id}/units`), tenant picker lists org members (`GET /organizations/{id}/members`) minus manager roles; submit sends tenant name/e-mail + landlord (org) name, only active templates offered.
- 2026-11-04 — agent: `leases` namespace fully translated into sk/cs/de/pl/hu (was ~95% English fallback).
