# PPT Project Roadmap (Deep Scan)

_Generated: 2026-06-19 04:45 UTC · coverage-refresh upkeep run driven by the routine fire payload "buffer-low + coverage-stale" (claimable=2/72, both already-shipped). Cross-matched 20 partial stories against 113 PRs merged in the 3-day window 2026-06-16→2026-06-19; verified airbnb integration + cron_expression column directly against dev codebase._

## State of the project

- **Story coverage: 45 done / 4 partial / 0 not-started (49 total, 13 epics).** This run advanced 16 stories partial → done (6-3, 6-4, 7a-1, 7a-2, 7a-4, 8a-3, 79-1, 80-2, 81-1, 81-2, 82-1, 82-2, 82-4, 83-1, 83-2, 84-2). Epic-8a is fully done. Epic-83 (Airbnb + Booking integrations) is fully done.
- **Candidates: 4 partial** —
  1. **6-2 announcement viewing/acknowledgment** — web UI never landed (draft PRs #474/#475/#479 sat unmerged; mobile shipped long ago). New action item filed.
  2. **82-3 home/search screens** — only AC-4 (infinite scroll) was evidenced via #1447; debounced search (AC-2), CoreLocation integration, and FilterSheet residuals remain.
  3. **85-1 environment variables** — react-native-config / Metro setup + iOS Info.plist keys + ENV.md docs not yet done.
  4. **85-2 build configuration (app icons)** — PR #1554 is an open draft; the auto-impl wrote the generator script + SVG source + Contents.json manifests but binary PNGs cannot be committed via the GitHub API; maintainer must run the script and commit.
- **Action-list reconciliation:** 8 of 9 prior open items have been silently shipped on `dev` and are now `status: done` with PR or dev-codebase evidence; 3 new action items added for the real remaining work; net **4 open** (down from 9).
- **Screen coverage:** no fresh screens cross-check this run (Phase 1.5/1.6 skipped). 2 new `screen-map-drift` signals filed against PRs #1559, #1555 (heuristic — may be false positive).

## Ranked plan

### mvp

- [high] **Restore the cloud routine scheduler.** Owner: operator. Why: 3-day gap (2026-06-17/18/19 missed). All coverage upkeep + dispatcher refill stalled in the window. Issue #1604 (dispatcher backlog exhausted) is downstream.
- [high] **Make backend `test` job a required check on `dev`** (issue #1538). Owner: pm-devops. Why: red-test PRs are merging freely and re-redding the merge pipeline. Tracked as backlog `issue-1538-backend-test-not-required-check` (score 2).
- [medium] **Land 6-2 announcement viewing/acknowledgment web UI on ppt-web** — owner: pm-frontend. Why: mobile shipped; backend routes (mark_read/acknowledge/get_acknowledgments) live; only web side remains for full Story 6.2 promotion. New action: `feat-announcement-viewing-acknowledgment-web-ui-frontend`.
- [medium] **Close 82-3 home/search residuals** (debounced search AC-2 + CoreLocation + FilterSheet) on reality-mobile/KMP — owner: pm-frontend. Why: PR #1447 closed only AC-4. Coordinate with backlog row `code-review-mobile-native-kmp-search-stale-response-race`. New action: `feat-home-search-screens-debounced-search-corelocation-mobile`.
- [medium] **Drive 6 ready plans through the implementer** (`security-llm-doc-idor`, `bug-ios-searchview-uncompilable`, `code-review-reality-web-share-comparison-404`, `code-review-reality-web-listing-page-ssr-crash`, `code-review-reality-web-realtor-mgmt-untranslated`, `code-review-mobile-rn-report-fault-fake-submit`) — owner: dispatcher. Why: all sit at `status: ready` for 3–18 days; the dispatcher should pick these up next over any synthetic gap.

### phase2

- [low] **Mobile env-var setup (85-1).** Owner: pm-frontend. Why: react-native-config / Metro / iOS Info.plist + ENV.md docs. Low priority — Expo Constants already covers the runtime path; this is documentation + iOS-Info.plist parity.
- [low] **App-icon generation for RN mobile (85-2).** Owner: maintainer (manual PNG commit). Why: PR #1554 has all scaffolding; only binary commit remains. Cannot be auto-completed.

### phase4

- [low] **Screen-map drift triage (2 new signals from this run, PR #1559 + #1555).** Owner: pm-frontend. Why: both heuristic-medium-confidence; may be false positives. Read the actual route changes vs `docs/screens/ppt/` before promoting.

Buffer: **4/9 open action items, all real-work** (vs prior run's 9/9 with 8 already-shipped phantoms). 6 ready backlog plans queued for the implementer. Next routine run can rotate Phase 1.6 PM role (pm-security at idx 5) + Phase 1.5 code-review (oldest unreviewed segment `mobile-native-kmp`, 2026-06-06).
