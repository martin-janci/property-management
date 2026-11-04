---
id: ppt/community-groups
name: Community Groups
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/community/groups"
    component: GroupsPage
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

# Community Groups

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

## Notes

### Specific (recent)
- 2026-11-04 — Groups list is now real: `GET /community/buildings/{building_id}/groups` (with caller membership), Join/Leave via `POST /community/groups/{id}/join|leave`; search/type/my-groups filters run client-side (server returns the whole list). Owners see their role instead of Leave. `apiStatus: partial` until verified end-to-end: the backend community happy-path tests are quarantined (BIT-440) and the repo decodes Postgres enum columns into `String` via `SELECT *`, which likely fails at runtime.
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:506`.

## Agent Log
- 2026-05-18 — agent: created stub for unmapped route.
- 2026-11-04 — agent: groups list wired to per-building endpoint, join/leave mutations, client-side filters. Tests: `features/community/**/*.test.tsx` + `api-client/src/community/api.test.ts`.
