---
id: ppt/announcement-edit
name: Edit Announcement
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/announcements/:announcementId/edit"
    component: EditAnnouncementPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: stub
endpoints: []
relatedScreens:
  - id: ppt/announcements
    rel: parent
sharedComponents: []
diagrams: []
useCases: []
epics:
  - Epic-6
designSources: []
owner: pm-frontend
---

# Edit Announcement

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

## Notes

### Specific (recent)
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:486`.

## Agent Log
- 2026-05-18 — agent: created stub for unmapped route.
- 2026-08-18 — agent: screen-map-drift-pr-2647 — noted i18n update in PR #2647: `EditAnnouncementPageRoute` in `frontend/apps/ppt-web/src/routes/groups/announcements.tsx` now renders its missing-param fallback via `t('errors.announcementNotFound', 'Announcement not found')` (was hardcoded English); `errors.announcementNotFound` key added to all locale bundles. No route or component change.
