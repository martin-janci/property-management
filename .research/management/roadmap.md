# PPT Project Roadmap (Deep Scan)

_Generated: 2026-05-28 (PM rotation: pm-scrum-master + pm-data) · supersedes `_bmad-output/implementation-artifacts/gap-analysis-remediation.md` (Epic 86, stale)._

_Upkeep 2026-05-28: 5 PRs merged since last run (#635–#638, #642) — admin-web Support Data page (#635), document-folder integration tests closing #580 (#636), HelpSidebar a11y across 11 admin-web pages (#637), mobile EAS iOS CI config (#638), cookie-Path reconciliation with tests (#642). Four coverage stories advanced to fresh `done` evidence (7a-2, 10b-5, 10b-7, 79-2-security-leg). Rotating epic re-checked: **epic-80** (coverage_cursor idx 5 → 6/epic-81) — disputes hooks/pages/screen-maps unchanged; 80-1/80-3 done, 80-2 still partial (AC verification pending). Coverage `scan_kind=upkeep` — no fresh screens cross-check this run._

## State of the project

- **Story coverage: 27 done / 21 partial / 1 not-started (49 total).** 2 of 13 epics fully done (epic-8a, epic-9). Up from 23 done last run — four stories carried fresh `done` evidence from this window's merges (7a-2 folder tests #636, 10b-5 support-data UI #635, 10b-7 help-sidebar #637, 79-2 cookie-Path #642).
- **Candidates: 22 partial/not-started** — 12 mvp, 3 phase2, 7 phase4. **10 of 22 candidates are mobile** — mobile remains the most-behind platform.
- **Biggest gaps:**
  1. Mobile slices for mvp document/notification stories (7a-2 folder mobile, 7a-4 mobile preview, 8a-3 FCM/APNs push) — backend + web shipped, mobile lags.
  2. Auth/reports backend authz follow-ups (#614/#624 report-schedule RBAC + tenant scope) still gate Epic 81; 79-2 auth-flow now only needs e2e after #642.
  3. SwiftUI Reality Portal (epic-82, all phase4) — 5 stories `partial/low-confidence`, no screen-maps.
- **Screen coverage:** coverage `scan_kind=upkeep` (no fresh screens cross-check this run); last deep scan reported epic-82 as the only orphan-epic (no `docs/screens/` reality-mobile maps). 0 new orphan epics · 0 new orphan screens this upkeep.

## Ranked plan

### mvp

- [high] Close report-schedule authz holes #614 (missing RBAC) + #624 (missing tenant/org scope) — owner: pm-security/pm-backend — why: cross-tenant IDOR class, gates Epic 81 promotion.
- [high] Implement/confirm POST `/api/v1/documents/upload` + 81 backend pause/resume/executions-download routes — owner: pm-backend — why: 7a-1/81-1/81-2 not promotable until backend lands; UI/tests outrun backend.
- [high] Schedule gap-79-2-auth-callback-e2e — owner: pm-qa — why: cookie-Path reconciled by #642 (with tests); e2e is the last gap before 79-2 promotes done.
- [medium] Land 6-2/6-3/6-4 announcement web UI out of draft (#474→#475→#479 order) — owner: pm-frontend — why: backend + pipeline live; only web UI gates closure.
- [medium] Finish 7a-2 folder mobile slice + 7a-4 mobile preview — owner: pm-frontend — why: web+backend done & now tested (#636); mobile is the only open gap.
- [medium] Define Support Data analytics events + reconcile FaultStatusCount KPI definition — owner: pm-data — why: new #635 admin surface reads cross-tenant diagnostics with no usage tracking and a fault-count metric that must match the owner/portfolio dashboards.
- [high] Implement 8a-3 mobile OS push (FCM/APNs) — owner: pm-backend — why: WS half done; mobile-push is the only leg before promotion.

### phase2

- [medium] Verify 81-2 execution-history download/retry end-to-end — owner: pm-frontend — why: `report_executions` table landed (#611); confirm presigned download + retry.
- [medium] Implement 84-2 e-signature email workflow UI — owner: pm-frontend — why: backend HMAC provider merged (#495); request/track UI missing.

### phase4

- [low] SwiftUI Reality Portal epic-82 (home/search, listing-detail, inquiries) — owner: pm-frontend — why: phase4, low-confidence partials, no screen-maps; lowest leverage now.
- [low] Airbnb/Booking.com channel sync backends (83-1/83-2) — owner: pm-backend — why: models only, no OAuth/OTA transport; phase4.

#### Screen-map drift

- [low] Add screen-map(s) for orphan epic epic-82 (reality-mobile) — owner: pm-frontend — why: SwiftUI screens have no `docs/screens/` entries (flagged in last deep scan).

Buffer: 102/36 open · 0 candidates ranked but unqueued (action-list already well above buffer; merge-only this run)
