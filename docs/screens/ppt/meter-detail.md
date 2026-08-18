---
id: ppt/meter-detail
name: Meter Detail
product: ppt
implementations:
  ppt-web:
    route: "/meters/:meterId"
    component: MeterDetailPage
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

# Meter Detail

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
- 2026-08-18 — agent: screen-map-drift-pr-2647 — noted i18n update in PR #2647: the shared `MeterLoadingNotice` fallback in `frontend/apps/ppt-web/src/routes/groups/meters.tsx` (rendered by the meter detail / edit-reading / submit-reading route wrappers when the meter is absent) now renders via `t('errors.meterNotFound', 'Meter not found')` (was hardcoded English); `useTranslation()` hook added inside the notice component; `errors.meterNotFound` key added to all locale bundles. No route or component change.
