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
- 2026-11-16 — PR #3486 wrapped this route in `<ProtectedRoute>` (auth-only, not role-gated) in `frontend/apps/ppt-web/src/routes/groups/community.tsx` — matching the guarded peer groups and the `/settings/privacy` precedent. An anonymous visitor is now redirected to `/login` (return-URL captured) instead of rendering page chrome; manager-only in-page actions remain gated via `isManagerRole`. `buildStatus`/`apiStatus` unchanged. Guard regression-covered by `routes/groups/mgmt-routes-auth-guard.route.test.tsx`.
- 2026-11-04 — Feed is now real: fans out `GET /community/buildings/{building_id}/groups` → `GET /community/groups/{id}/posts?limit=20` for every group the caller belongs to and merges newest-first (the server has no cross-group feed endpoint). Composer posts via `POST /community/groups/{id}/posts`; Like via `POST /community/posts/{id}/reactions`. Comment/share/edit/delete/post-detail hidden — no server endpoints. Building scope via `?building=` (`features/community/hooks/useCommunityBuilding.ts`). `apiStatus: partial` until verified end-to-end: the backend community happy-path tests are quarantined (BIT-440) and the repo decodes Postgres enum columns into `String` via `SELECT *`, which likely fails at runtime.
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:505`.

## Agent Log
- 2026-11-16 — agent: screen-map-drift-pr-3486 — reconciled with merged PR #3486: route now gated behind `<ProtectedRoute>` (auth-only, not role-gated) in `routes/groups/community.tsx`; anonymous visitors redirect to `/login`. No route/component/buildStatus/apiStatus change.
- 2026-05-18 — agent: created stub for unmapped route.
- 2026-11-04 — agent: Feed wired to real group/post endpoints (fan-out), composer + like; no-op handlers removed. Tests: `features/community/**/*.test.tsx` + `api-client/src/community/api.test.ts`.
