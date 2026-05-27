# pm-scrum-master — Delivery synthesis

_Ran: 2026-05-27 (always-on)_

## Summary

High-throughput window: **25 PRs merged (#563–#634)** — mostly gap-* feature delivery plus security/i18n fixes — alongside **~30 new post-merge follow-up issues (#569–#629)** and **12 open PRs** (mostly drafts). Delivery is healthy but the follow-up backlog is growing faster than it is being closed; three of the new follow-ups are security-relevant on the reports path (#614, #617, #624) and gate Epic 81 promotion.

## Shipped since last run

- **#565** — session-cookie scope hardening (P0-12 security).
- **#604** — FolderTree i18n (security-label) — advances 7a-2 folder organization (web slice).
- **#610** — document upload backend tests — closes the 7a-1 upload test-coverage gap.
- **#611** — `report_executions` table migration — backs 81-2 execution-history persistence.
- **#623** — report schedule editor UI — advances 81-1.
- **#607** — booking-push validation guards — advances 83-2.
- **#568 / #609** — SSO auth-callback wiring — advances 79-2 (consumer side).
- (+18 further gap-* / fix PRs across documents, disputes, reports, mobile announcements.)

## Sprint progress

- Sprint: **Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth**
- Epics fully done: **2 of 13** (epic-8a notifications, epic-9 MFA). Story-level: 23 done / 25 partial / 1 not-started (49 total). Several partials advanced toward done this window (7a-1, 7a-2, 79-2, 81-2) but none promoted — all carry verification or open security follow-ups.

## Next actions (top)

1. **[high · pm-backend]** Close report-schedule authz holes #614 (`update_schedule` missing RBAC) + #624 (missing tenant/org scope) together — cross-tenant IDOR class; blocks Epic 81 promotion.
2. **[high · pm-frontend]** Reconcile cookie `Path` breaking change (#617) from PR #565 — confirm no silent logout / broken SSO auth-callback cookie read.
3. **[high · pm-backend]** Implement/confirm POST `/api/v1/documents/upload` backend handler and pause/resume + executions download routes — 7a-1 and 81-1/81-2 still not promotable until backend endpoints land.

## Blockers

- **Reports authz (#614 + #624)** — `update_schedule` cross-tenant + missing-RBAC; Epic 81 cannot promote. Owner: pm-backend.
- **Cookie Path regression (#617)** — PR #565 cookie-scope change may drop sessions / break SSO callback. Owner: pm-frontend/pm-security.
- **Follow-up backlog growth** — ~30 new `from-merged-review` follow-ups this window vs few closed; risk of perpetual debt accrual on documents/disputes/reports hotspots. Owner: pm-scrum-master (triage cadence).
