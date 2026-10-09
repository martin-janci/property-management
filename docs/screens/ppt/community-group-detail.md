---
id: ppt/community-group-detail
name: Community Group Detail
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/community/groups/:groupId"
    component: GroupDetailPage
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

# Community Group Detail

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

## Notes

### Specific (recent)
- 2026-10-09 — PR #3034 wired the route to real Community API data. `GroupDetailPageInner` (exported from `frontend/apps/ppt-web/src/routes/groups/community.tsx`) now fetches the group + members for `groupId` via `useGroup`/`useGroupMembers`, renders a loading spinner and a proper not-found state (no more fabricated "Sample Group" mock), derives `isMember`/`isOwner`/`isAdmin` from the member list vs `useAuth().user?.id`, and wires Join/Leave to `useJoinGroup`/`useLeaveGroup` with busy states. Hence `apiStatus: stub → partial`. Edit/Delete/Settings and member promote/remove/ban stay no-ops — the API client exposes no endpoints for those yet. `endpoints:` left empty because the backing community-group operations are not present in `@ppt/sitemap` (`apiServerEndpoints`), so listing them would fail `/screens validate`; populate once the sitemap tracks them.
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:512`.

## Agent Log
- 2026-05-18 — agent: created stub for unmapped route.
- 2026-08-18 — agent: screen-map-drift-pr-2647 — noted i18n update in PR #2647: `GroupDetailPageRoute` in `frontend/apps/ppt-web/src/routes/groups/community.tsx` now renders its missing-param fallback via `t('errors.groupNotFound', 'Group not found')` (was hardcoded English); `errors.groupNotFound` key added to all locale bundles. No route or component change.
- 2026-10-09 — agent: screen-map-drift-pr-3034 — reconciled with PR #3034 (merged): route wired to real Community API (`useGroup`/`useGroupMembers`/`useJoinGroup`/`useLeaveGroup`), fabricated "Sample Group" mock removed, loading/not-found states added. Flipped `apiStatus: stub → partial`. No route/component/buildStatus change. `endpoints:` kept empty — community-group ops absent from `@ppt/sitemap`, so listing them would break `/screens validate`.
