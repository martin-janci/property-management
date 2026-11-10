---
id: ppt/community
name: Community Feed
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/community"
    component: FeedPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: partial
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

# Community Feed

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

## Notes

### Specific (recent)
- 2026-11-04 — Feed is now real: fans out `GET /community/buildings/{building_id}/groups` → `GET /community/groups/{id}/posts?limit=20` for every group the caller belongs to and merges newest-first (the server has no cross-group feed endpoint). Composer posts via `POST /community/groups/{id}/posts`; Like via `POST /community/posts/{id}/reactions`. Comment/share/edit/delete/post-detail hidden — no server endpoints. Building scope via `?building=` (`features/community/hooks/useCommunityBuilding.ts`). `apiStatus: partial` until verified end-to-end: the backend community happy-path tests are quarantined (BIT-440) and the repo decodes Postgres enum columns into `String` via `SELECT *`, which likely fails at runtime.
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:505`.

## Agent Log
- 2026-05-18 — agent: created stub for unmapped route.
- 2026-11-04 — agent: Feed wired to real group/post endpoints (fan-out), composer + like; no-op handlers removed. Tests: `features/community/**/*.test.tsx` + `api-client/src/community/api.test.ts`.
