# Action List

_Generated: 2026-10-27T16:40:00Z_

**24 open / 24 total.** Target buffer floor: 36. Below floor by **12** — coverage is 49/49 done so gap-scan has few net-new candidates; refill is bottlenecked on reviewer throughput (9 PRs waiting on human merge).

| Priority | Owner | Action | Dependency | Source |
|---|---|---|---|---|
| high | pm-backend | Gate violations add_evidence sub-resource with authz check — currently ungated allowing any authenticated user to attach evidence to any violation record (gh #3164) | none | pm-analysis 2026-10-26 |
| high | pm-devops | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimable in cloud; now compounded by #2966 so claimable=9/72 (buffer-low) | none | pm-analysis 2026-08-31 (carried) |
| high | pm-security | Failing test on dev blocks backend CI: cross_org_assign_violation_is_not_found (assign returns 403, expects 404) (Closes #3187) | none | dispatcher-issue-ingest 2026-10-27T08:26:25Z (#3187) |
| high | pm-tech-lead | Clear the 9-PR approved-review jam (merge #3024, #3075, #3108, #3119, #3171, #3184, #3185, #3186, #3188 after a last-minute re-approval sanity check) — dispatcher reports claimable=6/72 is almost enti | none | pm-analysis 2026-10-27 |
| high | pm-backend | Sweep violations.rs for role-gate / identity-check gaps (Phase 1.5 surfaced 3 fresh findings: add_evidence role-gate asymmetry at :375, list_violations/list_appeals bulk leak at :295/:620, create_appe | none | pm-analysis 2026-10-27 |
| medium | pm-tech-lead | Portfolio/owner analytics: comparisons + dashboard summary + owner portfolio runtime drift (500s) (Closes #3104) | none | dispatcher-issue-ingest 2026-10-16T08:28:25Z (#3104) |
| medium | pm-tech-lead | Follow-up: NewsScreen category badge lacks the i18n defaultValue fallback that ArticleDetailScreen uses (PR #3110) (Closes #3116) | none | dispatcher-issue-ingest 2026-10-17T16:25:39Z (#3116) |
| medium | pm-backend | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded | none | dispatcher-backlog-refill 2026-10-21T22:25:48Z (score=3 conf |
| medium | pm-backend | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries + /realtors/inquiries call | none | dispatcher-backlog-refill 2026-10-21T22:25:48Z (score=3 conf |
| medium | pm-tech-lead | Follow-up: violations add_evidence left ungated while delete_evidence is manager-gated (PR #3072) (Closes #3164) | none | dispatcher-issue-ingest 2026-10-25T22:26:31Z (#3164) |
| medium | pm-tech-lead | Decide whether Claude-Approvals PRs auto-merge after a 24-48h cool-down or whether human-merge stays the hard gate — the 9-PR jam suggests the human step is the primary delivery bottleneck | none | pm-analysis 2026-10-27 |
| medium | pm-backend | Diagnose + resolve the merge-pipeline deadlock (gh #3131) — long-stale carried; likely sqlx transaction scope or advisory-lock contention on the merge path | none | pm-analysis 2026-10-27 |
| low | pm-qa | ppt-web route refactor (PR #2994) landed without touching docs/screens/ppt | none | dispatcher-backlog-refill 2026-10-01T06:28:31Z (score=2 conf |
| low | pm-backend | violation_number / appeal_number generated via COUNT(*)+1 on the pool (no tx/sequence) collides with UNIQUE(org, number) under concurrency → opaque 500 on create | none | dispatcher-backlog-refill 2026-10-11T10:27:29Z (score=2 conf |
| low | pm-backend | Shared Ktor HttpClient has no HttpTimeout - mobile API calls can hang indefinitely | none | dispatcher-backlog-refill 2026-10-25T08:29:08Z (score=2 conf |
| low | pm-backend | Reality mobile MyListings + portfolio analytics silently cap at 100 listings (no pagination) | none | dispatcher-backlog-refill 2026-10-25T08:29:08Z (score=1 conf |
| low | pm-qa | Churn hotspot: backend/servers/api-server/tests/suites/violations_role_authz_tests.rs — merged PR touched this file heavily this run | none | dispatcher-backlog-refill 2026-10-26T04:28:55Z (score=1 conf |
| low | pm-backend | useNeighbors + usePrivacySettings bake mount-time token into api (useMemo [] deps) — stale Bearer after refresh | none | dispatcher-backlog-refill 2026-10-27T02:41:56Z (score=2 conf |
| low | pm-devops | useIotWebSocket.hasGivenUp exposed but IotDashboardPageRoute drops it — give-up state has no UI path | none | dispatcher-backlog-refill 2026-10-27T02:41:56Z (score=1 conf |
| low | pm-backend | Un-quarantine the dispute KPIs test + add window_start<=window_end validation (carried item from pm-backend 2026-07-30) | none | pm-analysis 2026-10-27 |
| low | pm-frontend | Link UC-33.1 / UC-33.2 / UC-33.3 to the correct screen-map entries (coverage.screen_gaps.missing_use_cases carries them) — update frontmatter use-cases arrays so missing_use_cases drops to 0 | none | pm-analysis 2026-10-27 |
| low | pm-frontend | Link UC-33.1 to its screen-map entry (currently missing from docs/screens/<product>/*.md use-cases arrays) | none | gap-scan |
| low | pm-frontend | Link UC-33.2 to its screen-map entry (currently missing from docs/screens/<product>/*.md use-cases arrays) | none | gap-scan |
| low | pm-frontend | Link UC-33.3 to its screen-map entry (currently missing from docs/screens/<product>/*.md use-cases arrays) | none | gap-scan |

## Done this run
_No items closed this run._

