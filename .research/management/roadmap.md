# PPT Project Roadmap

_Generated: 2026-06-18 (PM rotation: pm-scrum-master + pm-security) · supersedes `_bmad-output/implementation-artifacts/gap-analysis-remediation.md` (Epic 86, stale)._

_Upkeep 2026-06-18: 51 PRs merged this window (#1447–#1581). Heavy backend hardening wave — RLS coverage (#1460/#1473/#1492/#1495/#1499/#1551/#1563), OAuth/MFA (#1467/#1539/#1552/#1580), enum-cast bugfixes, IDOR regression seeds. **Story 7a-1 promoted to done (#1550 round-trip test).** Rotating epic refresh: **epic-85** (coverage_cursor idx 12 → wrap to 0/epic-6) — EAS workflow pins confirmed healthy (#1526) but workflow_dispatch green not yet recorded. New security headline: **#1538 confirms backend test job is NOT a required check on `dev`** — same structural cause as the #1426→#1437 dev-red incident._

## State of the project

- **Story coverage: 30 done / 19 partial / 0 not-started (49 total).** epic-7a moves to 1/5 (was 0/5) via 7a-1 done. epic-8a, epic-9 fully done. epic-10b 5/7.
- **Per-platform:** backend 22 done / 6 partial · frontend 15 done / 11 partial · mobile 4 done / 9 partial. Mobile remains the most-behind.
- **Top 3 gaps:**
  1. **CI gate (#1538)** — `dev` branch protection does not require `cargo test`; auth/RLS regressions can merge silently in active 8-PR churn on rental.rs/document.rs.
  2. **Mobile mvp slices** — 7a-2 folder mobile + 7a-4 mobile preview + 8a-3 FCM/APNs push; backend + web shipped, mobile lags.
  3. **OAuth/auth structural gaps** — #480 (WS JWT in query param), #481 (revoked-token bypass; partial backfill, no test), test-hardening batch #480-#487 still 7/8 open.
- **Screen coverage:** `scan_kind=upkeep` (no fresh screens cross-check); epic-82 remains only orphan-epic (no reality-mobile `docs/screens/`); 82.1 SwiftUI screen-map added via #1514 — partial progress.

## Ranked plan

### mvp

- [high] **Add `backend / test` as required status check on `dev`** — owner: pm-devops (filed by pm-security) — why: #1538 confirmed via branch-protection API; auth/RLS regression PRs can merge silently in current 8-PR rental.rs/document.rs churn cluster. Highest single delivery risk.
- [high] Close #481 OAuth refresh-token revocation bypass — owner: pm-backend (driven by pm-security) — why: session.rs:55 guard appears restored but no integration test asserts; revoked tokens may be silently usable.
- [high] Close #480 WebSocket JWT-in-query — owner: pm-backend (driven by pm-security) — why: JWT still travels in URL even though handler avoids logging it (access logs, proxy logs, Referer); migrate to OTT exchange.
- [high] Audit document.rs route handlers for deprecated non-RLS method calls; add `clippy -D deprecated` CI gate — owner: pm-backend (driven by pm-security) — why: 8-PR churn cluster + still-callable legacy methods = silent tenant-isolation bypass risk.
- [high] Resolve 7a-2 folder organization (PR #1316) — owner: pm-backend — why: stale in review; document.rs is hottest churn file (8 PRs in 48h), rebase debt growing daily.
- [high] Triage 6 untriaged issues #1520/#1521/#1522/#1532/#1533/#1538 — owner: pm-scrum-master — why: governance + sprint slot decision needed before next wave.
- [medium] Land 6-2/6-3/6-4 announcement web UI out of draft — owner: pm-frontend — why: backend + pipeline live; only web UI gates closure.
- [medium] Finish 7a-2 folder mobile slice + 7a-4 mobile preview — owner: pm-frontend — why: web+backend done; mobile is the only open gap.
- [high] Implement 8a-3 mobile OS push (FCM/APNs) — owner: pm-backend + pm-frontend — why: WS leg cleared (#1395), mobile-push is the only remaining leg before 8A→done.
- [medium] Close #486 (replace direct getToken() calls with axios-interceptor path) — owner: pm-frontend — why: unblocks 6-2 / 6-5 story gates.
- [medium] Audit churn cluster rental.rs / document.rs (8 PRs each) for missing integration-test coverage — owner: pm-qa — why: compounds the #1538 risk.

### phase2

- [medium] Verify 81-2 execution-history download/retry end-to-end — owner: pm-frontend — why: presigned + retry now landed (#1548); confirm UX path.
- [high] Audit + decide sqlx 0.8→0.9 (Dependabot) before merge — owner: pm-backend — why: workspace-wide query!/migrate breakage risk.
- [medium] Re-audit 5 IDOR backlog signals (equipment / voice-device / llm-doc / reality-inquiry / reserve-funds) — owner: pm-security — why: merge status unconfirmed; close #483.

### phase4

- [low] SwiftUI Reality Portal epic-82 (home/search, listing-detail, inquiries, favorites) — owner: pm-frontend — why: phase4 partials advancing (#1514/#1516/#1541), continue.
- [low] Airbnb/Booking.com channel sync backends (83-1/83-2) — owner: pm-backend — why: 83-2 advanced via #1420; 83-1 still ahead.

#### Screen-map drift

- [low] Add remaining screen-map(s) for orphan epic epic-82 (reality-mobile) — owner: pm-frontend — why: 82.1 landed via #1514; the rest still missing.

Buffer: 24/36 open · pm-analysis 2026-06-18 added 11 (6 pm-security + 5 scrum-master) — dispatcher already maintains a wider candidate pool via tier-1 refill; current open count is intentional after dispatcher reconcile this run.
