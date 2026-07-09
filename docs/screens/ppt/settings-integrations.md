---
id: ppt/settings-integrations
name: Integrations
product: ppt
sitemapRefs:
  ppt-web: ppt-settings-integrations
implementations:
  ppt-web:
    route: "/settings/integrations"
    component: IntegrationsPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: complete
endpoints: []
relatedScreens:
  - id: ppt/settings-notifications
    rel: sibling
sharedComponents: []
diagrams: []
useCases:
  - UC-29
epics:
  - Epic-83
designSources: []
owner: pm-frontend
---

# Integrations

Manager-facing surface for external channel integrations. Gap 83-1 wired the
previously route-less integrations UI: the `@ppt/api-client` `integrations`
module (Epic 61 + Gap 83) already carried live Airbnb hooks, but no ppt-web
route rendered them. `IntegrationsPage` now renders the Airbnb card with the
OAuth connect flow, sync, disconnect, live status, and listing mappings.

Airbnb calls (via `@ppt/api-client` `integrations` hooks):
- `useAirbnbStatus` — GET `/api/v1/integrations/organizations/{org}/airbnb/status`
- `useConnectAirbnb` — POST `.../airbnb/connect` (returns OAuth `auth_url`)
- `useSyncAirbnb` — POST `.../airbnb/sync`
- `useDisconnectAirbnb` — DELETE `.../airbnb`
- `useAirbnbListingMappings` — GET `/api/v1/rentals/connections` (platform=airbnb)

## Notes

### Specific (recent)
- 2026-07-08 — created `IntegrationsPage`, wired route `/settings/integrations`
  in `routes/groups/core.tsx` (settingsRoutes), and added the
  `ppt-settings-integrations` sitemap entry. `endpoints` left empty because the
  Airbnb integration endpoints are not (yet) in the `@ppt/sitemap` endpoint
  catalogue.

## Agent Log
- 2026-07-08 — agent: gap-83-1-frontend-integrations-ui-not-wired-settings — created the Integrations settings page (Airbnb OAuth connect/sync/disconnect + status + listing mappings) and wired `/settings/integrations` so the built-but-route-less integrations api-client is reachable.
