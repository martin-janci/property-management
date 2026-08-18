---
id: ppt/lease-application-detail
name: Application Detail
product: ppt
implementations:
  ppt-web:
    route: "/leases/applications/:applicationId"
    component: ApplicationDetailPage
    buildStatus: in-progress
    redesignStatus: not-started
    apiStatus: partial
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

# Application Detail

Wired into ppt-web `AppRoutes.tsx` by PAP-20 (Epic 19). The feature dir was
already built but unrouted; this screen is now reachable on web.

## States

- **Empty**: rendered today — the `@ppt/api-client` Epic module is not yet
  generated (client-lag), so the route mounts with empty data.
- **Loading**: n/a until the api-client lands.
- **Error**: n/a until the api-client lands.

## Notes

### Specific (recent)
- 2026-07-09 — Linked UC-33 (Tenant Screening). This detail screen is the
  screening workspace for a single application: request background check
  (UC-33.1), verify income (UC-33.2), view credit score (UC-33.4), generate
  the screening report (UC-33.6), and approve/reject the application
  (UC-33.7/33.8) with GDPR-compliant consent handling (UC-33.11/33.12).
- 2026-06-08 — PAP-20: mounted route group; renders against the dev stack with
  stub/empty data pending the meters+leases api-client module.

## Agent Log
- 2026-08-18 — agent: screen-map-drift-pr-2647 — noted i18n update in PR #2647: the shared `LeaseLoadingNotice` rendered by `ApplicationDetailPageRoute` in `frontend/apps/ppt-web/src/routes/groups/leases.tsx` now renders its missing-entity fallback via `t('errors.entityNotFound', { entity: 'Application', defaultValue: '{{entity}} not found' })` (was a hardcoded English template literal); `errors.entityNotFound` key added to all locale bundles. No route or component change.
- 2026-07-09 — agent: linked UC-33 (Tenant Screening) to useCases frontmatter — screening actions live on the application-detail screen.
- 2026-06-08 — CTO: created on route mount (PAP-20, Epic 19).
