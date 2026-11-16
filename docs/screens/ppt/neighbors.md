---
id: ppt/neighbors
name: Neighbors
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/neighbors"
    component: NeighborsPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: complete
  mobile:
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: complete
endpoints: []
relatedScreens: []
sharedComponents: []
diagrams: []
useCases: []
epics:
  - Epic-27
designSources: []
owner: pm-frontend
---

# Neighbors

List of neighbors within the same building, accessible to residents. Wired to backend `GET /api/v1/buildings/{id}/neighbors`. Includes a neighbor detail view (`/neighbors/:neighborId`) and a privacy-settings sub-screen (`/neighbors/privacy`).

## Notes

### Specific (recent)
- 2026-11-16 — PR #3486 wrapped this route in `<ProtectedRoute>` (auth-only, not role-gated) in `frontend/apps/ppt-web/src/routes/groups/neighbors.tsx` — matching the guarded peer groups and the `/settings/privacy` precedent. An anonymous visitor is now redirected to `/login` (return-URL captured) instead of rendering page chrome; manager-only in-page actions remain gated via `isManagerRole`. `buildStatus`/`apiStatus` unchanged. Guard regression-covered by `routes/groups/mgmt-routes-auth-guard.route.test.tsx`. The sibling `/neighbors/:neighborId` (NeighborDetailPage) and `/neighbors/privacy` (NeighborsPrivacySettingsPage) sub-routes are gated the same way but have no dedicated screen-map docs.
- 2026-05-25 — agent: created screen-map for neighbors feature added in PR #464 (gap-6-6-neighbor-listing-verify). Routes: `/neighbors`, `/neighbors/:neighborId`, `/neighbors/privacy` — components `NeighborsPage`, `NeighborDetailPage`, `NeighborsPrivacySettingsPage`.

## Agent Log
- 2026-11-16 — agent: screen-map-drift-pr-3486 — reconciled with merged PR #3486: route now gated behind `<ProtectedRoute>` (auth-only, not role-gated) in `routes/groups/neighbors.tsx`; anonymous visitors redirect to `/login`. No route/component/buildStatus/apiStatus change.
- 2026-05-25 — agent: created stub to resolve screen-map drift (backlog: test-gap-screen-map-drift-ppt-neighbors).
