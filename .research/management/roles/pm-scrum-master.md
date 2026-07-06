# pm-scrum-master — 2026-07-06

**Summary.** No sprint-epic story work merged this window (all 21 merged PRs were research/infra/security/refactor track); the sprint spine is significantly stale against `coverage.json` — epic-10a (OAuth, 0/3 in spine) and epic-10b are fully done in code, and 7a-3/7a-4 are code-complete but unflipped, while epic-7a is genuinely blocked (7a-2 CI red) and two security-relevant draft PRs have stalled 12–13 days.

## Shipped since last run

- **#2120** — security-fix: authorize outage mutations on DB role, not JWT claim
- **#2118** — mobile RN Meters/Leases/Forms/Threads screens wired to api-server (clears backlog `code-review-mobile-rn-screens-mock-data`)
- **#2115** — fix: ListingDetail auth change resets screen to spinner (reality-mobile)
- **#2100** — refactor: rental repository split into cohesive sub-modules
- **#2097** — test: un-quarantine #1771 messaging soft-delete unread invariant
- **#1979** — test: fix outages happy-path auth (BIT-414)
- 16 additional infra/test/refactor/docs/dependabot PRs — none map to sprint epic stories

## Next actions

| Priority | Action | Dep | DoD |
|---|---|---|---|
| high | Reconcile `sprint-status.yaml` against `coverage.json`: flip epic-10a (10a-1/2/3), epic-10b, 7a-3, 7a-4 to done with evidence citations | none | Spine matches coverage.json 2026-07-02 evidence; test-hardening batch gates re-evaluated against real state |
| high | Confirm closure of test-hardening batch issues #481 (OAuth refresh-token revocation bypass) and #487 (MFA rate-limit test gap) — both still gate 10a-1/10a-3 in sprint-status.yaml | pm-security | #481/#487 closed or explicitly deferred with linked evidence; story_gate entries updated |
| high | Fix CI red on `document_folder_tests` (PR #1316) blocking 7a-2 promotion from review to done | pm-backend | CI green; 7a-2 flips to done in both spine and coverage |
| high | Get reviewer assigned to draft PR #1797 — 13 days stale with no gate tracking it | pm-security | PR out of draft, reviewed, merged or closed with rationale |
| medium | Close #484 (serial dispatch_to_users + FCM stub swallowing failures) to fully clear 8a-3 from partial | rust-backend | pipeline dispatch made concurrent/failure-surfacing + regression test; #484 closed |
| medium | Implement missing POST `/reports/schedules` create route (81-1 gap: "Create new schedule" unimplemented) | pm-backend | create-schedule endpoint shipped + frontend wired; screen-map ppt/reports apiStatus flips to complete |

## Risks

- **[high/medium]** Sprint spine out of sync with actual shipped state across 2+ epics — misreports stakeholder progress and may block already-done stories on stale gates.
- **[medium/high]** Security-relevant PR #1797 stalled 13 days in draft with no active owner.
- **[medium/high]** OAuth epic being treated as fully done while #481 (RFC 9700 revocation) and #487 (MFA rate-limit) may still be open.
- **[medium/medium]** 7a-2 stuck indefinitely on red CI (PR #1316), holding epic-7a below completion.
- **[medium/medium]** No sprint-story PRs landed this window — all velocity went to research/infra track.

## Blockers

- **7a-2-folder-organization** — CI red on `document_folder_tests` (PR #1316) — owner: pm-backend
- **PR #1797** — draft, 13 days stale, no reviewer — owner: pm-security
- **PR #1812** — draft, 12 days stale, `needs-human-review` — owner: pm-tech-lead
- **sprint-status.yaml sync** — spine not reconciled with coverage.json — owner: pm-scrum-master
- **80-2-dispute-filing-flow** — blocked on AC-4 draft auto-save + 5-step wizard redesign — owner: pm-frontend

## Sprint progress

- Sprint: "Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"
- Epics done (spine): **1/6**. Coverage shows more done but spine not reconciled.
