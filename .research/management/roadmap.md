# PPT Project Roadmap (Deep Scan)

_Generated: 2026-05-29 (PM rotation: pm-scrum-master + pm-integration) · supersedes `_bmad-output/implementation-artifacts/gap-analysis-remediation.md` (Epic 86, stale)._

_Upkeep 2026-05-29: 11 PRs merged this window (#717–#730) + late-merges below the prior #709 cursor (#597/#657/#659/#685/#695/#706). App-code slice small: #718 iOS gesture/sheet/SSO-CSRF-tests, #719 gap-84-2 e-signature signerParties (resolves all 6 PR#513 follow-ups), #720 gap-10b-3 admin-health MFA test coverage, #724 gap-10a-4 OAuth scope picker. 84-2 e-signature moved not-started → partial. Rotating epic re-checked: **epic-81** (coverage_cursor idx 6 → 7/epic-82) — #643 closed report-schedule RBAC #614 + tenant-scope #624; remaining 81 gap is the missing cron_expression column (#616) + exec-history download/retry e2e. New security finding from the api-core Rust review: cross-tenant IDOR in the Epic-64 LLM-document handlers (`security-llm-doc-idor`, promoted). Coverage `scan_kind=upkeep` — no fresh screens cross-check this run._

## State of the project

- **Story coverage: 27 done / 22 partial / 0 not-started (49 total).** 2 of 13 epics fully done (epic-8a, epic-9); epic-10b complete in coverage (7/7). 84-2 e-signature email advanced not-started → partial (#719).
- **Candidates: 22 partial** — mostly mobile + reports backend. **10 of 22 candidates are mobile** — mobile remains the most-behind platform.
- **Biggest gaps:**
  1. Mobile slices for mvp document/notification stories (7a-2 folder mobile, 7a-4 mobile preview, 8a-3 FCM/APNs push) — backend + web shipped, mobile lags.
  2. Epic 81 reports: cron_expression column missing (#616) blocks 81-1; exec-history download/retry e2e for 81-2. (RBAC/tenant-scope #614/#624 now closed by #643.)
  3. SwiftUI Reality Portal (epic-82, all phase4) — 5 stories `partial`, no screen-maps; drafts #639/#641/#705 in flight.
- **Screen coverage:** coverage `scan_kind=upkeep` (no fresh screens cross-check this run); epic-82 remains the only orphan-epic (no reality-mobile `docs/screens/` maps). 0 new orphan epics · 0 new orphan screens this upkeep.

## Ranked plan

### mvp

- [high] Land `security-llm-doc-idor` — owner: pm-security/pm-backend — why: state-mutating cross-tenant IDOR (publish/list/get LLM-document handlers in ai.rs); promoted plan this run.
- [high] Review + merge #662 (reports cross-tenant IDOR, closes #646/#647) — owner: pm-security — why: unblocks Epic 81 authz promotion.
- [high] Resolve #725 verdict=changes (ai-maintenance/session/sentiment IDOR + missing test) — owner: pm-security — why: closes the maintenance IDOR vector.
- [high] Implement/confirm POST `/api/v1/documents/upload` + 81 backend pause/resume/executions-download routes — owner: pm-backend — why: 7a-1/81-1/81-2 not promotable until backend lands.
- [medium] Land 6-2/6-3/6-4 announcement web UI out of draft (#474→#475→#479 order) — owner: pm-frontend — why: backend + pipeline live; only web UI gates closure.
- [medium] Finish 7a-2 folder mobile slice + 7a-4 mobile preview — owner: pm-frontend — why: web+backend done & tested (#636); mobile is the only open gap.
- [high] Implement 8a-3 mobile OS push (FCM/APNs) — owner: pm-backend — why: WS leg confirmed (#597); mobile-push is the only remaining leg.
- [medium] Sync sprint-status.yaml to coverage reality (10b done, 8a-3 WS done) — owner: pm-scrum-master — why: stale tracked status risks duplicated work / mis-reporting.

### phase2

- [high] Add report_schedules.cron_expression column (SQLx migration) + rewrite update_schedule to the documented UPDATE — owner: pm-backend — why: cron edits currently round-trip through the overloaded `time` field (backlog bug-report-schedule-update-no-sql / #616); gates 81-1.
- [medium] Verify 81-2 execution-history download/retry end-to-end — owner: pm-frontend — why: `report_executions` table + API landed (#611); confirm presigned download + retry.
- [high] Audit + decide sqlx 0.8→0.9 (Dependabot PR #666) before merge — owner: pm-backend — why: workspace-wide query!/migrate breakage risk.

### phase4

- [low] SwiftUI Reality Portal epic-82 (home/search, listing-detail, inquiries) — owner: pm-frontend — why: phase4 partials; drafts #639/#641/#705 in flight; no screen-maps.
- [low] Airbnb/Booking.com channel sync backends (83-1/83-2) — owner: pm-backend — why: models only, no OAuth/OTA transport; add Airbnb webhook event_id dedup (backlog bug-webhook-airbnb-dup-sync-jobs).

#### Screen-map drift

- [low] Add screen-map(s) for orphan epic epic-82 (reality-mobile) — owner: pm-frontend — why: SwiftUI screens have no `docs/screens/` entries.

Buffer: 135/36 open · 0 candidates ranked but unqueued (action-list already well above buffer; merge-only this run)
