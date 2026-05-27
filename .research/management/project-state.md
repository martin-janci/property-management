# PPT Project State

_Generated: 2026-05-27 — daily PM rotation (Scrum Master + pm-security). Coverage map last rebuilt by `/ppt-project-management scan` on 2026-05-23; upkeep-refreshed 2026-05-27 (epic-10a rotating re-check, idx 4; cursor → 5/epic-10b)._

## Executive summary

- **25 PRs merged this window (#563–#634)** — mostly gap-* feature delivery plus security/i18n fixes. Notable: **#565** session-cookie scope hardening (P0-12), **#604** FolderTree i18n security-label, **#610** document-upload backend tests, **#611** `report_executions` table, **#623** report schedule editor UI, **#607** booking-push validation guards, **#568/#609** SSO auth-callback wiring.
- **~30 new follow-up issues (#569–#629)**, all `follow-up`/`from-merged-review`. Several open security-relevant: **#614** (`update_schedule` missing RBAC), **#617** (cookie `Path` breaking change), **#624** (`update_schedule` missing tenant/org scope). The follow-up backlog is growing faster than it is being closed.
- **Story coverage holds at 23 done / 25 partial / 1 not-started (49 total)** — several partials advanced toward done (7a-1 upload tests, 7a-2 folder web UI, 79-2 auth-callback, 81-2 executions table) but none promoted; each carries verification or an open security follow-up. 2 of 13 epics fully done (epic-8a, epic-9).
- **pm-security read (rotating role today):** authz/tenant-scoping is the dominant risk surface. #614 + #624 are a cross-tenant IDOR-class hole on the report-schedule path (same omission class as the prior ai.rs equipment IDOR) and gate Epic 81. #617 is a release-blocking cookie-scope regression from the #565 hardening. OAuth provider (10a) still ships with no introspection/refresh-rotation/PKCE security tests.
- **12 open PRs**, mostly drafts (#630, #632, #633, #635–#639) plus non-draft #533, #542, #558.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

| Epic | Tracked status | Real status (from coverage upkeep) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6-1/6-5/6-6 done; 6-2/6-3/6-4 web UI still `partial` |
| 7A — Basic Document Management | in-progress | 7a-3/7a-5 done; 7a-1 upload tests landed (#610) but still `partial`; 7a-2 folder web UI landed (#604), mobile slice open |
| 8A — Basic Notification Preferences | done | 8a-1/8a-2 done; 8a-3 WS half done, mobile-push leg open |
| 9 — TOTP MFA | done | 9-1 done (no regression this window) |
| 10A — OAuth Provider Foundation | in-progress | backend + admin/user-grants UI done; **integration/security test gap remains** (pm-security flag) |
| 10B — Platform Administration | in-progress | 3 done; 10b-4/5/6/7 backend still partial (no UI) |
| 80 — Dispute Resolution | in-progress | 80-3 done (#555); 80-2 filing-flow verify still open |
| 81 — Reports | in-progress | execution-history table (#611) + schedule editor (#623) landed; **blocked by authz follow-ups #614/#624 + missing backend pause/resume/download routes** |

## What's next (top 5)

1. **[high · pm-backend]** Close report-schedule authz holes #614 (`update_schedule` missing RBAC) + #624 (missing tenant/org scope) together — cross-tenant IDOR class; add a cross-tenant regression test. Blocks Epic 81 promotion.
2. **[high · pm-frontend]** Reconcile cookie `Path` breaking change (#617) from PR #565 — confirm no silent logout and that the SSO `/auth/callback` cookie read still works.
3. **[high · pm-backend]** Implement/confirm POST `/api/v1/documents/upload` and the 81 backend pause/resume + executions-download routes — 7a-1 / 81-1 / 81-2 not promotable until they land.
4. **[medium · pm-qa]** Land OAuth provider security tests (Epic 10a): revoked-token rejection, refresh-rotation family-reuse block, PKCE S256 enforcement.
5. **[medium · pm-scrum-master]** Triage the ~30 new `from-merged-review` follow-ups (#569–#629) into the gap buffer / close cadence — backlog is accruing faster than it drains.

See `roadmap.md` for the full ranked plan and `action-list.json`/`action-list.md` for the tracker view.

## Blockers

- **Reports authz (#614 + #624)** — `update_schedule` cross-tenant + missing-RBAC; Epic 81 cannot promote. Owner: pm-backend.
- **Cookie Path regression (#617)** — PR #565 cookie-scope change may drop live sessions / break SSO callback. Owner: pm-frontend / pm-security.
- **7a-1 / 81 backend endpoints** — upload + pause/resume/executions-download routes still to confirm; UI/tests outrun the backend. Owner: pm-backend.
- **Follow-up backlog growth** — ~30 new review follow-ups this window vs few closed. Owner: pm-scrum-master.

## Role focus today

Role focus today: pm-scrum-master, pm-security.

- **pm-scrum-master:** 25 PRs shipped (#563–#634); no story promotions (all partials carry verify/security follow-ups); ~30 new follow-ups accruing; 4 blockers carried.
- **pm-security:** authz/tenant-scoping is the top risk surface — #614 + #624 cross-tenant report-schedule IDOR (Epic 81 blocker), #617 cookie-Path regression from #565, residual P1-04 audit-hash Debug-format leak, and untested OAuth 10a security contract.
