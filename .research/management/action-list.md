# Action list

<sub>Generated: 2026-11-08T18:35:00Z — this file is regenerated from action-list.json.</sub>

| Priority | Status | Owner | Action | Dependency | Source |
|----------|--------|-------|--------|------------|--------|
| high | open | pm-devops | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimabl | none | pm-analysis 2026-08-31 (carried) |
| high | open | pm-scrum-master | Sweep remaining violations mutation handlers for the 403-vs-404 cross-tenant contract (#3187 revealed assign still retur | pm-security | pm-analysis 2026-10-28 |
| high | open | pm-scrum-master | Resolve #3357: re-land the dropped work from #3301, #3333 and #3356 onto the rewritten dev. Start with #3358 (draft of # | none | pm-analysis 2026-11-08 |
| high | open | pm-qa | Re-land the three orphaned PRs (#3301/#3333/#3356) as fresh branches from current dev — do not cherry-pick onto stale ba | none | pm-analysis 2026-11-08 |
| high | open | pm-qa | For each re-landed PR, list the tests the original PR added and confirm they still exist and run after conflict resoluti | pm-qa-reland-orphaned-prs-2026-11-08 | pm-analysis 2026-11-08 |
| high | open | pm-qa | Define and test acceptance criteria for the Tier-1d signal generator dead-end fix (#3357 item 2): signal emitted, signal | none | pm-analysis 2026-11-08 |
| high | open | pm-qa | Write test suites for the four mobile-rn auth screens (LoginScreen/TwoFactorScreen first). Cover validation errors, subm | none | pm-analysis 2026-11-08 |
| medium | open | pm-backend | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded |  | dispatcher-backlog-refill 2026-10-21T22:25:48Z (score=3 conf=high vector=bug) |
| medium | open | pm-backend | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on ever |  | dispatcher-backlog-refill 2026-10-21T22:25:48Z (score=3 conf=high vector=bug) |
| medium | open | pm-frontend | Adopt eslint-plugin-react-hooks + i18next/no-literal-string in frontend/apps/mobile (blocking, warn-only for one sprint) | pm-qa | pm-analysis 2026-10-28 |
| medium | open | pm-scrum-master | Mint non-gated planned vectors from this run's mobile-rn review findings (auth-screens-no-tests, notifications-screen-no | none | pm-analysis 2026-11-08 |
| medium | open | pm-scrum-master | Run a local coverage scan and a screen-map drift pass to refill the buffer; triage stalled PR #2555 and #2559 (keep/reba | none | pm-analysis 2026-11-08 |
| medium | open | pm-qa | Add a pipeline-health regression guard: a CI/scheduled check that flags approved PRs unmergeable >N hours and flags dev- | none | pm-analysis 2026-11-08 |
| medium | open | pm-qa | Supply fresh non-gated planned test-hardening vectors for the dispatcher buffer: mobile auth-screen tests (split per-scr | none | pm-analysis 2026-11-08 |
| low | open | pm-backend | Shared Ktor HttpClient has no HttpTimeout - mobile API calls can hang indefinitely |  | dispatcher-backlog-refill 2026-10-25T08:29:08Z (score=2 conf=medium vector=bug) |
| low | open | pm-backend | Reality mobile MyListings + portfolio analytics silently cap at 100 listings (no pagination) |  | dispatcher-backlog-refill 2026-10-25T08:29:08Z (score=1 conf=medium vector=bug) |
| low | open | pm-backend | KMP realtor SSO error-body parsing is unguarded — crashes on non-JSON response |  | dispatcher-backlog-refill 2026-11-04T04:24:45Z (score=2 conf=medium vector=bug) |
| low | open | pm-security | SECURITY: ppt-web refresh-token persisted in localStorage — XSS-reachable |  | dispatcher-backlog-refill 2026-11-04T04:24:45Z (score=2 conf=medium vector=security) |
| low | open | pm-qa | Screen-map drift: PR #3293 i18n'd reality-web realtor/[id] page without updating docs/screens/reality/realtor-detail.md |  | dispatcher-backlog-refill 2026-11-05T08:30:13Z (score=2 conf=medium vector=test-gap) |
| medium | in-progress | pm-tech-lead | Follow-up: realtor public profile page renders hardcoded Slovak UI on a 6-language portal (PR #3225) (Closes #3283) |  | dispatcher-issue-ingest 2026-11-04T14:28:32Z (#3283) |
| medium | in-progress | pm-tech-lead | Follow-up: member-role escalation gate reimplements role parsing instead of reusing common::TenantRole::from_role_type ( |  | dispatcher-issue-ingest 2026-11-04T20:32:18Z (#3304) |
| medium | in-progress | pm-tech-lead | Follow-up: fold IoT org-scope check into the insert on the hot add_reading path instead of a separate SELECT EXISTS (PR  |  | dispatcher-issue-ingest 2026-11-04T20:32:18Z (#3305) |
| medium | in-progress | pm-tech-lead | Follow-up: consolidate per-file require_manager(rls) helpers onto the shared rls_authz::require_manager_rls (PR #3287) ( |  | dispatcher-issue-ingest 2026-11-04T20:32:18Z (#3306) |
| medium | in-progress | pm-tech-lead | Follow-up: fault update/triage/status handlers manually guard org on an id-only query instead of using find_by_id_for_or |  | dispatcher-issue-ingest 2026-11-04T20:32:18Z (#3307) |
| medium | in-progress | pm-tech-lead | Follow-up: add RLS-aware repo variants for vote sub-resource reads instead of pool-bypass + manual vote_in_org guard (PR |  | dispatcher-issue-ingest 2026-11-04T20:32:18Z (#3308) |
| medium | in-progress | pm-tech-lead | Follow-up: cs.json/sk.json were reordered (~9k-line churn) instead of appended; enforce stable key order for messages/*. |  | dispatcher-issue-ingest 2026-11-04T20:32:18Z (#3309) |
| medium | in-progress | pm-tech-lead | Follow-up: community feed shows every author as "Neighbor" (API returns ids only); formatMoney coerces Decimal via Numbe |  | dispatcher-issue-ingest 2026-11-04T20:32:18Z (#3310) |
| medium | done | pm-frontend | ppt-web RegistryPage.tsx delete/review mutation handlers (L102-142, L171-211) swallow failures with only console.error — |  | dispatcher-tier1d-devreview 2026-11-07T12:31:00Z (signals/2026-11-07-ppt-web-ui-tier1d.json) |
| low | done | pm-tech-lead | ppt-web registry feature (pet/vehicle forms + lists) ships hardcoded English placeholders in a 4-locale app — 0 useTrans |  | dispatcher-backlog-refill 2026-11-05T08:30:13Z (score=1 conf=medium vector=refactor) |
| low | done | pm-backend | reality-web shared-comparison loader casts the listings API body with an unchecked `as Promise<ListingSummary>` — no sha |  | dispatcher-backlog-refill 2026-11-05T22:25:45Z (score=1 conf=medium vector=bug) |
| low | done | pm-frontend | ppt-web RegistryPage.tsx (L249-250,275,302,317-318) hardcodes English (title, subtitle, Pets/Vehicles tabs, load-error)  |  | dispatcher-tier1d-devreview 2026-11-07T12:31:00Z (signals/2026-11-07-ppt-web-ui-tier1d.json) |
