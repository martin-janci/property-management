---
id: ppt/community-events
name: Community Events
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/community/events"
    component: EventsPage
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

# Community Events

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

## Notes

### Specific (recent)
- 2026-11-04 — Events now real: `GET /community/buildings/{building_id}/events?limit=50` (upcoming, scheduled), RSVP Going/Maybe/Can't go via `POST /community/events/{id}/rsvp` (upsert; the list carries no own-RSVP, so the confirmed status is kept per session). Create → `/community/events/new`. Edit/delete/calendar export/event detail hidden — no endpoints. `apiStatus: partial` until verified end-to-end: the backend community happy-path tests are quarantined (BIT-440) and the repo decodes Postgres enum columns into `String` via `SELECT *`, which likely fails at runtime.
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:515`.

## Agent Log
- 2026-05-18 — agent: created stub for unmapped route.
- 2026-11-04 — agent: events list + RSVP wired; create route added; edit/delete/export hidden. Tests: `features/community/**/*.test.tsx` + `api-client/src/community/api.test.ts`.
