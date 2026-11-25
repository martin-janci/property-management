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
    apiStatus: complete
endpoints: []
relatedScreens:
  - id: ppt/messages
    rel: parent
sharedComponents: []
diagrams: []
useCases:
  - UC-07
epics:
  - Epic-6
designSources: []
owner: pm-frontend
---

# New Message

Wired to real API in 2026-05-24 (Epic 6, Story 6.5). NewMessagePageRoute uses
`useMessageRecipients` (sourced from the neighbors API — visible neighbors only)
and `useStartThread` to open a new conversation. On success, navigates to the
new thread's detail page.

The recipient list is now resolved from the user's first accessible building
(via `useBuildings`, mirroring the NeighborsPage convention) — the previous
`useMessageRecipients(undefined)` call left the neighbors query permanently
disabled, so the recipient list was always empty and the page unusable
(fixed in PR #922, dev-review round 5). The route also honors a `?recipientId=`
query param to preselect a recipient (e.g. arriving from "Contact" on a
neighbor). A dedicated building selector is still deferred until building
context flows through auth.

## Notes

### Specific (recent)
- 2026-11-16 — PR #3486 wrapped this route in `<ProtectedRoute>` (auth-only, not role-gated) in `frontend/apps/ppt-web/src/routes/groups/messaging.tsx` — matching the guarded peer groups and the `/settings/privacy` precedent. An anonymous visitor is now redirected to `/login` (return-URL captured) instead of rendering page chrome; manager-only in-page actions remain gated via `isManagerRole`. `buildStatus`/`apiStatus` unchanged. Guard regression-covered by `routes/groups/mgmt-routes-auth-guard.route.test.tsx`.
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:491`.
- 2026-05-24 — api-integration: wired to POST /messages/threads (useStartThread) + GET neighbors for recipient list; API only supports single recipient per thread.
- 2026-06-03 — bugfix (PR #922): recipient list now sourced from the user's first building via `useBuildings` (was `useMessageRecipients(undefined)` → empty/unusable). Added `?recipientId=` preselection passed through as `initialRecipientIds`. Loading state now also waits on the buildings query.
- 2026-06-25 — group conversations (BIT-244): `toStartThreadRequest` now sends the **full** recipient set as snake_case `recipient_ids` (was collapsed to a single `recipientId`), so multi-select recipients open one N-party thread (UC-05.8 / BIT-183). Backend merge: PR #1848.

## Agent Log
- 2026-11-24 — agent: gap-audit-r14 — mobile counterpart added (`NewMessageScreen`), see ppt/messages log. No ppt-web change.
- 2026-11-16 — agent: screen-map-drift-pr-3486 — reconciled with merged PR #3486: route now gated behind `<ProtectedRoute>` (auth-only, not role-gated) in `routes/groups/messaging.tsx`; anonymous visitors redirect to `/login`. No route/component/buildStatus/apiStatus change.
- 2026-11-04 — agent: recipient picker now gets neighbors: the neighbors client maps the snake_case `NeighborView` (every neighbor was filtered out as hidden) and builds auth headers per request (R3 #18). The building list still needs `organization_id` (buildings-contract). Status unchanged.
- 2026-07-13 — agent: gap-screens-normalize-frontmatter — normalized story-id-style epic ref(s) Epic-6-5 → Epic-6 (strip story suffix); /screens validate clean.
- 2026-06-25 — agent: group conversations (BIT-244 / PM #972.5b) — `toStartThreadRequest` sends `recipient_ids: string[]` (snake_case wire); reconciled camelCase `recipientId` → snake_case to match backend `StartThreadRequest`. buildStatus/apiStatus unchanged (shipped/complete).
- 2026-06-03 — agent: test-gap-screen-map-drift-pr-922-ppt — reconciled drift from PR #922 (dev-review rounds 1-5). NewMessagePageRoute now resolves buildingId via useBuildings + honors ?recipientId= preselection; updated Specific notes. buildStatus/apiStatus unchanged (shipped/complete).
- 2026-05-24 — agent: promoted apiStatus stub→integrated; wired NewMessagePageRoute to useStartThread/useMessageRecipients hooks.
- 2026-05-18 — agent: created stub for unmapped route.
