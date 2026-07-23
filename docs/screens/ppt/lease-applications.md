---
id: ppt/lease-applications
name: Lease Applications
product: ppt
implementations:
  ppt-web:
    route: "/leases/applications"
    component: ApplicationsPage
    buildStatus: in-progress
    redesignStatus: not-started
    apiStatus: complete
endpoints: []
relatedScreens: []
sharedComponents: []
diagrams: []
useCases:
  - UC-33
epics:
  - Epic-19
designSources: []
owner: pm-frontend
---

# Lease Applications

Wired into ppt-web `AppRoutes.tsx` by PAP-20 (Epic 19). The feature dir was
already built but unrouted; this screen is now reachable on web.

## States

- **Empty**: rendered today — the `@ppt/api-client` Epic module is not yet
  generated (client-lag), so the route mounts with empty data.
- **Loading**: n/a until the api-client lands.
- **Error**: n/a until the api-client lands.

## Notes

### Specific (recent)
- 2026-07-09 — Linked UC-33 (Tenant Screening). The applications list is where
  a screener compares multiple tenant applications side by side (UC-33.10)
  before drilling into a single application for the full screening flow on
  `ppt/lease-application-detail`.
- 2026-06-08 — PAP-20: mounted route group; renders against the dev stack with
  stub/empty data pending the meters+leases api-client module.

## Agent Log
- 2026-07-09 — agent: linked UC-33 (Tenant Screening) to useCases frontmatter — list-level compare (UC-33.10) belongs here; detail screening on lease-application-detail.
- 2026-06-08 — CTO: created on route mount (PAP-20, Epic 19).
