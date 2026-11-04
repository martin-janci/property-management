---
id: ppt/community-events-new
name: Create Community Event
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/community/events/new"
    component: CreateEventPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: partial
endpoints: []
relatedScreens:
  - id: ppt/community-events
    rel: parent
sharedComponents: []
diagrams: []
useCases: []
epics:
  - Epic-27
designSources: []
owner: pm-frontend
---

# Create Community Event

Form to schedule a community event in the selected building.

## Notes

### Specific (recent)
- 2026-11-04 — Submits `POST /community/buildings/{building_id}/events` (title, type, description, start/end as ISO instants or all-day, location, online link, max attendees, cost per person); validates end > start; returns to the events list. `endpoints:` left empty — community ops are absent from `@ppt/sitemap`. `apiStatus: partial` until verified end-to-end (backend community happy-path tests quarantined, BIT-440).

## Agent Log
- 2026-11-04 — agent: created with the route (`frontend/apps/ppt-web/src/routes/groups/community.tsx`); page in `features/community/pages/CreateEventPage.tsx`.
