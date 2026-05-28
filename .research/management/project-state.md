# PPT Project State

_Generated: 2026-05-28 — daily PM rotation (Scrum Master + pm-data). Coverage `scan_kind=upkeep`; last deep scan 2026-05-23. Rotating-epic re-check this run: epic-80 (coverage_cursor idx 5 → 6/epic-81)._

## Executive summary

- **5 PRs merged this window (#635–#638, #642).** All from-merged-review / gap delivery: **#635** admin-web Support Data page (10b-5 frontend surface), **#636** 7 document-folder integration tests (CLOSES issue #580), **#637** HelpSidebar a11y (focus-trap, dialog role, tooltips) across 11 admin-web pages + tests, **#638** mobile EAS iOS CI config (removed submit.staging, guarded updates.url), **#642** auth.rs + sso.rs cookie-Path reconciliation + runbook **with inline tests**.
- **Story coverage advanced to 27 done / 21 partial / 1 not-started (49 total)** — up from 23 done. Four stories carried fresh `done` evidence: 7a-2 (folder web slice now tested), 10b-5 (support-data UI), 10b-7 (contextual-help UI), and 79-2's cookie-Path security leg. 2 of 13 epics fully done (epic-8a, epic-9).
- **#642 closed the #617 cookie-Path regression risk** — the last security blocker on 79-2 Authentication Flow; only the gap-79-2-auth-callback-e2e verification remains before 79-2 promotes done.
- **9 open PRs**, all draft, none reviewed yet: #597, #632, #633, #639, #640, #641, #643, #644, #645.
- **Issue #580 CLOSED** (resolved by #636). Follow-up close-rate improved this window: 5 queue items resolved against the from-merged-review backlog.
- **pm-data read (rotating role today):** the new Support Data page (#635) is the only data/analytics surface that moved. It exposes cross-tenant tenant diagnostics (user counts, active sessions, fault-status summary) with no per-view usage tracking, and a FaultStatusCount metric that overlaps owner_analytics / portfolio_performance fault KPIs — a metric-divergence + PII-access-traceability concern.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

| Epic | Tracked status | Real status (from coverage upkeep) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6-1/6-5/6-6 done; 6-2/6-3/6-4 web UI still `partial` (drafts) |
| 7A — Basic Document Management | in-progress | 7a-3/7a-5 done; 7a-2 folder web slice now tested (#636) but mobile slice open; 7a-1/7a-4 partial |
| 8A — Basic Notification Preferences | done | 8a-1/8a-2 done; 8a-3 WS half done, mobile-push leg open |
| 9 — TOTP MFA | done | 9-1 done |
| 10A — OAuth Provider Foundation | in-progress | backend + admin/user-grants UI done; integration/security test gap remains |
| 10B — Platform Administration | in-progress | 10b-5 + 10b-7 frontend surfaces shipped (#635/#637); 10b-4/6 still UI-light |
| 80 — Dispute Resolution | partial | 80-1/80-3 done; 80-2 filing-flow AC verify still open (epic-80 re-check this run — no change) |
| 81 — Reports | in-progress | editor + executions table landed; **blocked by authz follow-ups #614/#624 + backend pause/resume/download routes** |

## What's next (top 5)

1. **[high · pm-security/pm-backend]** Close report-schedule authz holes #614 (missing RBAC) + #624 (missing tenant/org scope) — cross-tenant IDOR class; add cross-tenant regression test. Blocks Epic 81 promotion.
2. **[high · pm-backend]** Implement/confirm POST `/api/v1/documents/upload` + 81 backend pause/resume + executions-download routes — 7a-1 / 81-1 / 81-2 not promotable until they land.
3. **[high · pm-qa]** Schedule gap-79-2-auth-callback-e2e — cookie-Path reconciled by #642 (with tests); e2e is the last gap before 79-2 promotes done.
4. **[medium · pm-data]** Define Support Data analytics/audit events + reconcile the FaultStatusCount metric definition with owner/portfolio fault KPIs.
5. **[medium · pm-scrum-master]** Slot remaining from-merged-review follow-ups (fix-569/573/574/581/583) into the dispatcher buffer with explicit owners; confirm close-rate now matches ingest.

See `roadmap.md` for the full ranked plan and `action-list.json`/`action-list.md` for the tracker view.

## Blockers

- **Reports authz (#614 + #624)** — `update_schedule` cross-tenant + missing-RBAC; Epic 81 cannot promote. Owner: pm-backend.
- **7a-1 / 81 backend endpoints** — upload + pause/resume/executions-download routes still to confirm; UI/tests outrun the backend. Owner: pm-backend.
- **Mobile lag** — 10 of 22 partial/not-started candidates are mobile (7a-2 folder, 7a-4 preview, 8a-3 push, epic-82 SwiftUI). Owner: pm-frontend.

## Role focus today

Role focus today: pm-scrum-master, pm-data.

- **pm-scrum-master:** 5 PRs shipped (#635–#638, #642); four stories advanced to done evidence; issue #580 closed; #617 cookie-Path regression resolved by #642; follow-up close-rate improved (5 queue items resolved).
- **pm-data:** only data surface this window is the #635 Support Data admin page — flags missing support-access usage tracking, a fault-status metric that must be unified with owner/portfolio KPIs, and a PII-access retention/traceability gap on the session/activity diagnostics.
