# PPT Project State

_Generated: 2026-06-06 — daily PM rotation (Scrum Master + pm-backend; lightweight in-routine refresh). Coverage `scan_kind=upkeep`; coverage_cursor idx 8 → 9 (rotating-epic re-check)._ 

## Executive summary

- **26 PRs merged this window (#1107–#1135; gap on #1126/#1134 still open).** The batch was overwhelmingly the dispatcher's auto-impl pipeline landing churn-reduction refactors (oauth.rs services+routes #1132/#1133, App.tsx route extraction #1108/#1131, module-splits #1109/#1110/#1114, evidence-uploader+esignature+auth-refresh test stabilization #1116/#1119/#1113) and gap stories (gap-82-3 KMP search #1125, gap-82-4 KMP listing-detail favorites #1121, gap-82-2 reality-mobile nav-state #1124, gap-85-2 mobile build scripts #1112, story 6-2 announcement-ack coverage #1122, dx-push-fanout-blpop-drain #1115). Issue #1137 (PKCE test became a tautology after #1132 DRY refactor) is the only new follow-up surfaced.
- **Backlog churn: 25 items resolved → done (PR-body verbatim matches), 6 new items added (4 code-review findings from mobile-native-kmp segment + 2 cumulative churn-hotspot rows). Open queue: 44 → 25.** No decay this run (all open items <14d old).
- **Code-review slice — `mobile-native-kmp` segment (oldest unreviewed + churn-aligned):** three findings. (1) `SearchScreen.kt` stale-response race — overlapping filter/debounce/onSelect launches can clobber a newer page-1 with an older slower response. (2) `DeepLinkRouter.queryParam` skips URL-decoding while `MainActivity.handleDeepLink` calls Android `Uri.getQueryParameter` (which DOES decode) — SSO tokens diverge per platform. (3) `MainActivity` reimplements deep-link dispatch instead of routing through the shared `DeepLinkRouter` — drift trap for future targets.
- **Open PRs (24 total, 6 draft):** no stalled PRs (oldest is ~1 day old; the queue is young — all post-2026-05-30). Active drafts: #1136 (oauth integration test helper consolidation), #1140 (#755 Epic 8A RLS GUC fix), #1139 (#1014 dispatcher action-list.json size bound), #1138 (story 79-1 api-client integration), #1126 (gap-82-5 reality mobile inquiries + account), #1123 (gap-82-1 reality iOS XcodeGen).
- **PR #1118 anomaly (noted, not actioned):** the `refactor-mediation-duplicated-spinner` squash on `dev` shows 3,132 files changed / 898K insertions; the legitimate refactor is one Spinner component extract. Confirmed via per-commit numstat and excluded from churn detection this run. Worth a manual audit of the squash to confirm it didn't accidentally re-import vendored files; tag for next dispatcher pass.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

| Epic | Tracked status | Real status (from coverage upkeep) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6-1/6-2/6-5/6-6 done; 6-3/6-4 web UI partial |
| 7A — Basic Document Management | in-progress | 7a-3/7a-5 done; 7a-2 folder web tested, mobile slice open; 7a-1/7a-4 partial |
| 8A — Basic Notification Preferences | done | 8a-1/8a-2 done; 8a-3 WS leg confirmed; mobile-push leg open → partial; #755 RLS GUC mismatch in flight via draft PR #1140 |
| 10A — OAuth Provider | in-progress | services+routes oauth.rs deduped this window (#1132/#1133); #1137 (PKCE test tautology) is the only carry-over |
| 10B — Platform Admin | ready-for-dev (STALE) | 10b-1..10b-7 all done in coverage; sprint-status.yaml still needs sync |
| 79 — Frontend Integration | in-progress | 79-1 api-client coverage in flight (draft #1138); 79-2 auth-refresh churn refactored this window (#1113) |
| 81 — Reports | in-progress | 81-1/81-2 partial; cron_expression column (#616) still gates promotion |
| 82 — SwiftUI Reality Portal | in-progress | 82-2/82-3/82-4 landed this window (#1124/#1125/#1121); 82-1 iOS XcodeGen in draft #1123; 82-5 inquiries+account in draft #1126 |
| 84 — Advanced Features | in-progress | 84-1/84-3/84-4/84-5 done; 84-2 e-signature webhook tests stabilized (#1119) |
| 85 — Mobile Build Pipeline | in-progress | 85-2 DEV/STG icon variants shipped (#1112) |

## What's next (top actions)

1. **[high] Land draft #1140 (Epic 8A notification RLS GUC fix, #755 findings #4 & #5)** — owner: pm-security/pm-backend — closes the RLS GUC mismatch on `notification_preferences` + `critical_notifications`.
2. **[high] Land draft #1136 (oauth_integration_tests churn consolidation) + fix #1137 tautology** — owner: pm-backend — keep the OAuth slice converged after #1132/#1133 dedup.
3. **[high] Promote `bug-schema-drift-runtime-sql-issue-1008`** (score 3, vector bug, confidence high) — owner: pm-backend — generate `.sqlx` offline metadata and audit hand-written query strings in voting/messaging/notification paths.
4. **[medium] Triage the 3 new code-review findings on mobile-native-kmp** (search stale-response race, deep-link URL-decoding mismatch, MainActivity bypass of shared router) — owner: pm-frontend (KMP lead).
5. **[medium] Bound action-list.json size (#1014 / draft #1139)** — owner: pm-devops — prevent MCP-push corruption from large JSON.
6. **[low] Audit the PR #1118 squash anomaly** — owner: pm-scrum-master — confirm the 3,132-file squash didn't accidentally re-import third-party content into `dev`.

## Blockers

- **Epic 81 — Reports promotion:** `cron_expression` column still missing (#616 / backlog `bug-report-schedule-update-no-sql`); 81-1/81-2 stay partial.
- **OAuth production readiness (Epic 10A):** refresh-token revocation bypass (#481, high) and JWT-in-WS-logs (#480, high) remain open; gate any external OAuth exposure.
- **Mobile coverage lag:** reality mobile inquiries (#1126) + iOS XcodeGen (#1123) still in draft; gap-82 set not yet fully landed.

## Role focus today

- **Role focus today:** Scrum Master + pm-backend.
- **pm-backend read:** the backend surface saw three behavior-preserving refactors land in OAuth (services/oauth.rs DRY consolidation #1132, routes/oauth.rs churn reduction #1133, announcements.rs per-surface module split #1110, platform_admin.rs by-concern split #1109, ai.rs module split #1114, main.rs router/middleware extraction #1120) and one new runtime/Redis dx fix (push-fanout BLPOP drain #1115). The DRY refactor in #1132 introduced a tautological PKCE test (#1137) — the only behaviour-relevant regression in the batch. Schema drift (`bug-schema-drift-runtime-sql-issue-1008`, score 3) is now the top-ranked backend item; promoting it to a plan would close the gap between hand-written `sqlx::query` strings and the live schema in voting/messaging/notification paths. Draft #1140 (Epic 8A RLS GUC mismatch) is the next-most-important backend ship — both `notification_preferences` and `critical_notifications` repos run against `app.current_user_id` while policies expect `request.user_id`, and there's no FORCE RLS.
