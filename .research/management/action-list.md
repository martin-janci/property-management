# Action list

_Generated 2026-05-25 from `action-list.json` (merged: pm-analysis rotation + gap-scan + code-review-finding). 36 open · 4 in-progress · 23 done._

## New this run (2026-05-25 — pm-frontend rotation + code review)

| Priority | Action | Owner | Status |
|---|---|---|---|
| high | Fix cross-tenant IDOR cluster in ai.rs equipment endpoints (update/delete equipment + update_maintenance discard `_principal`) — scope by tenant + regression test | pm-backend | open |
| high | Sequence + land Epic 6 announcement web UI draft PRs in order #474 → #475 → #479 | pm-frontend | open |
| medium | Slot follow-up issues #480-#487 (test/security/UX gaps on merged PRs) into a single hardening batch | pm-frontend | open |
| medium | Add ai.rs/platform_admin.rs/announcements.rs to module-split backlog + CI lint for discarded `_principal` on mutating handlers | pm-tech-lead | open |
| medium | Verify Epic 81 schedule/execution e2e as PRs #488/#489 land; promote 81-1/81-2 from partial | pm-frontend | open |

## Resolved this run (merged PRs #441–#473)

| Action | Owner | Resolved by |
|---|---|---|
| MFA frontend integration + e2e (9-1) | pm-security | PR #441, #473 |
| Mobile document upload (7a-1), share sheet (7a-5), permission UI (7a-3) | pm-frontend | PRs #447 / #445 / #462,#465 |
| Web document sharing UI (7a-5) | pm-frontend | PRs #451, #467 |
| Direct messaging apiStatus → integrated (6-5) | pm-frontend | PRs #449, #472 |
| Neighbor listing privacy-aware UI (6-6) | pm-frontend | PR #464 |
| OAuth client admin UI (10a-2) + user-grants UI (10a-3) | pm-frontend | PRs #468/#469/#471 |
| Epic 2B notification pipeline + WebSocket realtime sync | pm-backend | PRs #463, #472 |
| P1-05 SSRF outbound URL validation | pm-backend | PR #450 |
| ProtectedRoute fail-open role guard | pm-frontend | PR #459 |
| Voice device IDOR | pm-security | PR #461 |

## Still open — top carryovers

| Priority | Action | Owner | Status |
|---|---|---|---|
| high | Resolve residual #438/#439 security findings (cookie scope, Debug-hash, ordering, IG3) | pm-security | open |
| high | Implement Epic 81 report-schedule backend endpoints (pause/resume/executions) | pm-backend | open |
| high | Mobile push leg (FCM/APNs) for 8a-3 — WS half now done | pm-backend | open |
| high | Folder-tree UI page (7a-2) | pm-frontend | open |
| high | Wire AnnouncementsPage/FaultsPage to API hooks (79-1) | pm-frontend | open |
| high | OAuth integration test suite (10a-1) | pm-backend | open |
| high | Admin health monitoring UI (10b-3) | pm-frontend | open |
| medium | 10b-4/5/6/7 handler bodies (return 501 until real) | pm-backend | open |
| medium | Dispute filing + mediation AC sweep (80-2/80-3) | pm-frontend | open |
| medium | Split churn-hot route modules (integrations/organizations/documents.rs) | pm-tech-lead | open |

_See `action-list.json` for the full 63-item tracker (36 open)._
