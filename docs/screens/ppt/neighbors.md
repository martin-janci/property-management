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
- 2026-05-25 — agent: created screen-map for neighbors feature added in PR #464 (gap-6-6-neighbor-listing-verify). Routes: `/neighbors`, `/neighbors/:neighborId`, `/neighbors/privacy` — components `NeighborsPage`, `NeighborDetailPage`, `NeighborsPrivacySettingsPage`.

## Agent Log
- 2026-05-25 — agent: created stub to resolve screen-map drift (backlog: test-gap-screen-map-drift-ppt-neighbors).
