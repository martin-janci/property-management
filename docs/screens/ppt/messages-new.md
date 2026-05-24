---
id: ppt/messages-new
name: New Message
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/messages/new"
    component: NewMessagePage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: integrated
endpoints:
  - GET /api/v1/buildings/:buildingId/neighbors
  - POST /api/v1/messages/threads
relatedScreens:
  - id: ppt/messages
    rel: parent
sharedComponents: []
diagrams: []
useCases:
  - UC-07
epics:
  - "6-5"
designSources: []
owner: pm-frontend
---

# New Message

Wired to real API in 2026-05-24 (Epic 6, Story 6.5). NewMessagePageRoute uses
`useMessageRecipients` (sourced from the neighbors API — visible neighbors only)
and `useStartThread` to open a new conversation. On success, navigates to the
new thread's detail page.

Note: buildingId is not available in the auth token; the recipient list is empty
until the neighbors API is called with a building context. A follow-up task should
pass buildingId from the user's building context once that data flows through auth.

## Notes

### Specific (recent)
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:491`.
- 2026-05-24 — api-integration: wired to POST /messages/threads (useStartThread) + GET neighbors for recipient list; API only supports single recipient per thread.

## Agent Log
- 2026-05-18 — agent: created stub for unmapped route.
- 2026-05-24 — agent: promoted apiStatus stub→integrated; wired NewMessagePageRoute to useStartThread/useMessageRecipients hooks.
