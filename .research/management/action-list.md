# Action List

<sub>Generated: 2026-11-10 03:12 UTC by pm Phase 1.6 (pm-scrum-master + pm-devops)</sub>

| Status | Owner | Priority | Action | Depends on |
|--------|-------|----------|--------|------------|
| open | pm-devops | high | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimable in cloud; now compounded by #2966 so claimable=9/72 (buffer-low) |  |
| open | pm-backend | medium | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded |  |
| open | pm-backend | medium | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries + /realtors/inquiries call |  |
| open | pm-backend | low | Shared Ktor HttpClient has no HttpTimeout - mobile API calls can hang indefinitely |  |
| open | pm-backend | low | Reality mobile MyListings + portfolio analytics silently cap at 100 listings (no pagination) |  |
| open | pm-frontend | medium | Adopt eslint-plugin-react-hooks + i18next/no-literal-string in frontend/apps/mobile (blocking, warn-only for one sprint) — closes the chronic class-of-defect behind the carried mobile-rn lint-gap risk (ranked-ballot + i18n fallback bugs still open 83 days later) | UNRESOLVED:pm-qa |
| open | pm-scrum-master | high | Sweep remaining violations mutation handlers for the 403-vs-404 cross-tenant contract (#3187 revealed assign still returns 403) — pattern audit across moderation.rs/aml_dsa.rs after PR #3183 fixed most but not assign | UNRESOLVED:pm-security |
| open | pm-backend | low | KMP realtor SSO error-body parsing is unguarded — crashes on non-JSON response |  |
| open | pm-security | low | SECURITY: ppt-web refresh-token persisted in localStorage — XSS-reachable |  |
| open | pm-scrum-master | high | Promote a non-mobile-native code-review vector from the next Phase 1.5 segment (ppt-web-core / reality-server oldest = 2026-11-04) to refill the planner buffer |  |
| open | pm-qa | medium | QA audit: non-mobile-native backlog items with confidence=medium need a one-shot verification step before Phase 3 can promote them |  |
| open | pm-backend | low | reality server sso cb db err leak |  |
| open | pm-backend | low | reality server sso cb upstream err leak |  |
| open | pm-backend | low | ppt web ui payment match silent fail |  |
| open | pm-tech-lead | low | ppt web ui buildings err raw i18n |  |
| open | pm-tech-lead | low | ppt web ui doc search raw err i18n |  |
| open | pm-backend | low | reality server price map mode unvalidated |  |
| open | pm-backend | low | reality server pw reset timing oracle |  |
| open | pm-backend | low | reality web json guard array gap |  |
| open | pm-backend | low | reality web realtor crud en errors |  |
| open | pm-security | low | api handlers db error leak |  |
| open | pm-security | low | legal rs db error leak |  |
| open | pm-backend | low | reality web date locale ignored |  |
| open | pm-backend | low | reality web analytics no i18n |  |
| open | pm-backend | low | reality web inquiry type raw enum |  |
| open | pm-backend | low | mobile rn voting i18n missing keys |  |
| open | pm-tech-lead | low | mobile rn neighbors search hardcoded en |  |
| open | pm-backend | low | reality server compare add limit toctou |  |
| open | pm-backend | low | reality web realtor agency i18n gap |  |
| open | pm-backend | low | reality web report page i18n gap |  |
| open | pm-backend | low | ppt web core ws votes dead invalidation |  |
| open | pm-backend | low | reality server import jobs limit unclamped |  |
| open | pm-tech-lead | low | ppt web ui documents toast hardcoded mixed locale |  |
| open | pm-tech-lead | low | ppt web ui evidence uploader hardcoded strings |  |
| open | pm-backend | low | mobile-rn NFCAccessController: midnight-crossing access-point time restriction checks the tap-day against restriction.days, so a window like Mon 23:00–01:00 wrongly denies Tue 00:00–01:00 and spuriously grants Mon 00:00–01:00 |  |
| open | pm-scrum-master | high | Triage #3357: rebase the 3 blocked PRs OR adjust Tier-1d signal sourcing to stop emitting dead-end vectors |  |
| open | pm-scrum-master | medium | Monitor the 6 new draft PRs (#3372-#3377) through CI and review; retire any that fail Band-C gates early |  |
| open | pm-devops | medium | Update pm-devops-unblock-mobile-native-cloud-builds action text: drop #2652 reference (issue is closed), re-anchor to structural claim-starvation evidence directly, and open a fresh issue if the gap deserves one |  |
| open | pm-devops | low | Verify deploy-server.yml remains dormant (last run 2026-09-27 ~44d ago); confirm no stale artifact or expired credential would silently break on next invocation |  |
