# PPT Action List

_Generated: 2026-11-09T00:39:10Z_

| Priority | Owner | Action | Status | Depends on | Source |
|----------|-------|--------|--------|------------|--------|
| high | pm-devops | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimabl | open | none | pm-analysis 2026-08-31 (carried) |
| medium | pm-backend | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded | open | none | dispatcher-backlog-refill 2026-10-21T22:25:48Z (score=3 conf=high vector=bug) |
| medium | pm-backend | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on ever | open | none | dispatcher-backlog-refill 2026-10-21T22:25:48Z (score=3 conf=high vector=bug) |
| low | pm-backend | Shared Ktor HttpClient has no HttpTimeout - mobile API calls can hang indefinitely | open | none | dispatcher-backlog-refill 2026-10-25T08:29:08Z (score=2 conf=medium vector=bug) |
| low | pm-backend | Reality mobile MyListings + portfolio analytics silently cap at 100 listings (no pagination) | open | none | dispatcher-backlog-refill 2026-10-25T08:29:08Z (score=1 conf=medium vector=bug) |
| medium | pm-frontend | Adopt eslint-plugin-react-hooks + i18next/no-literal-string in frontend/apps/mobile (blocking, warn-only for one sprint) | open | pm-devops-unblock-mobile-native-cloud-builds | pm-analysis 2026-10-28 |
| high | pm-scrum-master | Sweep remaining violations mutation handlers for the 403-vs-404 cross-tenant contract (#3187 revealed assign still retur | open | UNRESOLVED:pm-security | pm-analysis 2026-10-28 |
| low | pm-backend | KMP realtor SSO error-body parsing is unguarded — crashes on non-JSON response | open | none | dispatcher-backlog-refill 2026-11-04T04:24:45Z (score=2 conf=medium vector=bug) |
| low | pm-security | SECURITY: ppt-web refresh-token persisted in localStorage — XSS-reachable | open | none | dispatcher-backlog-refill 2026-11-04T04:24:45Z (score=2 conf=medium vector=security) |
| medium | pm-tech-lead | Follow-up: realtor public profile page renders hardcoded Slovak UI on a 6-language portal (PR #3225) (Closes #3283) | in-progress | none | dispatcher-issue-ingest 2026-11-04T14:28:32Z (#3283) |
| medium | pm-tech-lead | Follow-up: member-role escalation gate reimplements role parsing instead of reusing common::TenantRole::from_role_type ( | in-progress | none | dispatcher-issue-ingest 2026-11-04T20:32:18Z (#3304) |
| medium | pm-tech-lead | Follow-up: fold IoT org-scope check into the insert on the hot add_reading path instead of a separate SELECT EXISTS (PR  | in-progress | none | dispatcher-issue-ingest 2026-11-04T20:32:18Z (#3305) |
| medium | pm-tech-lead | Follow-up: consolidate per-file require_manager(rls) helpers onto the shared rls_authz::require_manager_rls (PR #3287) ( | in-progress | none | dispatcher-issue-ingest 2026-11-04T20:32:18Z (#3306) |
| medium | pm-tech-lead | Follow-up: fault update/triage/status handlers manually guard org on an id-only query instead of using find_by_id_for_or | in-progress | none | dispatcher-issue-ingest 2026-11-04T20:32:18Z (#3307) |
| medium | pm-tech-lead | Follow-up: add RLS-aware repo variants for vote sub-resource reads instead of pool-bypass + manual vote_in_org guard (PR | in-progress | none | dispatcher-issue-ingest 2026-11-04T20:32:18Z (#3308) |
| medium | pm-tech-lead | Follow-up: cs.json/sk.json were reordered (~9k-line churn) instead of appended; enforce stable key order for messages/*. | in-progress | none | dispatcher-issue-ingest 2026-11-04T20:32:18Z (#3309) |
| medium | pm-tech-lead | Follow-up: community feed shows every author as "Neighbor" (API returns ids only); formatMoney coerces Decimal via Numbe | in-progress | none | dispatcher-issue-ingest 2026-11-04T20:32:18Z (#3310) |
| low | pm-tech-lead | ppt-web registry feature (pet/vehicle forms + lists) ships hardcoded English placeholders in a 4-locale app — 0 useTrans | in-progress | none | dispatcher-backlog-refill 2026-11-05T08:30:13Z (score=1 conf=medium vector=refactor) |
| low | pm-backend | reality-web shared-comparison loader casts the listings API body with an unchecked `as Promise<ListingSummary>` — no sha | in-progress | none | dispatcher-backlog-refill 2026-11-05T22:25:45Z (score=1 conf=medium vector=bug) |
| medium | pm-frontend | ppt-web RegistryPage.tsx delete/review mutation handlers (L102-142, L171-211) swallow failures with only console.error — | in-progress | none | dispatcher-tier1d-devreview 2026-11-07T12:31:00Z (signals/2026-11-07-ppt-web-ui-tier1d.json) |
| low | pm-frontend | ppt-web RegistryPage.tsx (L249-250,275,302,317-318) hardcodes English (title, subtitle, Pets/Vehicles tabs, load-error)  | in-progress | none | dispatcher-tier1d-devreview 2026-11-07T12:31:00Z (signals/2026-11-07-ppt-web-ui-tier1d.json) |
| medium | pm-qa | Add a RegistryPage integration test covering all four pet/vehicle mutation paths in one file — current coverage is PR-sh | open | none | pm-analysis 2026-11-09 |
| medium | pm-qa | Add dispatcher-self-test T31 that hard-fails when an assignment base commit is not an ancestor of origin/dev (issue #335 | open | none | pm-analysis 2026-11-09 |
| high | pm-scrum-master | Resolve or escalate the two 12-day-stale UNRESOLVED: sentinels in action-list (pm-scrum-master-violations-xtenant-404-co | open | UNRESOLVED:pm-security owes adoption or deferral | pm-analysis 2026-11-09 |
| high | pm-devops | Formal decision on pm-devops-unblock-mobile-native-cloud-builds keystone (#2652) — either commit to a ship date within 7 | open | pm-devops-unblock-mobile-native-cloud-builds | pm-analysis 2026-11-09 |
