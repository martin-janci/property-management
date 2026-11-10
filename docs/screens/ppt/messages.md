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
  - Epic-6
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
- 2026-06-25 — group conversations (BIT-244): thread list renders the full participant set, not one arbitrary "other". `ThreadWithPreview.otherParticipant` → `participants: ParticipantInfo[]` (backend PR #1848); list preview attributed to the actual last-message sender.
- 2026-08-31 — realtime sync (PR #2889): the `messages` query root now auto-refetches on a `message.created` frame (and on `notification.created` with `category: messages`). Previously `WebSocketContext.eventToQueryKeys` keyed on dead `entity:*` names the api-server never emits, and `App.tsx` never passed `onEntityEvent`, so this list never refreshed in realtime (100% dead sync). REST wiring unchanged.

## Agent Log
- 2026-08-31 — agent: screen-map-drift-pr-2889-ppt — reconcile drift from PR #2889 (realtime ws→query-invalidation fix). ppt-web `WebSocketContext` re-keyed cache invalidation from dead `entity:*` names to the api-server's canonical `domain.action` events, and `App.tsx` now wires `onEntityEvent → queryClient.invalidateQueries`, so the `messages` root is invalidated on `message.created` / `notification.created(category=messages)`. No route/component/endpoint/status change — frontmatter unchanged; docs-only.
- 2026-07-13 — agent: gap-screens-normalize-frontmatter — normalized story-id-style epic ref(s) Epic-6-5 → Epic-6 (strip story suffix); /screens validate clean.
- 2026-06-25 — agent: group conversations (BIT-244 / PM #972.5b) — `mapApiThreadToUi` maps every other participant from `participants[]`; preview sender resolved from the participant list. No route/status change.
- 2026-05-18 — agent: created stub for unmapped route.
- 2026-05-24 — agent: promoted apiStatus stub→integrated; wired MessagesPageRoute to useThreads/useUnreadCount hooks.
- 2026-06-25 — agent: reconciliation pass — sprint-status 6-5 flipped ready-for-dev→done. Backend (20+ endpoints, RLS repo, migrations 00017/00018/00019/00189/00190/00191, cross-tenant tests) and ppt-web (MessagesPage/ThreadDetailPage/NewMessagePage + hooks) confirmed shipped. Gate #486 satisfied: getToken() in useMessaging.ts routes through centralised token-provider.ts (globalTokenProvider via AuthContext), not a raw bypass. Mobile messaging UI deferred and not overstated.
