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
    apiStatus: complete
endpoints: []
relatedScreens:
  - id: ppt/messages
    rel: parent
sharedComponents: []
diagrams: []
useCases:
  - UC-07
  - UC-05.9
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
- 2026-06-22 — attachments (UC-05.9, BIT-208): composer now keeps the real `File` and on send runs request-upload-url → PUT bytes → link per attachment (`useSendMessageWithAttachments`). Received messages lazily resolve their attachments via `useMessageAttachments` and download through short-lived presigned URLs. Client guards: 25 MiB cap + storage allow-list (incl. text/csv). Backend (PR #1702) not yet merged; browser QA against a MinIO stack still pending.

## Agent Log
- 2026-05-18 — agent: created stub for unmapped route.
- 2026-05-24 — agent: promoted apiStatus stub→integrated; wired ThreadDetailPageRoute to useThread/useSendMessage/useMarkThreadRead hooks.
- 2026-06-22 — agent: wired message attachments UI (UC-05.9) — upload+link on send, lazy attachment list + presigned download on received messages; unit-tested the send orchestration.
