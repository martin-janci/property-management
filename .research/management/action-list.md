# Action list

<sub>Generated: 2026-11-16T03:12:00Z — delivery actions tracked by the research routine (Phase 1.6 PM analysis).</sub>

| Priority | Owner | Action | Dependency | Status | Source |
|----------|-------|--------|------------|--------|--------|
| high | pm-devops | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimable in cloud; now compounded by #2966 so claimable=9/72 (buffer-low) | none | open | pm-analysis 2026-08-31 (carried) |
| high | pm-scrum-master | Unblock mobile-native/KMP and swagger-ui egress in the cloud runner (#2652, #2966). Claimable work is a small share of the backlog, and backend security PRs cannot compile. | none | open | pm-analysis 2026-11-16 |
| high | pm-scrum-master | Run a coverage scan to refresh coverage.json and project-state.md (both dated 2026-11-13), and confirm that epic rollups and coverage are still 100% done. | none | open | pm-analysis 2026-11-16 |
| high | pm-scrum-master | Triage the stalled reviews #2555 and #2559 (110 days). Merge them, close them, or explicitly defer them. | none | open | pm-analysis 2026-11-16 |
| high | pm-scrum-master | Sweep remaining violations mutation handlers for the 403-vs-404 cross-tenant contract (#3187 revealed assign still returns 403) — pattern audit across moderation.rs/aml_dsa.rs after PR #3183 fixed most but not assign | pm-security | open | pm-analysis 2026-10-28 |
| high | pm-scrum-master | Run the cross-tenant 404 contract sweep (pm-scrum-master-violations-xtenant-404-contract-sweep-2026-10-28), and close the open IDOR retries #2944/#2945/#2946 once the build is unblocked. | none | open | pm-analysis 2026-11-16 |
| high | pm-security | Audit routes with optional unit_id or null-scope for the missing-owner-check pattern from #3554, and add a negative cross-owner test | none | open | pm-analysis 2026-11-16 |
| high | pm-security | Merge #3560 after confirming gdpr routes return generic errors with no sqlx/db strings, and grep other routes for raw e.to_string() in responses | none | open | pm-analysis 2026-11-16 |
| medium | pm-backend | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded | none | open | dispatcher-backlog-refill 2026-10-21T22:25:48Z (score=3 conf=high vector=bug) |
| medium | pm-backend | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries + /realtors/inquiries call | none | open | dispatcher-backlog-refill 2026-10-21T22:25:48Z (score=3 conf=high vector=bug) |
| medium | pm-frontend | Adopt eslint-plugin-react-hooks + i18next/no-literal-string in frontend/apps/mobile (blocking, warn-only for one sprint) — closes the chronic class-of-defect behind the carried mobile-rn lint-gap risk (ranked-ballot + i18n fallback bugs still open 83 days later) | pm-qa | open | pm-analysis 2026-10-28 |
| medium | pm-qa | QA audit: non-mobile-native backlog items with confidence=medium need a one-shot verification step before Phase 3 can promote them | none | open | pm-analysis 2026-11-09 |
| medium | pm-scrum-master | Decide on #3427 (first-party JWT iss/aud emit and validate) and on the Dependabot majors (#3444 jsonwebtoken 10 to 11, #3443 utoipa-swagger-ui 9 to 10, #3447 vitest 5). Sequence them after the swagger-ui egress fix. | none | open | pm-analysis 2026-11-16 |
| medium | pm-security | api handlers voting cast no unit ownership | none | open | dispatcher-signals-promote 2026-11-16T00:27:09Z (tier1d api-handlers score=3 vector=security) |
| medium | pm-security | Fix the RN deep-link pending singleton by clearing it on logout and keying it by user, and guard the KMP SSO error body parsing | none | open | pm-analysis 2026-11-16 |
| low | pm-backend | admin web tenant restore stub | none | open | dispatcher-signals-promote 2026-11-12T20:25:35Z (tier1d admin-web score=2 vector=bug) |
| low | pm-backend | api core digest send failure abandoned | none | open | dispatcher-signals-promote 2026-11-15T16:26:57Z (tier1d api-core score=1 vector=bug) |
| low | pm-backend | api core digest worker no replica claim | none | open | dispatcher-signals-promote 2026-11-15T16:26:57Z (tier1d api-core score=2 vector=bug) |
| low | pm-backend | api core quiet hours weekend master gate | none | open | dispatcher-signals-promote 2026-11-15T16:26:57Z (tier1d api-core score=2 vector=bug) |
| low | pm-backend | api core statement account iban hardcoded | none | open | dispatcher-signals-promote 2026-11-13T22:25:04Z (tier1d api-core score=1 vector=bug) |
| low | pm-backend | api handlers ai workflows raw error leak | none | open | dispatcher-signals-promote 2026-11-15T00:28:30Z (tier1d api-handlers score=2 vector=bug) |
| low | pm-backend | api handlers audit logs action filter dropped | none | open | dispatcher-signals-promote 2026-11-16T00:27:09Z (tier1d api-handlers score=1 vector=bug) |
| low | pm-backend | api handlers audit summary integrity swallow | none | open | dispatcher-signals-promote 2026-11-16T00:27:09Z (tier1d api-handlers score=2 vector=bug) |
| low | pm-backend | api handlers auth inactive revoke swallowed | none | open | dispatcher-signals-promote 2026-11-15T16:26:57Z (tier1d api-handlers score=1 vector=bug) |
| low | pm-backend | api handlers esg reporting raw sqlx error leak | none | open | dispatcher-signals-promote 2026-11-15T00:28:30Z (tier1d api-handlers score=2 vector=bug) |
| low | pm-backend | api handlers mfa report population mismatch | none | open | dispatcher-signals-promote 2026-11-16T00:27:09Z (tier1d api-handlers score=2 vector=bug) |
| low | pm-backend | api handlers property valuation raw error leak | none | open | dispatcher-signals-promote 2026-11-15T16:26:57Z (tier1d api-handlers score=2 vector=bug) |
| low | pm-backend | api handlers tenant config rls clear swallowed | none | open | dispatcher-signals-promote 2026-11-15T16:26:57Z (tier1d api-handlers score=2 vector=bug) |
| low | pm-backend | api handlers vote update no tenant rls | none | open | dispatcher-signals-promote 2026-11-16T00:27:09Z (tier1d api-handlers score=2 vector=bug) |
| low | pm-backend | Shared Ktor HttpClient has no HttpTimeout - mobile API calls can hang indefinitely | none | open | dispatcher-backlog-refill 2026-10-25T08:29:08Z (score=2 conf=medium vector=bug) |
| low | pm-backend | Reality mobile MyListings + portfolio analytics silently cap at 100 listings (no pagination) | none | open | dispatcher-backlog-refill 2026-10-25T08:29:08Z (score=1 conf=medium vector=bug) |
| low | pm-backend | KMP realtor SSO error-body parsing is unguarded — crashes on non-JSON response | none | open | dispatcher-backlog-refill 2026-11-04T04:24:45Z (score=2 conf=medium vector=bug) |
| low | pm-backend | mobile-rn ReportFaultScreen category chips always render English — locale switch leaves 7 chips in EN while rest of form translates | none | open | dispatcher-backlog-refill 2026-11-15T04:24:57Z (score=2 conf=high vector=bug) |
| low | pm-backend | ppt web core authed query roots drift | none | open | dispatcher-signals-promote 2026-11-15T16:26:57Z (tier1d ppt-web-core score=2 vector=bug) |
| low | pm-backend | ppt web core errorhandler 403 mislabel | none | open | dispatcher-signals-promote 2026-11-14T20:26:05Z (tier1d ppt-web-core score=1 vector=bug) |
| low | pm-backend | ppt web core networkstatus navigator guard | none | open | dispatcher-signals-promote 2026-11-15T00:28:30Z (tier1d ppt-web-core score=1 vector=bug) |
| low | pm-backend | ppt web core org provider value not memoized | none | open | dispatcher-signals-promote 2026-11-15T16:26:57Z (tier1d ppt-web-core score=1 vector=bug) |
| low | pm-backend | ppt web core orgprovider unstable value | none | open | dispatcher-signals-promote 2026-11-15T00:28:30Z (tier1d ppt-web-core score=1 vector=bug) |
| low | pm-backend | ppt web core validation error i18n gap | none | open | dispatcher-signals-promote 2026-11-16T00:27:09Z (tier1d ppt-web-core score=1 vector=bug) |
| low | pm-backend | ppt web core ws send envelope mismatch | none | open | dispatcher-signals-promote 2026-11-16T00:27:09Z (tier1d ppt-web-core score=1 vector=bug) |
| low | pm-backend | ppt web ui moderation full page reload nav | none | open | dispatcher-signals-promote 2026-11-15T00:28:30Z (tier1d ppt-web-ui score=1 vector=bug) |
| low | pm-backend | ppt web ui voting list page no i18n | none | open | dispatcher-signals-promote 2026-11-15T00:28:30Z (tier1d ppt-web-ui score=1 vector=bug) |
| low | pm-backend | reality server import history limit 100 trunc | none | open | dispatcher-signals-promote 2026-11-16T00:27:09Z (tier1d reality-server score=1 vector=bug) |
| low | pm-backend | reality server import jobs total page len | none | open | dispatcher-signals-promote 2026-11-16T00:27:09Z (tier1d reality-server score=2 vector=bug) |
| low | pm-backend | reality server suggestions stub popular | none | open | dispatcher-signals-promote 2026-11-14T20:26:05Z (tier1d reality-server score=1 vector=bug) |
| low | pm-backend | reality web agency branding save no error | none | open | dispatcher-signals-promote 2026-11-16T00:27:09Z (tier1d reality-web score=2 vector=bug) |
| low | pm-backend | reality web analytics no i18n | none | open | dispatcher-signals-promote 2026-11-10T02:25:23Z (tier1d reality-web score=1 vector=bug) |
| low | pm-backend | reality web auth nav drops locale | none | open | dispatcher-signals-promote 2026-11-15T16:26:57Z (tier1d reality-web score=1 vector=bug) |
| low | pm-backend | reality web comparison timer leak | none | open | dispatcher-signals-promote 2026-11-15T16:26:57Z (tier1d reality-web score=1 vector=bug) |
| low | pm-backend | reality web currency code rangeerror crash | none | open | dispatcher-signals-promote 2026-11-15T16:26:57Z (tier1d reality-web score=2 vector=bug) |
| low | pm-backend | reality web login open redirect backslash | none | open | dispatcher-signals-promote 2026-11-15T16:26:57Z (tier1d reality-web score=2 vector=bug) |
| low | pm-qa | PR #3497 fix(api-server) voice book-facility false-success — shipped without regression test | none | open | dispatcher-backlog-refill 2026-11-15T04:24:57Z (score=2 conf=high vector=test-gap) |
| low | pm-qa | PR #3525 fix(api-handlers) non-admin peer-role mutation — privilege-escalation fix without a cross-tenant test | none | open | dispatcher-backlog-refill 2026-11-15T04:24:57Z (score=2 conf=high vector=test-gap) |
| low | pm-qa | PR #3524 i18n leaf-parity fix on top-churn pl.json without a parity test — follow-up #3531 re-opened the same gap | none | open | dispatcher-backlog-refill 2026-11-15T04:24:57Z (score=2 conf=medium vector=test-gap) |
| low | pm-qa | PR #3482 gated /settings/privacy behind ProtectedRoute without updating docs/screens/ppt/ | none | open | dispatcher-backlog-refill 2026-11-15T04:24:57Z (score=2 conf=medium vector=test-gap) |
| low | pm-qa | PR #3486 gated community/disputes/messaging/neighbors behind ProtectedRoute without updating docs/screens/ppt/ | none | open | dispatcher-backlog-refill 2026-11-15T04:24:57Z (score=2 conf=medium vector=test-gap) |
| low | pm-scrum-master | Mark as done in action-list the items already merged this window: errorhandler-403-mislabel, currency-code-rangeerror, login-open-redirect-backslash. Check that the 17 merges are reflected in the backlog. | none | open | pm-analysis 2026-11-16 |
| low | pm-security | api handlers gdpr raw db error leak | none | open | dispatcher-signals-promote 2026-11-16T00:27:09Z (tier1d api-handlers score=2 vector=security) |
| low | pm-security | mobile-rn DeepLinkManager.pendingLink never cleared on logout — shared-device deep-link intent crosses user boundary | none | open | dispatcher-backlog-refill 2026-11-15T04:24:57Z (score=2 conf=high vector=security) |
| low | pm-tech-lead | ppt-web messages/cs.json churn-hotspot — 4 commits in 2d (ambient i18n sweeps + UC-52 payments) | none | open | dispatcher-backlog-refill 2026-11-15T04:24:57Z (score=1 conf=high vector=refactor) |
| low | pm-tech-lead | ppt-web messages/pl.json churn-hotspot — 4 commits in 2d (i18n leaf-parity sweeps PR #3524 + locale-specific fixes) | none | open | dispatcher-backlog-refill 2026-11-15T04:24:57Z (score=1 conf=high vector=refactor) |
| low | pm-tech-lead | ppt web core api default export eager instance | none | open | dispatcher-signals-promote 2026-11-16T00:27:09Z (tier1d ppt-web-core score=1 vector=refactor) |
| low | pm-tech-lead | ppt-web messages/sk.json repeated-churn — primary locale touched in 3 of 3 recent runs | none | open | dispatcher-backlog-refill 2026-11-15T04:24:57Z (score=1 conf=high vector=refactor) |
