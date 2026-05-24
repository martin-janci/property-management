---
id: ppt/message-thread
name: Message Thread
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/messages/:threadId"
    component: ThreadDetailPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: integrated
endpoints:
  - GET /api/v1/messages/threads/:id
  - POST /api/v1/messages/threads/:id/messages
  - POST /api/v1/messages/threads/:id/read
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

# Message Thread

Wired to real API in 2026-05-24 (Epic 6, Story 6.5). ThreadDetailPageRoute uses
`useThread`, `useSendMessage`, and `useMarkThreadRead` hooks. Data is adapted from
the API's `ThreadDetailResponse` to the feature-layer `ThreadWithMessages` type via
`mapApiThreadDetailToUi`.

## Notes

### Specific (recent)
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:492`.
- 2026-05-24 — api-integration: wired to GET /threads/:id, POST /messages, POST /read; loading skeleton shown while data fetches.

## Agent Log
- 2026-05-18 — agent: created stub for unmapped route.
- 2026-05-24 — agent: promoted apiStatus stub→integrated; wired ThreadDetailPageRoute to useThread/useSendMessage/useMarkThreadRead hooks.
