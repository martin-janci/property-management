---
id: ppt/lease-violation-detail
name: Violation Detail
product: ppt
implementations:
  ppt-web:
    route: "/leases/violations/:violationId"
    component: ViolationDetailPage
    buildStatus: in-progress
    redesignStatus: not-started
    apiStatus: partial
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

# Violation Detail

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
- 2026-08-18 — agent: screen-map-drift-pr-2647 — noted i18n update in PR #2647: the shared `LeaseLoadingNotice` rendered by `ViolationDetailPageRoute` in `frontend/apps/ppt-web/src/routes/groups/leases.tsx` now renders its missing-entity fallback via `t('errors.entityNotFound', { entity: 'Violation', defaultValue: '{{entity}} not found' })` (was a hardcoded English template literal); `errors.entityNotFound` key added to all locale bundles. No route or component change.
