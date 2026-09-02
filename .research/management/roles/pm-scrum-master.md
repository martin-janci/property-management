# pm-scrum-master — 2026-09-02

_Always-on synthesis this run (routine Phase 1.6). Static read; no compile/run._

## Summary

Auto-review loop is still working: both follow-up issues opened this window (#2924 report_summary snapshot; #2923 update-error i18n) were **closed by merged PRs in the SAME window** (#2928, #2927). 5 PRs merged since last run — all correctness / hygiene, zero regressions. Delivery unchanged at 47/49 stories (84-1 direct-to-S3 wiring and 84-2 signer page still partial); none of the 5 merged PRs touched those surfaces.

## shipped_since_last_run

- **#2928** — fix(db): snapshot-consistent report_summary counts + entries (closes #2924)
- **#2927** — test(UC-62): cover update-error i18n path in EmergencyContactDirectoryPage (closes #2923)
- **#2925** — code-review compliance-raw-db-leak-regression: route audit-log count error through db_error (masks sqlx internals in 500 body)
- **#2926** — refactor(api-server): remove decommissioned FCM legacy send path (attack-surface reduction)
- **#2922** — refactor(reality-server): derive saved-search HTTP status from a typed error enum

## sprint_progress

`{"sprint":"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth","epics_done":3,"epics_total":5}` — unchanged.

## next_actions

1. **[high]** Ship 84-1 (ppt-web direct-to-S3 upload) — 5th consecutive upkeep window with no progress; API landed in #2309, no consumer. DoD: POST /api/v1/documents/upload-url consumer + regression test — **owner: pm-frontend**.
2. **[high]** Ship 84-2 (signer-facing document-sign page) — screen-map planned, API complete; paired with #1 closes MVP 49/49. DoD: buildStatus planned→shipped + signature-request email verified end-to-end — **owner: pm-frontend**.
3. **[high]** Resolve gh-issue-2797 (RUSTSEC-2026-0258 h2 empty-DATA-frame DoS) — 15+ days standing; blocks every backend PR through cargo-deny. DoD: h2 bumped, cargo-deny green — **owner: pm-security**.
4. **[high]** Unblock mobile-native/KMP cloud-runner builds (issue #2652) — 7/8 mobile-native items structurally unclaimable; dispatcher runs a Tier-1d generator kick every cycle to keep buffer above floor. DoD: cloud runner can build a KMP change end-to-end — **owner: pm-devops**.
5. **[medium]** Grep-audit for the raw-db-leak class fixed in #2925 across other routes' secondary DB calls (count / aggregate helpers) — the pattern in compliance.rs is likely mirrored in reports / audit / analytics helpers. DoD: audit note listing sites that still leak sqlx internals, or clean bill of health — **owner: pm-backend**.
6. **[medium]** Extend the reality-server typed-error-enum pattern from #2922 (saved_searches) to inquiries / favorites / reports routes — same class of `unwrap → 500` cliff on the same server. DoD: 1 more route migrated to a typed error enum this sprint — **owner: pm-backend**.

## risks

- **84-x partials now aging 5+ upkeep windows (high/medium):** 84-1 + 84-2 unchanged for 5 windows; MVP visibility risk. Mitigation: **promote directly from ranked backlog** rather than waiting on dispatcher spawn — both are frontend-only, no dependency chain — owner: pm-frontend / pm-scrum-master.
- **Cloud-buffer starvation is now chronic (high/medium):** every dispatcher commit since 2026-08-30 records `GC3-buffer-bounds=FAIL (record-only)` because open mobile-native items dominate the queue. Mitigation: fix #2652 OR split action-list into cloud + local buckets — owner: pm-devops.
- **Standing: RUSTSEC-2026-0258 h2 DoS (high/high):** blocks every backend PR. Mitigation: land the h2 bump — owner: pm-security.

## blockers

- **[Standing] gh-issue-2797 (cargo-deny RUSTSEC-2026-0258 h2 DoS)** — blocks every backend PR — owner: pm-security.
- **[Standing infra] issue #2652 (mobile-native/KMP in cloud runner)** — 7/8 mobile-native items unclaimable — owner: pm-devops.
- **[Aging] 84-1 + 84-2 partial** — 5 upkeep windows without dispatcher pick-up — owner: pm-frontend.
- **[Human PRs stalled >30d]** #2555 / #2558 / #2559 (accounting MVP trio — invoice PDF, PAY-by-square QR, lifecycle) — reviewer capacity issue (open risk `pm-scrum-master-accounting-mvp-trio-reviewer-starvation-2026-07-30`).

## open_questions

- Is anyone actively picking up the accounting MVP trio (#2555/#2558/#2559) or should they be paused / re-scoped?
- Should we split action-list.json into `queue-cloud.json` + `queue-local.json` to end the record-only GC3 failures?

## decisions_needed

- (none new this run)
