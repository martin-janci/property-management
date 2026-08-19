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
  - Epic-6
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
- 2026-06-25 — group conversations (BIT-244): detail header renders all other participants (`ThreadDetailResponse.otherParticipant` → `participants: ParticipantInfo[]`, backend PR #1848). `formatParticipantNames` already collapses N>2 to "X and N others".

## Agent Log
- 2026-07-13 — agent: gap-screens-normalize-frontmatter — normalized story-id-style epic ref(s) Epic-6-5 → Epic-6 (strip story suffix); /screens validate clean.
- 2026-06-25 — agent: group conversations (BIT-244 / PM #972.5b) — `mapApiThreadDetailToUi` maps every other participant from `participants[]`; the N-party header rendering was already in place. No route/status change.
- 2026-05-18 — agent: created stub for unmapped route.
- 2026-05-24 — agent: promoted apiStatus stub→integrated; wired ThreadDetailPageRoute to useThread/useSendMessage/useMarkThreadRead hooks.
- 2026-06-22 — agent: wired message attachments UI (UC-05.9) — upload+link on send, lazy attachment list + presigned download on received messages; unit-tested the send orchestration.
- 2026-08-18 — agent: screen-map-drift-pr-2647 — noted i18n update in PR #2647: `ThreadDetailPageRoute` in `frontend/apps/ppt-web/src/routes/groups/messaging.tsx` now renders its missing-param fallback via `t('errors.threadNotFound', 'Thread not found')` (was hardcoded English); `errors.threadNotFound` key added to all locale bundles. No route or component change.
