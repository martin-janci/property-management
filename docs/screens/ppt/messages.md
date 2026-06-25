---
id: ppt/messages
name: Messages
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/messages"
    component: MessagesPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: complete
endpoints: []
relatedScreens: []
sharedComponents: []
diagrams: []
useCases:
  - UC-07
epics:
  - "6-5"
designSources: []
owner: pm-frontend
---

# Messages

Wired to real API in 2026-05-24 (Epic 6, Story 6.5). MessagesPageRoute uses
`useThreads` and `useUnreadCount` from `@ppt/api-client` via the local
`features/messaging/hooks/useMessaging.ts` adapter. Pagination is mapped from
the UI's page/pageSize params to the API's limit/offset params.

## Notes

### Specific (recent)
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:490`.
- 2026-05-24 — api-integration: wired to GET /api/v1/messages/threads + unread-count; onDeleteThreads shows a not-supported toast (API does not expose thread deletion).

## Agent Log
- 2026-05-18 — agent: created stub for unmapped route.
- 2026-05-24 — agent: promoted apiStatus stub→integrated; wired MessagesPageRoute to useThreads/useUnreadCount hooks.
- 2026-06-25 — agent: reconciliation pass — sprint-status 6-5 flipped ready-for-dev→done. Backend (20+ endpoints, RLS repo, migrations 00017/00018/00019/00189/00190/00191, cross-tenant tests) and ppt-web (MessagesPage/ThreadDetailPage/NewMessagePage + hooks) confirmed shipped. Gate #486 satisfied: getToken() in useMessaging.ts routes through centralised token-provider.ts (globalTokenProvider via AuthContext), not a raw bypass. Mobile messaging UI deferred and not overstated.
