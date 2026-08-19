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
    apiStatus: stub
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
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:512`.

## Agent Log
- 2026-05-18 — agent: created stub for unmapped route.
- 2026-08-18 — agent: screen-map-drift-pr-2647 — noted i18n update in PR #2647: `GroupDetailPageRoute` in `frontend/apps/ppt-web/src/routes/groups/community.tsx` now renders its missing-param fallback via `t('errors.groupNotFound', 'Group not found')` (was hardcoded English); `errors.groupNotFound` key added to all locale bundles. No route or component change.
