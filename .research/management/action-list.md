# Action List

_Generated: 2026-10-26T04:30:00Z_

**17 open / 17 total.** Target buffer floor: 36. Below floor by **19** — the dispatcher backlog scan should refresh (external, not this upkeep run).

| Priority | Owner | Action | Dependency | Source |
|---|---|---|---|---|
| high | pm-backend | Gate violations add_evidence sub-resource with authz check — currently ungated allowing any authenticated user to attach evidence to any violation record (gh #3164) | none | pm-analysis 2026-10-26 |
| high | pm-devops | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimable in cloud; now compounded by #2966 so claimable=9/72 (buffer-low) | none | pm-analysis 2026-08-31 (carried) |
| high | pm-security | Resolve 3 new RUSTSEC advisories blocking backend merge pipeline (cargo-deny advisories FAILED on dev) — bump offending transitive crate(s) or add bounded cargo-deny exceptions with deadline (Closes #3168) | none | dispatcher-issue-ingest 2026-10-26T04:30:00Z (#3168) |
| medium | pm-backend | Diagnose + resolve merge-pipeline deadlock (gh #3131) — long-running issue; likely sqlx transaction scope or advisory-lock contention in the merge path | none | pm-analysis 2026-10-26 |
| medium | pm-tech-lead | Portfolio/owner analytics: comparisons + dashboard summary + owner portfolio runtime drift (500s) (Closes #3104) | none | dispatcher-issue-ingest 2026-10-16T08:28:25Z (#3104) |
| medium | pm-tech-lead | Follow-up: NewsScreen category badge lacks the i18n defaultValue fallback that ArticleDetailScreen uses (PR #3110) (Closes #3116) | none | dispatcher-issue-ingest 2026-10-17T16:25:39Z (#3116) |
| medium | pm-backend | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded | none | dispatcher-backlog-refill 2026-10-21T22:25:48Z (score=3 c... |
| medium | pm-backend | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries + /realtors/inquiries call | none | dispatcher-backlog-refill 2026-10-21T22:25:48Z (score=3 c... |
| medium | pm-tech-lead | Follow-up: unassignModerationCase still 404s after #3157 was closed — missing backend route + contract-test gap (PR #3158) (Closes #3163) | none | dispatcher-issue-ingest 2026-10-25T22:26:31Z (#3163) |
| medium | pm-tech-lead | Follow-up: violations add_evidence left ungated while delete_evidence is manager-gated (PR #3072) (Closes #3164) | none | dispatcher-issue-ingest 2026-10-25T22:26:31Z (#3164) |
| low | pm-scrum-master | Reconcile sprint-status.yaml drift against coverage.json (Epics 6/7A/10B/80 show in-progress/partial in sprint-status but 100% done in coverage) — flip sprint-status to reflect reality or schedule a deep scan to re-ve... | none | pm-analysis 2026-10-26 |
| low | pm-qa | ppt-web route refactor (PR #2994) landed without touching docs/screens/ppt | none | dispatcher-backlog-refill 2026-10-01T06:28:31Z (score=2 c... |
| low | pm-backend | violation_number / appeal_number generated via COUNT(*)+1 on the pool (no tx/sequence) collides with UNIQUE(org, number) under concurrency → opaque 500 on create | none | dispatcher-backlog-refill 2026-10-11T10:27:29Z (score=2 c... |
| low | pm-qa | Churn hotspot: backend/servers/api-server/tests/suites/migration_import_upload_persists_file.rs — merged PR touched this file heavily this run | none | dispatcher-backlog-refill 2026-10-25T04:26:57Z (score=1 c... |
| low | pm-tech-lead | Churn hotspot: backend/servers/api-server/src/services/push_fanout.rs — merged PR touched this file heavily this run | none | dispatcher-backlog-refill 2026-10-25T04:26:57Z (score=1 c... |
| low | pm-backend | Shared Ktor HttpClient has no HttpTimeout - mobile API calls can hang indefinitely | none | dispatcher-backlog-refill 2026-10-25T08:29:08Z (score=2 c... |
| low | pm-backend | Reality mobile MyListings + portfolio analytics silently cap at 100 listings (no pagination) | none | dispatcher-backlog-refill 2026-10-25T08:29:08Z (score=1 c... |

## Done this run
_No items closed this run._
