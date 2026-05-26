# pm-frontend — 2026-05-25

_Frontend/mobile delivery lens. Daily PM rotation (rotation index 2). Static, read-only analysis._

## Summary

The biggest frontend delivery burst of the sprint landed this run — document sharing/permission UI (web+mobile), OAuth client-management + user-grants UI, MFA frontend+e2e, neighbor listing, and direct-messaging screens with WebSocket realtime sync all merged (PRs #441–#473). The active frontend front is now the Epic 6 announcement web UI, split across three unmerged draft PRs (#474/#475/#479) on a shared AnnouncementsPage — sequencing them is the near-term priority, alongside the #480–#487 follow-up test-gap batch.

## Next actions

| Priority | Action | Dependency | Definition of done |
|---|---|---|---|
| high | Sequence + land Epic 6 announcement web UI drafts in order #474 (viewing/ack) → #475 (comments) → #479 (pin) | none | 6-2/6-3/6-4 apiStatus → integrated; 3 PRs merged without AnnouncementsPage conflicts; stories promote from partial |
| high | Verify Epic 81 schedule/execution UI e2e as PRs #488/#489 land | pm-backend (endpoints) | EditScheduleModal pause/resume + ExecutionHistory no longer 404; 81-1/81-2 promoted |
| medium | Slot follow-up issues #480–#487 into one test-hardening batch | pm-scrum-master | Each issue closed or explicitly deferred before its parent story promotes to done |
| medium | Build dedicated folder-tree UI page for document organization (7a-2) | none | Folder hierarchy CRUD UI wired to backend 5-level tree; mobile slice scoped |
| medium | Wire AnnouncementsPage/FaultsPage to @ppt/api-client hooks (79-1) | none | Both pages off mocks, using real query hooks; loading/empty/error states present |
| low | Mobile document download/preview slice (7a-4 mobile) + RN env config (85-1) | none | DocumentPreviewScreen calls get_preview_url; Expo app.config.ts + iOS Info.plist keys added |

## Risks

| Risk | Prob | Impact | Mitigation |
|---|---|---|---|
| Epic 6 announcement web UI split across 3 unmerged drafts (#474/#475/#479) on shared AnnouncementsPage — review-queue stall + merge conflicts | medium | medium | Land in dependency order #474 → #475 → #479; promote out of draft once apiStatus verified against the now-live notification pipeline |
| Follow-up test gaps (#480–#487) on merged messaging/share/OAuth/MFA features — done-count outruns regression coverage | high | medium | Schedule as a single hardening batch; gate done-promotion on the follow-up closing |
| `ai.rs` cross-tenant IDOR cluster (update/delete equipment + update_maintenance discard principal) — not a frontend file but the equipment UI consumes these endpoints | high | high | Flagged for pm-backend (`pm-backend-fix-ai-equipment-idor`); frontend should not surface delete/update equipment actions until backend scopes by tenant |

## Open questions

- Does the messaging realtime sync (PR #472) cover optimistic UI / reconnection on the web client, or only server push? (relevant to 6-5 e2e coverage in #480–#487)
- Are the OAuth user-grants and admin client-management UIs (#468/#469/#471) covered by any frontend tests, or are those part of the #480–#487 gap set?

## Decisions needed

- Epic 6 announcement web UI: single squashed PR vs. incremental #474 → #475 → #479 — owner: pm-frontend / pm-scrum-master.
