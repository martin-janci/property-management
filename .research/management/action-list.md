# PPT Action List

_Generated: 2026-11-08T00:00:00Z (routine Phase 1.6)_

**Open: 18** · In-progress: 8 · Done this run: 2 · Total open+in-progress: 26

| ID | Owner | Priority | Status | Action |
|---|---|---|---|---|
| `pm-devops-unblock-mobile-native-cloud-builds` | pm-devops | high | open | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimable in cloud |
| `code-review-mobile-native-kmp-create-listing-not-wired` | pm-backend | medium | open | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded |
| `code-review-mobile-native-kmp-inquiries-response-contract` | pm-backend | medium | open | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries call |
| `code-review-mobile-native-kmp-http-client-no-timeout` | pm-backend | low | open | Shared Ktor HttpClient has no HttpTimeout — mobile API calls can hang indefinitely |
| `code-review-mobile-native-kmp-my-listings-pagination-cap` | pm-backend | low | open | Reality mobile MyListings + portfolio analytics silently cap at 100 listings (no pagination) |
| `pm-frontend-mobile-rn-lint-gates-adopt-2026-10-28` | pm-frontend | medium | open | Adopt eslint-plugin-react-hooks + i18next/no-literal-string in frontend/apps/mobile |
| `pm-scrum-master-violations-xtenant-404-contract-sweep-2026-10-28` | pm-scrum-master | high | open | Sweep remaining violations mutation handlers for the 403-vs-404 cross-tenant contract (#3187 revealed assign still returns 403) |
| `code-review-mobile-native-kmp-sso-error-body-unguarded` | pm-backend | low | open | KMP realtor SSO error-body parsing is unguarded — crashes on non-JSON response |
| `code-review-ppt-web-core-refresh-token-localstorage` | pm-security | low | open | SECURITY: ppt-web refresh-token persisted in localStorage — XSS-reachable |
| `gh-issue-3283` | pm-tech-lead | medium | in-progress | Follow-up: realtor public profile page renders hardcoded Slovak UI on a 6-language portal (PR #3225) (Closes #3283) |
| `code-review-reality-server-inquiry-dead-fk-classify` | pm-tech-lead | low | open | InquiriesHandler splits ListingNotFound vs RealtorNotFound by error substring, but both collapse to the same 404 |
| `gh-issue-3304` | pm-tech-lead | medium | in-progress | Follow-up: member-role escalation gate reimplements role parsing (PR #3291) (Closes #3304) |
| `gh-issue-3305` | pm-tech-lead | medium | in-progress | Follow-up: fold IoT org-scope check into the insert on the hot add_reading path (PR #3289) (Closes #3305) |
| `gh-issue-3306` | pm-tech-lead | medium | in-progress | Follow-up: consolidate per-file require_manager(rls) helpers onto rls_authz::require_manager_rls (PR #3287) (Closes #3306) |
| `gh-issue-3307` | pm-tech-lead | medium | in-progress | Follow-up: fault update/triage/status handlers manually guard org on an id-only query (PR #3286) (Closes #3307) |
| `gh-issue-3308` | pm-tech-lead | medium | in-progress | Follow-up: add RLS-aware repo variants for vote sub-resource reads (PR #3281) (Closes #3308) |
| `gh-issue-3309` | pm-tech-lead | medium | in-progress | Follow-up: cs.json/sk.json were reordered (~9k-line churn); enforce stable key order (PR #3260) (Closes #3309) |
| `gh-issue-3310` | pm-tech-lead | medium | in-progress | Follow-up: community feed shows every author as "Neighbor"; formatMoney Decimal via Number() (PR #3269) (Closes #3310) |
| `screen-map-drift-pr-3293-reality` | pm-qa | low | open | Screen-map drift: PR #3293 i18n'd reality-web realtor/[id] page without updating docs/screens/reality/realtor-detail.md |
| `code-review-reality-web-share-cmp-unchecked-json` | pm-backend | low | **done** | Resolved by PR #3350 (API-boundary validation of shared-comparison listing bodies) |
| `code-review-ppt-web-ui-registry-mutation-silent-fail` | pm-frontend | medium | open | **Re-opened** — PR #3356 closed-not-merged by dispatcher cleanup; RegistryPage destructive mutations still silent-fail in dev |
| `code-review-ppt-web-ui-registry-no-i18n` | pm-tech-lead | low | in-progress | ppt-web registry feature (pet/vehicle forms + lists) ships hardcoded English — covered by #3355 chrome i18n |
| `code-review-ppt-web-ui-registry-page-i18n-gap` | pm-frontend | low | **done** | Resolved by PR #3351 (registry pet/vehicle forms + lists through react-i18next) + PR #3355 (RegistryPage chrome) |
| `pm-qa-pr3353-multi-session-revocation-regression-2026-11-08` | pm-qa | high | open | Add multi-session revocation regression test for PR #3353 — OTHER refresh tokens revoked, CURRENT session preserved on /auth/change-password |
| `pm-qa-tier1d-signals-ingestion-gate-2026-11-08` | pm-qa | high | open | Add failing-on-main test gate for Tier-1d signal ingestion — 7 orphaned signals from 2026-11-07 (#3357) |
| `pm-qa-pr3350-shared-comparison-boundary-test-2026-11-08` | pm-qa | medium | open | Add reality-web shared-comparison body-validator unit test (PR #3350 surface) |
| `pm-qa-pr3354-registry-auth-context-regression-2026-11-08` | pm-qa | medium | open | Add RegistryPage auth-context regression test (PR #3354 surface) — same gate shape as gate #486 |
| `pm-qa-dispatcher-fix-pr-failing-test-gate-2026-11-08` | pm-qa | medium | open | Enforce dispatcher-fix-PR-requires-failing-test gate — carried from 2026-08-25 decision proposal, 75 days unresolved |
| `pm-qa-registry-i18n-missing-key-guard-2026-11-08` | pm-qa | low | open | Add i18n missing-key guard test for registry pet/vehicle forms (PR #3351/#3355 surface) |
