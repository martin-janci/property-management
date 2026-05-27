# PPT Project State

_Generated: 2026-05-27 — daily PM rotation (Scrum Master + pm-devops). Coverage map last rebuilt by `/ppt-project-management scan` on 2026-05-23; upkeep-refreshed 2026-05-27 (epic-9 rotating re-check, idx 3)._

## Executive summary

- **1 PR merged since the last run — #555 (gap-80-3 mediation workspace UI)**, which landed the mediation timeline + resolution form + manager/tenant chat thread, **closed the 4 critical App.tsx dispute-route wiring gaps**, and added the missing `docs/screens/ppt/dispute-detail.md` mediation screen-map. +1612/-137 with tests. **Story 80-3 promoted partial → done.** (PRs #556–562 merged before the prior run and were already counted; #555 landed late despite its lower number.)
- **Story coverage now 23 done / 25 partial / 1 not-started (49 total)** — up one done from last run. 2 of 13 epics fully done (epic-8a, epic-9).
- **Mobile release pipeline has no merged path.** DevOps read: the EAS build workflows (`eas-build-android.yml` / `eas-build-ios.yml`) exist only in unmerged draft PRs (#566 cluster) and pin non-existent `@v6` actions; nothing in `.github/workflows/` can cut a mobile artifact today. Two blocked CI-fix items (gap-85-2 android + ios) must land together.
- **Six dispatcher drafts in flight (#563–#568), none merged.** #566 (esignature UI) and #567 (admin-health MFA fix) picked up reviewer **verdict=approve overnight** and are ready to promote out of draft.
- **Security-test-gate enforcement unconfirmed.** `security-test-gate.yml` exists, but after PR #497 shipped a security fix with zero tests it is unclear whether the gate is a required status check on `dev` or only advisory.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

| Epic | Tracked status | Real status (from coverage upkeep) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6-1/6-5/6-6 done; 6-2/6-3/6-4 web UI in dispatcher drafts, still `partial` |
| 7A — Basic Document Management | in-progress | 7a-3/7a-5 done; **7a-1 blocked** — backend POST `/documents/upload` missing |
| 8A — Basic Notification Preferences | done | 8a-1/8a-2 done; 8a-3 WS half done, mobile-push leg open |
| 9 — TOTP MFA | done | 9-1 done (re-checked 2026-05-27 — no regression; #567 is an admin-health modal fix, not a 9-1 regression) |
| 10A — OAuth Provider Foundation | in-progress | backend + admin/user-grants UI done; integration/security test gap remains |
| 10B — Platform Administration | in-progress | 3 done; 10b-3 admin-health MFA fix approved (#567); 10b-4/5/6/7 backend still partial |
| 80 — Dispute Resolution | in-progress | **80-3 done (#555)**; 80-2 filing-flow verify still open |
| 81 — Reports | in-progress | execution-history + schedule-edit retries queued; backend endpoints still absent |

## What's next (top 5)

1. **[high · pm-scrum-master]** Promote the two overnight-approved drafts and merge to dev: #566 (gap-84-2 esignature UI) + #567 (gap-10b-3 health UI MFA) — both verdict=approve; clears two medium queue items.
2. **[high · pm-backend]** Implement POST `/api/v1/documents/upload` backend handler — 7a-1 still not promotable; mobile upload UI calls a missing route.
3. **[high · pm-devops]** Land the two blocked EAS mobile CI fixes together (gap-85-2 android + ios) — downgrade `@v6`→`@v4` action pins, add eas-cli devDependency + missing Android npm scripts; mobile release pipeline is red until both merge.
4. **[medium · pm-devops]** Make `security-test-gate.yml` a required status check on `dev` (not advisory) so security-labelled PRs without a test file are blocked, per the PR #497 incident.
5. **[medium · pm-devops]** Enable a merge queue / auto-rebase ordering for App.tsx-touching PRs so the concurrent dispatcher draft cluster (#563–#568) does not triple-conflict on the router file.

See `roadmap.md` for the full ranked plan and `action-list.json`/`action-list.md` for the tracker view.

## Blockers

- **7a-1-document-upload-metadata** — backend POST `/documents/upload` absent; mobile upload UI calls a missing route. Owner: pm-backend.
- **Mobile EAS release pipeline** — build workflows exist only in draft PRs with broken `@v6` action pins; no merged mobile build path. Owner: pm-devops.
- **App.tsx churn cluster** — 6 concurrent dispatcher drafts (#563–#568) risk triple-conflict on the router file. Owner: pm-devops.

## Role focus today

Role focus today: pm-scrum-master, pm-devops.

- **pm-scrum-master:** 1 PR shipped (#555 → 80-3 done); two approved drafts (#566/#567) ready to promote; 3 blockers carried (7a-1 backend upload, mobile EAS pipeline, App.tsx churn).
- **pm-devops:** Mobile EAS release pipeline has no merged path (workflows only in draft PRs with broken `@v6` pins); security-test-gate enforcement unconfirmed after PR #497; App.tsx churn + 6 concurrent drafts need a merge queue.
