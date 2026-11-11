# Action List

<sub>Generated: 2026-11-11 03:10 UTC by pm Phase 1.6 (pm-scrum-master + pm-devops)</sub>

| Status | Owner | Priority | Action | Depends on |
|--------|-------|----------|--------|------------|
| open | pm-devops | high | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — carried from 2026-08-31. This week only 2 open non-triage items remain (both mobile-native-kmp); the dispatcher pressure has eased but the structural claim-gap in cloud remains. Decide: fix the runner, or formally split the queue |  |
| open | pm-backend | medium | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded |  |
| open | pm-backend | medium | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries + /realtors/inquiries call |  |
| open | pm-backend | low | Shared Ktor HttpClient has no HttpTimeout - mobile API calls can hang indefinitely |  |
| open | pm-backend | low | Reality mobile MyListings + portfolio analytics silently cap at 100 listings (no pagination) |  |
| open | pm-frontend | medium | Adopt eslint-plugin-react-hooks + i18next/no-literal-string in frontend/apps/mobile (blocking, warn-only for one sprint) — closes the chronic class-of-defect behind the carried mobile-rn lint-gap risk (ranked-ballot + i18n fallback bugs still open 83 days later) | UNRESOLVED:pm-qa |
| open | pm-scrum-master | high | Sweep remaining violations mutation handlers for the 403-vs-404 cross-tenant contract (#3187 revealed assign still returns 403) — pattern audit across moderation.rs/aml_dsa.rs after PR #3183 fixed most but not assign | UNRESOLVED:pm-security |
| open | pm-backend | low | KMP realtor SSO error-body parsing is unguarded — crashes on non-JSON response |  |
| open | pm-scrum-master | high | Promote a non-mobile-native code-review vector from the next Phase 1.5 segment (ppt-web-core / reality-server oldest = 2026-11-04) to refill the planner buffer |  |
| open | pm-qa | medium | QA audit: non-mobile-native backlog items with confidence=medium need a one-shot verification step before Phase 3 can promote them |  |
| open | pm-backend | low | reality web analytics no i18n |  |
| open | pm-backend | low | api core cors localhost prod default |  |
| open | pm-backend | low | ppt web ui portfolio dashboard no i18n |  |
| open | pm-backend | low | api core tenant filter dead stub |  |
| open | pm-backend | low | reality web authapi unwrapped json parse |  |
| open | pm-backend | low | reality web comparison ctx unvalidated parse |  |
| open | pm-backend | low | ppt web ui authcallback raw err message |  |
| open | pm-backend | low | ppt web ui confirmdialog loading focustrap |  |
| open | pm-backend | low | api core pushfanout queue drops transient |  |
| open | pm-backend | low | mobile rn meter photo not uploaded |  |
| open | pm-backend | low | mobile rn naive en pluralization |  |
| open | pm-backend | low | ppt web core meters allmeters false empty |  |
| open | pm-backend | low | reality server realtor respond no validation |  |
| open | pm-backend | low | ppt web ui categorypref hardcoded labels |  |
| open | pm-backend | low | ppt web ui disableall dialog no i18n |  |
| open | pm-backend | low | reality server favorites limit toctou |  |
| open | pm-backend | low | reality web sso probe caches transient blip |  |
| open | pm-tech-lead | low | api core syndication jobs never drained |  |
| open | pm-backend | low | reality server branding coalesce no clear |  |
| open | pm-tech-lead | low | reality server url validator triplicated |  |
| open | pm-backend | low | api core jwt no aud iss |  |
| open | pm-backend | low | api core retention days no floor |  |
| open | pm-backend | low | ppt web ui rentals load error unhandled |  |
| open | pm-backend | low | reality web anchor nav drops locale |  |
| open | pm-backend | low | mobile rn feedback device info i18n |  |
| open | pm-qa | low | mobile rn states untested hardcoded |  |
| open | pm-tech-lead | low | mobile rn sync notifications dead hook |  |
| open | pm-backend | low | ppt web ui critical notif no i18n |  |
| open | pm-backend | low | reality web authbutton hardcoded en |  |
| open | pm-backend | low | reality web csvimport fileselect noval |  |
| open | pm-backend | low | reality web date tolocale no locale |  |
| open | pm-tech-lead | low | reality web listing edit i18n |  |
| open | pm-devops | medium | Add scheduled (nightly cron) smoke of `SKIP_NETWORK=1 ./.claude/skills/verify-all.sh --quick` on `dev` HEAD so silent infra-only regressions surface without waiting for a PR — verify-all is the auto-fix gate but has no scheduled sweep (re-emit of pm-devops 2026-08-31 action) |  |
| open | pm-tech-lead | medium | Drive a reviewer decision on #2555 (invoice lifecycle) and #2559 (PAY by square QR) — open 103d with zero reviewDecision, both authored by owner. Either assign a reviewer this sprint, break the PRs down for faster review, or formally defer to post-MVP |  |
| open | pm-devops | low | Resolve the two stuck expo-57 dependabot PRs #2588 (expo-status-bar) and #2590 (expo-device) — 103d idle on `dev`; expo SDK 57 bump already landed via sibling PRs this sprint. Either rebase+merge or close as superseded |  |
