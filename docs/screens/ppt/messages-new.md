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
  - "6-5"
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
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:491`.
- 2026-05-24 — api-integration: wired to POST /messages/threads (useStartThread) + GET neighbors for recipient list; API only supports single recipient per thread.
- 2026-06-03 — bugfix (PR #922): recipient list now sourced from the user's first building via `useBuildings` (was `useMessageRecipients(undefined)` → empty/unusable). Added `?recipientId=` preselection passed through as `initialRecipientIds`. Loading state now also waits on the buildings query.

## Agent Log
- 2026-06-03 — agent: test-gap-screen-map-drift-pr-922-ppt — reconciled drift from PR #922 (dev-review rounds 1-5). NewMessagePageRoute now resolves buildingId via useBuildings + honors ?recipientId= preselection; updated Specific notes. buildStatus/apiStatus unchanged (shipped/complete).
- 2026-05-24 — agent: promoted apiStatus stub→integrated; wired NewMessagePageRoute to useStartThread/useMessageRecipients hooks.
- 2026-05-18 — agent: created stub for unmapped route.
