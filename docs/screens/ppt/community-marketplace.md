---
id: ppt/community-marketplace
name: Community Marketplace
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/community/marketplace"
    component: MarketplacePage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: partial
endpoints: []
relatedScreens:
  - id: ppt/community
    rel: parent
sharedComponents: []
diagrams: []
useCases: []
epics:
  - Epic-27
designSources: []
owner: pm-frontend
---

# Community Marketplace

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

## Notes

### Specific (recent)
- 2026-11-04 — Marketplace now real: `GET /community/buildings/{building_id}/marketplace?category=&limit=24&offset=` (server-side category + paging), cards open `/community/marketplace/:itemId`; create → `/community/marketplace/new`. Edit/delete/mark-sold hidden — no endpoints. `apiStatus: partial` until verified end-to-end: the backend community happy-path tests are quarantined (BIT-440) and the repo decodes Postgres enum columns into `String` via `SELECT *`, which likely fails at runtime.
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:517`.

## Agent Log
- 2026-05-18 — agent: created stub for unmapped route.
- 2026-11-04 — agent: marketplace list wired with server-side category filter + paging; detail/create routes added. Tests: `features/community/**/*.test.tsx` + `api-client/src/community/api.test.ts`.
