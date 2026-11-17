---
id: ppt/community-groups-new
name: Create Community Group
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/community/groups/new"
    component: CreateGroupPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: partial
endpoints: []
relatedScreens:
  - id: ppt/community-groups
    rel: parent
sharedComponents: []
diagrams: []
useCases: []
epics:
  - Epic-27
designSources: []
owner: pm-frontend
---

# Create Community Group

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

## Notes

### Specific (recent)
- 2026-11-16 — PR #3486 wrapped this route in `<ProtectedRoute>` (auth-only, not role-gated) in `frontend/apps/ppt-web/src/routes/groups/community.tsx` — matching the guarded peer groups and the `/settings/privacy` precedent. An anonymous visitor is now redirected to `/login` (return-URL captured) instead of rendering page chrome; manager-only in-page actions remain gated via `isManagerRole`. `buildStatus`/`apiStatus` unchanged. Guard regression-covered by `routes/groups/mgmt-routes-auth-guard.route.test.tsx`.
- 2026-11-04 — Create group now calls `POST /community/buildings/{building_id}/groups` (name, description, group_type, visibility public/private/secret, rules); success opens the new group, failure shows the server message inline + error toast and keeps the form (previously it toasted success without any API call). `apiStatus: partial` until verified end-to-end: the backend community happy-path tests are quarantined (BIT-440) and the repo decodes Postgres enum columns into `String` via `SELECT *`, which likely fails at runtime.
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:508`.

## Agent Log
- 2026-11-16 — agent: screen-map-drift-pr-3486 — reconciled with merged PR #3486: route now gated behind `<ProtectedRoute>` (auth-only, not role-gated) in `routes/groups/community.tsx`; anonymous visitors redirect to `/login`. No route/component/buildStatus/apiStatus change.
- 2026-05-18 — agent: created stub for unmapped route.
- 2026-11-04 — agent: create group calls the real API; error path surfaced inline + toast. Tests: `features/community/**/*.test.tsx` + `api-client/src/community/api.test.ts`.
