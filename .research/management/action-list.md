# Action list

<sub>Generated: 2026-11-17T03:17:29Z — delivery actions tracked by the research routine (Phase 1.6 PM analysis).</sub>

| Priority | Owner | Action | Dependency | Status | Source |
|----------|-------|--------|------------|--------|--------|
| high | pm-devops | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimable in cloud; now compounded by #2966 so claimable=9/72 (buffer-low) | none | open | pm-analysis 2026-08-31 (carried) |
| high | pm-scrum-master | Sweep remaining violations mutation handlers for the 403-vs-404 cross-tenant contract (#3187 revealed assign still returns 403) — pattern audit across moderation.rs/aml_dsa.rs after PR #3183 fixed most but not assign | pm-security | open | pm-analysis 2026-10-28 |
| high | pm-scrum-master | Unblock mobile-native/KMP and swagger-ui egress in the cloud runner (#2652, #2966). Claimable work is a small share of the backlog, and backend security PRs cannot compile. | none | open | pm-analysis 2026-11-16 |
| high | pm-scrum-master | Fix digest-candidates INNER JOIN in granular_notification.rs find_inactive_digest_candidates (use LEFT JOIN with COALESCE(la.last_active, u.created_at)) and add a regression test with a user who has no refresh_tokens row | none | open | pm-analysis 2026-11-17 |
| high | pm-scrum-master | Add terminal abandoned state for digests (abandoned_at column via new migration, set in report_abandoned) so abandoned rows are queryable, not log-only | none | open | pm-analysis 2026-11-17 |
| high | pm-scrum-master | Triage #3583 (RLS super-admin context bleed, follow-up to #3577) and confirm whether pooled connections can retain a super-admin RLS context | none | open | pm-analysis 2026-11-17 |
| medium | pm-backend | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded | none | open | dispatcher-backlog-refill 2026-10-21T22:25:48Z (score=3 conf=high vector=bug) |
| medium | pm-backend | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries + /realtors/inquiries call | none | open | dispatcher-backlog-refill 2026-10-21T22:25:48Z (score=3 conf=high vector=bug) |
| medium | pm-qa | QA audit: non-mobile-native backlog items with confidence=medium need a one-shot verification step before Phase 3 can promote them | none | open | pm-analysis 2026-11-09 |
| medium | pm-qa | Add a dispatcher-self-test.sh test case that covers the quarantined-row recovery path so #3357-class stalls fail CI instead of silently queueing [retry 1/2 of failed pm-qa-dispatcher-self-test-quarantined-recovery-2026-11-09] | none | open | dispatcher-retry-remint 2026-11-17T00:24:33Z (retry_of=pm-qa-dispatcher-self-test-quarantined-recovery-2026-11-09 reason=failed-no-pr cooldown_ok newest_failure=2026-11-09T18:34:56Z) |
| medium | pm-scrum-master | Fix digest sweep advisory-lock release leak (drop or close the connection on release error instead of returning it to the pool) | none | open | pm-analysis 2026-11-17 |
| medium | pm-scrum-master | Resolve #3584 (mobile biome gate downgrade) by deciding the warn-to-error promotion date for the #3566 lint rules | none | open | pm-analysis 2026-11-17 |
| medium | pm-data | Define digest KPIs and emit counters (candidates_considered, sent, failed, abandoned, excluded_by_filter) plus a per-sweep summary log or metric | none | open | pm-analysis 2026-11-17 |
| medium | pm-data | Add trackEvent calls for fault_report_submitted/failed (mobile ReportFault) and moderation route decisions (SPA), with a name, trigger and property schema; mobile needs an equivalent sink | none | open | pm-analysis 2026-11-17 |
| low | pm-backend | Shared Ktor HttpClient has no HttpTimeout - mobile API calls can hang indefinitely | none | open | dispatcher-backlog-refill 2026-10-25T08:29:08Z (score=2 conf=medium vector=bug) |
| low | pm-backend | Reality mobile MyListings + portfolio analytics silently cap at 100 listings (no pagination) | none | open | dispatcher-backlog-refill 2026-10-25T08:29:08Z (score=1 conf=medium vector=bug) |
| low | pm-backend | KMP realtor SSO error-body parsing is unguarded — crashes on non-JSON response | none | open | dispatcher-backlog-refill 2026-11-04T04:24:45Z (score=2 conf=medium vector=bug) |
| low | pm-backend | reality web analytics no i18n | none | open | dispatcher-signals-promote 2026-11-10T02:25:23Z (tier1d reality-web score=1 vector=bug) |
| low | pm-backend | api core statement account iban hardcoded | none | open | dispatcher-signals-promote 2026-11-13T22:25:04Z (tier1d api-core score=1 vector=bug) |
| low | pm-tech-lead | ppt-web messages/cs.json churn-hotspot — 4 commits in 2d (ambient i18n sweeps + UC-52 payments) | none | open | dispatcher-backlog-refill 2026-11-15T04:24:57Z (score=1 conf=high vector=refactor) |
| low | pm-tech-lead | ppt-web messages/pl.json churn-hotspot — 4 commits in 2d (i18n leaf-parity sweeps PR #3524 + locale-specific fixes) | none | open | dispatcher-backlog-refill 2026-11-15T04:24:57Z (score=1 conf=high vector=refactor) |
| low | pm-tech-lead | ppt-web messages/sk.json repeated-churn — primary locale touched in 3 of 3 recent runs | none | open | dispatcher-backlog-refill 2026-11-15T04:24:57Z (score=1 conf=high vector=refactor) |
| low | pm-backend | ppt web core org provider value not memoized | none | open | dispatcher-signals-promote 2026-11-15T16:26:57Z (tier1d ppt-web-core score=1 vector=bug) |
| low | pm-backend | reality web comparison timer leak | none | open | dispatcher-signals-promote 2026-11-15T16:26:57Z (tier1d reality-web score=1 vector=bug) |
| low | pm-backend | reality web auth nav drops locale | none | open | dispatcher-signals-promote 2026-11-15T16:26:57Z (tier1d reality-web score=1 vector=bug) |
| low | pm-backend | ppt web core authed query roots drift | none | open | dispatcher-signals-promote 2026-11-15T16:26:57Z (tier1d ppt-web-core score=2 vector=bug) |
| low | pm-backend | ppt web core validation error i18n gap | none | open | dispatcher-signals-promote 2026-11-16T00:27:09Z (tier1d ppt-web-core score=1 vector=bug) |
| low | pm-backend | api handlers audit summary integrity swallow | none | open | dispatcher-signals-promote 2026-11-16T00:27:09Z (tier1d api-handlers score=2 vector=bug) |
| low | pm-backend | api handlers mfa report population mismatch | none | open | dispatcher-signals-promote 2026-11-16T00:27:09Z (tier1d api-handlers score=2 vector=bug) |
| low | pm-backend | api handlers vote update no tenant rls | none | open | dispatcher-signals-promote 2026-11-16T00:27:09Z (tier1d api-handlers score=2 vector=bug) |
| low | pm-backend | reality server import jobs total page len | none | open | dispatcher-signals-promote 2026-11-16T00:27:09Z (tier1d reality-server score=2 vector=bug) |
| low | pm-backend | api handlers audit logs action filter dropped | none | open | dispatcher-signals-promote 2026-11-16T00:27:09Z (tier1d api-handlers score=1 vector=bug) |
| low | pm-tech-lead | ppt web core api default export eager instance | none | open | dispatcher-signals-promote 2026-11-16T00:27:09Z (tier1d ppt-web-core score=1 vector=refactor) |
| low | pm-backend | ppt web core ws send envelope mismatch | none | open | dispatcher-signals-promote 2026-11-16T00:27:09Z (tier1d ppt-web-core score=1 vector=bug) |
| low | pm-backend | reality server import history limit 100 trunc | none | open | dispatcher-signals-promote 2026-11-16T00:27:09Z (tier1d reality-server score=1 vector=bug) |
| low | pm-backend | reality web agency branding save no error | none | open | dispatcher-signals-promote 2026-11-16T00:27:09Z (tier1d reality-web score=2 vector=bug) |
| low | pm-scrum-master | Mark as done in action-list the items already merged this window: errorhandler-403-mislabel, currency-code-rangeerror, login-open-redirect-backslash. Check that the 17 merges are reflected in the backlog. | none | open | pm-analysis 2026-11-16 |
| low | pm-qa | Screen-map drift: PR #3546 faults URL-param validation without docs/screens/ppt update | none | open | dispatcher-backlog-refill 2026-11-16T18:25:46Z (score=2 conf=medium vector=test-gap) |
| low | pm-qa | Screen-map drift: PR #3559 login open-redirect fix without docs/screens/reality update | none | open | dispatcher-backlog-refill 2026-11-16T18:25:46Z (score=2 conf=medium vector=test-gap) |
| low | pm-tech-lead | Churn hotspot: reality-server routes/listings.rs (slug + category i18n sweeps) | none | open | dispatcher-backlog-refill 2026-11-16T18:25:46Z (score=1 conf=high vector=refactor) |
| low | pm-tech-lead | Churn hotspot: ppt-web features/reports/AnalyticsChart.tsx (reports feature rollout) | none | open | dispatcher-backlog-refill 2026-11-16T18:25:46Z (score=1 conf=high vector=refactor) |
| low | pm-tech-lead | Churn hotspot: ppt-web features/reports/TrendChart.tsx (reports feature rollout) | none | open | dispatcher-backlog-refill 2026-11-16T18:25:46Z (score=1 conf=high vector=refactor) |
| low | pm-backend | reality-server record_view: comment claims existence check but handler does not verify + no per-IP throttle | none | open | dispatcher-backlog-refill 2026-11-16T18:25:46Z (score=1 conf=medium vector=bug) |
| low | pm-qa | reality-server ListingSummary (/listings search) emits bare UUID; SEO slug only on /featured | none | open | dispatcher-backlog-refill 2026-11-16T18:25:46Z (score=1 conf=medium vector=test-gap) |
| low | pm-backend | reality web realtor detail modal no error | none | open | dispatcher-signals-promote 2026-11-17T00:24:34Z (tier1d reality-web score=2 vector=bug) |
| low | pm-backend | reality web agent profile mock stub | none | open | dispatcher-signals-promote 2026-11-17T00:24:34Z (tier1d reality-web score=1 vector=bug) |
| low | pm-security | api handlers oauth client error leak | none | open | dispatcher-signals-promote 2026-11-17T00:24:34Z (tier1d api-handlers score=2 vector=security) |
| low | pm-security | ppt web core route groups no protectedroute | none | open | dispatcher-signals-promote 2026-11-17T00:24:34Z (tier1d ppt-web-core score=2 vector=security) |
| low | pm-backend | ppt web ui facilities ismanager stub true | none | open | dispatcher-signals-promote 2026-11-17T00:24:34Z (tier1d ppt-web-ui score=2 vector=bug) |
| low | pm-backend | ppt web ui registry api client no auth | none | open | dispatcher-signals-promote 2026-11-17T00:24:34Z (tier1d ppt-web-ui score=2 vector=bug) |
| low | pm-backend | reality server suggestions hardcoded sk region | none | open | dispatcher-signals-promote 2026-11-17T00:24:34Z (tier1d reality-server score=2 vector=bug) |
| low | pm-backend | mobile rn alert hardcoded en | none | open | dispatcher-signals-promote 2026-11-17T00:24:34Z (tier1d mobile-rn score=1 vector=bug) |
| low | pm-backend | mobile rn personmonths screen no i18n | none | open | dispatcher-signals-promote 2026-11-17T00:24:34Z (tier1d mobile-rn score=1 vector=bug) |
| low | pm-backend | mobile rn widget settings i18n leak | none | open | dispatcher-signals-promote 2026-11-17T00:24:34Z (tier1d mobile-rn score=1 vector=bug) |
| low | pm-backend | ppt web ui facilities forms hardcoded errors | none | open | dispatcher-signals-promote 2026-11-17T00:24:34Z (tier1d ppt-web-ui score=1 vector=bug) |
| low | pm-backend | reality server categories labels sk only | none | open | dispatcher-signals-promote 2026-11-17T00:24:34Z (tier1d reality-server score=1 vector=bug) |
| low | pm-scrum-master | Review draft PRs #3582 (T33 quarantine tests), #3585 and #3586 (ppt-web locale churn plans), then mark ready or close | none | open | pm-analysis 2026-11-17 |
| low | pm-data | Backfill KPI definitions for shipped stories in sprint-status.yaml or the epic docs, and add an events/KPI column to the story template | none | open | pm-analysis 2026-11-17 |
| low | pm-backend | api core digest worker no replica claim | none | in-progress | dispatcher-signals-promote 2026-11-15T16:26:57Z (tier1d api-core score=2 vector=bug) |
| low | pm-backend | api handlers tenant config rls clear swallowed | none | in-progress | dispatcher-signals-promote 2026-11-15T16:26:57Z (tier1d api-handlers score=2 vector=bug) |
| medium | pm-scrum-master | Decide on #3427 (first-party JWT iss/aud emit and validate) and on the Dependabot majors (#3444 jsonwebtoken 10 to 11, #3443 utoipa-swagger-ui 9 to 10, #3447 vitest 5). Sequence them after the swagger-ui egress fix. | none | done | pm-analysis 2026-11-16 |
| low | pm-qa | PR #3482 gated /settings/privacy behind ProtectedRoute without updating docs/screens/ppt/ | none | done | dispatcher-backlog-refill 2026-11-15T04:24:57Z (score=2 conf=medium vector=test-gap) |
| low | pm-qa | PR #3486 gated community/disputes/messaging/neighbors behind ProtectedRoute without updating docs/screens/ppt/ | none | done | dispatcher-backlog-refill 2026-11-15T04:24:57Z (score=2 conf=medium vector=test-gap) |
