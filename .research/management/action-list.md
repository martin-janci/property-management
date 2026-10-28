# Action List

_Generated: 2026-10-28T03:11:13Z_

**20 open / 20 total.** Target buffer floor: 36. Below floor by **16** — dispatcher backlog scan should refresh (external, not this upkeep run).

| Priority | Owner | Action | Dependency | Source |
|---|---|---|---|---|
| high | pm-backend | Gate violations add_evidence sub-resource with authz check — currently ungated allowing any authenticated user to attach evidence to any violation record (gh #3164) | none | pm-analysis 2026-10-26 |
| high | pm-devops | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimable in cloud; now compounded by #2966 so claimable=9/72 (buffer-low) | none | pm-analysis 2026-08-31 (carried) |
| high | pm-security | Failing test on dev blocks backend CI: cross_org_assign_violation_is_not_found (assign returns 403, expects 404) (Closes #3187) | none | dispatcher-issue-ingest 2026-10-27T08:26:25Z (#3187) |
| medium | pm-backend | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded | none | dispatcher-backlog-refill 2026-10-21T22:25:48Z (score=3 c... |
| medium | pm-backend | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries + /realtors/inquiries call | none | dispatcher-backlog-refill 2026-10-21T22:25:48Z (score=3 c... |
| medium | pm-frontend | Ship a VoteDetailScreen fix pass: implement rank-position UI for ranked questions (currently rendered as unordered multi-select at L452-453 so ranked ballots cannot express preference) AND remove the dead `t(key) ?? "... | none | pm-analysis 2026-10-28 |
| medium | pm-frontend | Adopt eslint-plugin-react-hooks + i18next no-hardcoded-strings in frontend/apps/mobile and gate CI on it (addresses risk pm-qa-mobile-rn-lint-gap-hooks-i18n-2026-08-25). The 2 recent VoteDetailScreen findings (ranked ... | none | pm-analysis 2026-10-28 |
| medium | pm-tech-lead | Portfolio/owner analytics: comparisons + dashboard summary + owner portfolio runtime drift (500s) (Closes #3104) | none | dispatcher-issue-ingest 2026-10-16T08:28:25Z (#3104) |
| medium | pm-tech-lead | Follow-up: NewsScreen category badge lacks the i18n defaultValue fallback that ArticleDetailScreen uses (PR #3110) (Closes #3116) | none | dispatcher-issue-ingest 2026-10-17T16:25:39Z (#3116) |
| medium | pm-tech-lead | Follow-up: violations add_evidence left ungated while delete_evidence is manager-gated (PR #3072) (Closes #3164) | none | dispatcher-issue-ingest 2026-10-25T22:26:31Z (#3164) |
| low | pm-backend | violation_number / appeal_number generated via COUNT(*)+1 on the pool (no tx/sequence) collides with UNIQUE(org, number) under concurrency → opaque 500 on create | none | dispatcher-backlog-refill 2026-10-11T10:27:29Z (score=2 c... |
| low | pm-backend | Shared Ktor HttpClient has no HttpTimeout - mobile API calls can hang indefinitely | none | dispatcher-backlog-refill 2026-10-25T08:29:08Z (score=2 c... |
| low | pm-backend | Reality mobile MyListings + portfolio analytics silently cap at 100 listings (no pagination) | none | dispatcher-backlog-refill 2026-10-25T08:29:08Z (score=1 c... |
| low | pm-backend | useNeighbors + usePrivacySettings bake mount-time token into api (useMemo [] deps) — stale Bearer after refresh | none | dispatcher-backlog-refill 2026-10-27T02:41:56Z (score=2 c... |
| low | pm-devops | useIotWebSocket.hasGivenUp exposed but IotDashboardPageRoute drops it — give-up state has no UI path | none | dispatcher-backlog-refill 2026-10-27T02:41:56Z (score=1 c... |
| low | pm-frontend | Reality-web [locale] route tree is a running churn hotspot across runs (sell/report/home/components/import). After #3179 price-locale fix, inventory open route changes and plan a batched sweep for locale/pricing/curre... | pm-qa | pm-analysis 2026-10-28 |
| low | pm-frontend | VoteDetailScreen renders `ranked` questions with the same unordered checkbox UI as multiple_choice (isMulti groups both at L452-453) — no rank position is shown or captured, so a ranked ballot cannot express preferenc... | none | rotating-expert-review 2026-10-27 (score=2 conf=medium ve... |
| low | pm-frontend | VoteDetailScreen uses `t('key') ?? 'English'` throughout (L180-354) — i18next t() never returns nullish (returns the key string on a miss), so the English fallbacks are dead code and a missing locale key renders the r... | none | rotating-expert-review 2026-10-27 (score=1 conf=medium ve... |
| low | pm-qa | ppt-web route refactor (PR #2994) landed without touching docs/screens/ppt | none | dispatcher-backlog-refill 2026-10-01T06:28:31Z (score=2 c... |
| low | pm-qa | Churn hotspot: backend/servers/api-server/tests/suites/violations_role_authz_tests.rs — merged PR touched this file heavily this run | none | dispatcher-backlog-refill 2026-10-26T04:28:55Z (score=1 c... |

## Done this run
_No items closed this run._
