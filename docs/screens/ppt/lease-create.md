---
id: ppt/lease-create
name: Create Lease
product: ppt
implementations:
  ppt-web:
    route: "/leases/new"
    component: CreateLeasePage
    buildStatus: shipped
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

The `@ppt/api-client` leases/meters module has landed; the route now renders
live backend data and all three non-happy states are implemented (see
`routes/groups/leases.create-pickers.route.test.tsx`).

- **Loading**: the unit picker (`useBuildings` → per-building
  `listBuildingUnits`), tenant picker (`useOrganizationMembers`) and template
  (`useLeaseTemplates`) queries drive a combined loading state while fetching.
- **Error**: when a required picker source fails (units or tenants), the page
  shows a `role="alert"` retryable error notice ("Could not load the unit and
  tenant options" / `leases.create.failedToLoad`) with a **Retry** button that
  refetches units, members and templates. A templates-only fetch error does not
  block the form (templates are optional). The `LeaseForm` is not rendered while
  the pickers are in an error state.
- **Empty**: when the fetch succeeds but no units or no eligible tenants exist,
  the form still renders with inline amber hints under each picker
  (`leases.form.noUnits` / `leases.form.noTenants`) guiding the manager to add a
  building with units / invite residents first.

## Notes

### Specific (recent)
- 2026-11-22 — PR #3700 added the Create Lease picker loading/empty/error
  states: a retryable `role="alert"` notice when the unit or tenant query fails
  (`CreateLeasePage` `isError`/`onRetry`, backed by `useOrganizationUnits` now
  exposing `isError`/`refetch`), and inline empty-state hints under the unit and
  tenant selects in `LeaseForm`. New i18n keys `leases.create.failedToLoad`,
  `leases.form.noUnits`, `leases.form.noTenants` added to all 6 bundles
  (en/sk/cs/de/pl/hu). Regression test:
  `routes/groups/leases.create-pickers.route.test.tsx`.
- 2026-06-08 — PAP-20: mounted route group; renders against the dev stack with
  stub/empty data pending the meters+leases api-client module.

## Agent Log
- 2026-06-08 — CTO: created on route mount (PAP-20, Epic 19).
- 2026-11-04 — agent: unit picker now lists units of every org building (fan-out `GET /buildings/{id}/units`), tenant picker lists org members (`GET /organizations/{id}/members`) minus manager roles; submit sends tenant name/e-mail + landlord (org) name, only active templates offered.
- 2026-11-04 — agent: `leases` namespace fully translated into sk/cs/de/pl/hu (was ~95% English fallback).
- 2026-11-22 — agent: screen-map-drift-pr-3700-ppt — reconciled with PR #3700 (Create Lease picker loading/empty/error states). The route now renders live picker data with real loading, retryable-error and empty states, so flipped `ppt-web.buildStatus: in-progress → shipped` (client-lag that kept it in-progress is resolved; `apiStatus` already `complete`). Rewrote States to describe the retryable `role="alert"` error notice + Retry (`CreateLeasePage` `isError`/`onRetry`, `useOrganizationUnits` `isError`/`refetch`) and the inline `noUnits`/`noTenants` empty hints in `LeaseForm`; documented the new i18n keys and the `leases.create-pickers.route.test.tsx` regression. `endpoints:` kept `[]` (leases ops not in `@ppt/sitemap`, listing them would break `/screens validate`). `/screens validate` clean.
