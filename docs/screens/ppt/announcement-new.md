---
id: ppt/announcement-new
name: Create Announcement
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/announcements/new"
    component: CreateAnnouncementPage
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

# Create Announcement

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

## Notes

### Specific (recent)
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:478`.

## Agent Log
- 2026-11-04 — agent: create/update/schedule/comment bodies are mapped to the server's snake_case fields (was 422 `missing field target_type`) (R3 #8). Status unchanged.
- 2026-05-18 — agent: created stub for unmapped route.
