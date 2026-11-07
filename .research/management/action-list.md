# PPT Action List

_Generated: 2026-11-07T11:00:00Z (routine Phase 1.6)_

**Open: 15** · Total: 25

| ID | Owner | Priority | Status | Action |
|---|---|---|---|---|
| `pm-devops-unblock-mobile-native-cloud-builds` | pm-devops | high | open | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimabl |
| `code-review-mobile-native-kmp-create-listing-not-wired` | pm-backend | medium | open | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded |
| `code-review-mobile-native-kmp-inquiries-response-contract` | pm-backend | medium | open | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on ever |
| `code-review-mobile-native-kmp-http-client-no-timeout` | pm-backend | low | open | Shared Ktor HttpClient has no HttpTimeout - mobile API calls can hang indefinitely |
| `code-review-mobile-native-kmp-my-listings-pagination-cap` | pm-backend | low | open | Reality mobile MyListings + portfolio analytics silently cap at 100 listings (no pagination) |
| `pm-frontend-mobile-rn-lint-gates-adopt-2026-10-28` | pm-frontend | medium | open | Adopt eslint-plugin-react-hooks + i18next/no-literal-string in frontend/apps/mobile (blocking, warn-only for one sprint) |
| `pm-scrum-master-violations-xtenant-404-contract-sweep-2026-10-28` | pm-scrum-master | high | open | Sweep remaining violations mutation handlers for the 403-vs-404 cross-tenant contract (#3187 revealed assign still retur |
| `code-review-mobile-native-kmp-sso-error-body-unguarded` | pm-backend | low | open | KMP realtor SSO error-body parsing is unguarded — crashes on non-JSON response |
| `code-review-ppt-web-core-refresh-token-localstorage` | pm-security | low | open | SECURITY: ppt-web refresh-token persisted in localStorage — XSS-reachable |
| `gh-issue-3283` | pm-tech-lead | medium | in-progress | Follow-up: realtor public profile page renders hardcoded Slovak UI on a 6-language portal (PR #3225) (Closes #3283) |
| `code-review-reality-server-inquiry-dead-fk-classify` | pm-tech-lead | low | open | InquiriesHandler splits ListingNotFound vs RealtorNotFound by error substring, but both collapse to the same 404 at the  |
| `gh-issue-3304` | pm-tech-lead | medium | in-progress | Follow-up: member-role escalation gate reimplements role parsing instead of reusing common::TenantRole::from_role_type ( |
| `gh-issue-3305` | pm-tech-lead | medium | in-progress | Follow-up: fold IoT org-scope check into the insert on the hot add_reading path instead of a separate SELECT EXISTS (PR  |
| `gh-issue-3306` | pm-tech-lead | medium | in-progress | Follow-up: consolidate per-file require_manager(rls) helpers onto the shared rls_authz::require_manager_rls (PR #3287) ( |
| `gh-issue-3307` | pm-tech-lead | medium | in-progress | Follow-up: fault update/triage/status handlers manually guard org on an id-only query instead of using find_by_id_for_or |
| `gh-issue-3308` | pm-tech-lead | medium | in-progress | Follow-up: add RLS-aware repo variants for vote sub-resource reads instead of pool-bypass + manual vote_in_org guard (PR |
| `gh-issue-3309` | pm-tech-lead | medium | in-progress | Follow-up: cs.json/sk.json were reordered (~9k-line churn) instead of appended; enforce stable key order for messages/*. |
| `gh-issue-3310` | pm-tech-lead | medium | in-progress | Follow-up: community feed shows every author as "Neighbor" (API returns ids only); formatMoney coerces Decimal via Numbe |
| `screen-map-drift-pr-3293-reality` | pm-qa | low | open | Screen-map drift: PR #3293 i18n'd reality-web realtor/[id] page without updating docs/screens/reality/realtor-detail.md |
| `code-review-ppt-web-ui-registry-no-i18n` | pm-tech-lead | low | done | ppt-web registry feature (pet/vehicle forms + lists) ships hardcoded English placeholders in a 4-locale app — 0 useTrans |
| `code-review-reality-web-share-cmp-unchecked-json` | pm-backend | low | done | reality-web shared-comparison loader casts the listings API body with an unchecked `as Promise<ListingSummary>` — no sha |
| `pm-qa-reality-web-listing-boundary-coverage-2026-11-07` | pm-qa | medium | open | Extend reality-web parseListingSummary coverage — add negative-case tests for the two surviving holes (summary with null |
| `pm-qa-ppt-web-registry-i18n-parity-lock-2026-11-07` | pm-qa | low | open | Add a localeNamespaceParity-style assertion for the new registry.{status,actions,pagination,pet,vehicle} leaf trees acro |
| `pm-qa-dispatcher-ig3-tdd-gate-enforce-2026-11-07` | pm-qa | medium | open | Formalize the IG3 failing-on-main regression-test gate for dispatcher-spawned PRs — PR #3350 modelled it (two-commit TDD |
| `pm-qa-screen-map-drift-sweep-2026-11-07` | pm-qa | low | open | Sweep the five open from-merged-review follow-ups (#3304-#3310) for screen-map drift — none reference docs/screens/ upda |
