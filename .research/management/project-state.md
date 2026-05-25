# PPT Project State

_Generated: 2026-05-25 — daily PM rotation (Scrum Master + pm-frontend). Coverage map last rebuilt by `/ppt-project-management scan` on 2026-05-23; upkeep-refreshed 2026-05-25._

## Executive summary

- **30 PRs merged since the last run (#441–#473)** — the biggest delivery burst of the sprint, almost entirely frontend/feature work. Highlights: **OAuth provider UI** (client-management admin + user-grants revoke, PRs #468/#469/#471), **document sharing & permission UI** across web and mobile (gap-7a-* cluster: #443/#445/#447/#451/#462/#465/#467), **MFA frontend + e2e** (#441/#473), **Epic 2B notification pipeline** (#463), and **WebSocket realtime sync** (#472).
- **Two long-standing blockers cleared.** Epic 2B notification pipeline (#463) plus WebSocket realtime sync (#472) fired the DEC-001 unblock triggers — 6-2/6-3/6-4 announcement web UI, 6-5 direct messaging, and the WS half of 8a-3 are all unblocked. 6-5 and 9-1 (MFA) advanced to **done**; the Epic 6 announcement web UI is now in-flight across draft PRs #474/#475/#479.
- **Three security fixes landed** — voice-device IDOR (#461), SSRF outbound-URL validation (#450), and the ProtectedRoute fail-open role guard (#459) — clearing three previously-open risks.
- **New code-review finding (high):** a cross-tenant IDOR cluster in `ai.rs` equipment endpoints — `update_equipment`/`delete_equipment`/`update_maintenance` bind `_principal` and discard it, issuing unscoped `DELETE/UPDATE ... WHERE id=$1`. This is exactly the failure mode predicted for the 3k-line route monoliths (ai.rs 3142 LOC).
- **Follow-up issues #480–#487** track test gaps + minor security/UX follow-ups on the merged PRs — scheduled as a hardening batch so the done-count doesn't outrun test coverage.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

| Epic | Tracked status | Real status (from coverage upkeep) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6-1/6-5/6-6 done; 6-2/6-3/6-4 web UI in draft (#474/#475/#479), backend+infra ready |
| 7A — Basic Document Management | in-progress | 7a-3/7a-5 done (web+mobile UI merged); 7a-1 mobile merged; 7a-2/7a-4-mobile still partial |
| 8A — Basic Notification Preferences | in-progress | 8a-1/8a-2 done; 8a-3 WS half done (#472), mobile-push leg remains |
| 9 — Account Security (MFA) | in-progress | 9-1 done — frontend integration (#441) + e2e (#473) merged |
| 10A — OAuth Provider Foundation | in-progress | 3 backend done + admin/user-grants UI merged (#468/#469/#471); only integration tests remain |
| 10B — Platform Administration | in-progress | 3 done, 4 partial (handler stubs 10b-4/5/6/7); admin health UI still missing |

## What's next (top 5)

1. **[high · pm-backend]** Fix the cross-tenant IDOR cluster in `ai.rs` — scope `update_equipment`/`delete_equipment`/`update_maintenance` by tenant (or route through RLS connection) + add a cross-tenant regression test. _Why:_ an authenticated user in any org can delete/mutate another org's equipment rows today.
2. **[high · pm-frontend]** Sequence and land the Epic 6 announcement web UI drafts in dependency order #474 (viewing/ack) → #475 (comments) → #479 (pin). _Why:_ backend + notification pipeline are live; the only thing gating 6-2/6-3/6-4 closure is this three-PR cluster — a stall risks a review-queue jam.
3. **[high · pm-backend]** Implement the missing Epic 81 report-schedule endpoints (pause/resume/executions). _Why:_ frontend already calls them — 404 in production; verify PRs #488/#489 in flight.
4. **[high · pm-backend]** Build the mobile push leg (FCM/APNs) for 8a-3 — the WS half is now delivered (#472). _Why:_ last slice before 8a-3 promotes to done.
5. **[medium · pm-frontend]** Slot follow-up issues #480–#487 into a single test-hardening batch; gate done-promotion of the merged frontend features on their follow-up closing. _Why:_ heavy delivery shipped without full regression coverage on auth/sharing surfaces.

See `roadmap.md` for the full ranked plan and `action-list.json`/`action-list.md` for the tracker view.

## Blockers

- **None hard-blocking this run.** Epic 2B WebSocket/notification infrastructure — the prior sprint's top blocker — is cleared (#463/#472). Remaining gating items are now ordinary work, not blockers.
- **Watch:** the Epic 6 announcement web UI is split across three unmerged drafts (#474/#475/#479) sharing AnnouncementsPage — a soft review-queue risk, not a hard block.

## Role focus today

- **pm-scrum-master** (always-on): synthesized the 30-PR delivery burst; confirmed DEC-001 unblock triggers fired (2B pipeline #463 + WS sync #472); flagged the #480–#487 follow-up test debt and the three-draft Epic 6 web-UI cluster.
- **pm-frontend** (rotation index 2): confirmed the document sharing/permission UI (web+mobile), OAuth admin/user-grants UI, MFA frontend+e2e, neighbor listing, and messaging screens all landed; surfaced the Epic 6 announcement-web-UI draft-sequencing risk and the #480–#487 test-gap follow-ups as the near-term frontend priorities; noted the ai.rs IDOR cluster for pm-backend ownership.
