---
id: ppt/leases-list
name: Leases List
product: ppt
implementations:
  ppt-web:
    route: "/leases/list"
    component: LeasesListPage
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

# Leases List

Wired into ppt-web `AppRoutes.tsx` by PAP-20 (Epic 19). The feature dir was
already built but unrouted; this screen is now reachable on web.

## States

- **Empty**: rendered today — the `@ppt/api-client` Epic module is not yet
  generated (client-lag), so the route mounts with empty data.
- **Loading**: spinner while the leases query is in flight.
- **Error**: when the leases query fails, the list renders a `role="alert"`
  error panel (`leases.list.failedToLoad`) with a Retry button that refetches —
  the sibling-list pattern (faults / announcements: `isError` + `onRetry`).

## Notes

### Specific (recent)
- 2026-06-08 — PAP-20: mounted route group; renders against the dev stack with
  stub/empty data pending the meters+leases api-client module.

## Agent Log
- 2026-06-08 — CTO: created on route mount (PAP-20, Epic 19).
- 2026-11-04 — agent: Filters wired (were `onFilterChange={() => {}}`): status + expiry window + page persist in the URL (`?status=&expiring=&page=`) and map to `GET /leases` `status` / `expiring_within_days` / `limit=12` / `offset` (`pending_signatures`→`pending_signature`; synthetic `expiring_soon`→`active` + 90 days). Search and building filter hidden — the list endpoint supports neither. Card Renew/Terminate deep-link to `/leases/{id}?action=…`.
- 2026-11-12 — agent: Surfaced the swallowed query error. `LeasesListPageRoute` ignored `useLeases` `isError`/`refetch`, so a failed query fell through to the empty state. Route now passes `isError` + `onRetry` (refetch) and `LeasesListPage` renders the standard `role="alert"` error panel (`leases.list.failedToLoad`, en/sk/cs) with Retry, guarding the empty/grid states with `!isError`. Regression test: `leases.list-error.route.test.tsx`.
