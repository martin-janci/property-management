# PPT Action List

_Generated: 2026-11-08T20:47:52Z (routine Phase 1.6)_

**Open: 17** · Total: 30

| ID | Owner | Priority | Status | Action |
|---|---|---|---|---|
| `code-review-ppt-web-ui-registry-mutation-silent-fail` | pm-frontend | medium | in-progress | ppt-web RegistryPage.tsx delete/review mutation handlers (L102-142, L171-211) swallow failures with only console.erro… |
| `gh-issue-3283` | pm-tech-lead | medium | in-progress | Follow-up: realtor public profile page renders hardcoded Slovak UI on a 6-language portal (PR #3225) (Closes #3283) |
| `gh-issue-3304` | pm-tech-lead | medium | in-progress | Follow-up: member-role escalation gate reimplements role parsing instead of reusing common::TenantRole::from_role_typ… |
| `gh-issue-3305` | pm-tech-lead | medium | in-progress | Follow-up: fold IoT org-scope check into the insert on the hot add_reading path instead of a separate SELECT EXISTS (… |
| `gh-issue-3306` | pm-tech-lead | medium | in-progress | Follow-up: consolidate per-file require_manager(rls) helpers onto the shared rls_authz::require_manager_rls (PR #3287… |
| `gh-issue-3307` | pm-tech-lead | medium | in-progress | Follow-up: fault update/triage/status handlers manually guard org on an id-only query instead of using find_by_id_for… |
| `gh-issue-3308` | pm-tech-lead | medium | in-progress | Follow-up: add RLS-aware repo variants for vote sub-resource reads instead of pool-bypass + manual vote_in_org guard … |
| `gh-issue-3309` | pm-tech-lead | medium | in-progress | Follow-up: cs.json/sk.json were reordered (~9k-line churn) instead of appended; enforce stable key order for messages… |
| `gh-issue-3310` | pm-tech-lead | medium | in-progress | Follow-up: community feed shows every author as "Neighbor" (API returns ids only); formatMoney coerces Decimal via Nu… |
| `code-review-ppt-web-ui-registry-no-i18n` | pm-tech-lead | low | in-progress | ppt-web registry feature (pet/vehicle forms + lists) ships hardcoded English placeholders in a 4-locale app — 0 useTr… |
| `code-review-ppt-web-ui-registry-page-i18n-gap` | pm-frontend | low | in-progress | ppt-web RegistryPage.tsx (L249-250,275,302,317-318) hardcodes English (title, subtitle, Pets/Vehicles tabs, load-erro… |
| `code-review-reality-web-share-cmp-unchecked-json` | pm-backend | low | in-progress | reality-web shared-comparison loader casts the listings API body with an unchecked `as Promise<ListingSummary>` — no … |
| `pm-devops-unblock-mobile-native-cloud-builds` | pm-devops | high | open | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaim… |
| `pm-qa-create-web-reality-server-coverage-vectors` | pm-frontend | high | open | Create web-only + reality-server coverage-gap vectors: shipped-without-sibling-test screens (ppt-web, reality-web), r… |
| `pm-qa-deflake-rentals-connections-test` | pm-frontend | high | open | Quarantine and deflake frontend/apps/ppt-web/src/routes/groups/rentals.connections.test.tsx (flake noted in PR #3356 … |
| `pm-scrum-master-reland-pr-3301-reality-server-fk-classify` | pm-backend | high | open | Re-land #3301 (reality-server inquiry FK classification via SQLSTATE 23503) as a fresh branch off current dev — close… |
| `pm-scrum-master-resolve-3357-dispatcher-pipeline-stall` | pm-devops | high | open | Resolve issue #3357 dispatcher pipeline stall — operator rebases or retires orphaned branches and fixes the Tier-1d s… |
| `pm-scrum-master-seed-nongated-web-realityserver-vectors` | pm-tech-lead | high | open | Seed fresh non-gated backlog vectors: reality-server and ppt-web code-review sweeps, refresh-token localStorage secur… |
| `pm-scrum-master-violations-xtenant-404-contract-sweep-2026-10-28` | pm-scrum-master | high | open | Sweep remaining violations mutation handlers for the 403-vs-404 cross-tenant contract (#3187 revealed assign still re… |
| `code-review-mobile-native-kmp-create-listing-not-wired` | pm-backend | medium | open | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded |
| `code-review-mobile-native-kmp-inquiries-response-contract` | pm-backend | medium | open | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on e… |
| `pm-frontend-mobile-rn-lint-gates-adopt-2026-10-28` | pm-frontend | medium | open | Adopt eslint-plugin-react-hooks + i18next/no-literal-string in frontend/apps/mobile (blocking, warn-only for one spri… |
| `pm-qa-define-cloud-testable-substitute-mobile-native` | pm-tech-lead | medium | open | Define a cloud-testable substitute for mobile-native-gated items: OpenAPI schema diff on generated Kotlin client, rea… |
| `pm-qa-reconcile-8a-3-sprint-status-gate` | pm-tech-lead | medium | open | Reconcile sprint-status 8a-3 status against the test-hardening batch gate rule: its gate issues #480 and #484 are now… |
| `pm-qa-review-pr-assertion-quality-2026-11-08` | pm-qa | medium | open | Review the 7 merged PRs this cycle for assertion quality (sample 2 of 7): verify #3353 session-revocation asserts rev… |
| `code-review-mobile-native-kmp-http-client-no-timeout` | pm-backend | low | open | Shared Ktor HttpClient has no HttpTimeout - mobile API calls can hang indefinitely |
| `code-review-mobile-native-kmp-my-listings-pagination-cap` | pm-backend | low | open | Reality mobile MyListings + portfolio analytics silently cap at 100 listings (no pagination) |
| `code-review-mobile-native-kmp-sso-error-body-unguarded` | pm-backend | low | open | KMP realtor SSO error-body parsing is unguarded — crashes on non-JSON response |
| `code-review-ppt-web-core-refresh-token-localstorage` | pm-security | low | open | SECURITY: ppt-web refresh-token persisted in localStorage — XSS-reachable |
| `screen-map-drift-pr-3293-reality` | pm-qa | low | done | Screen-map drift: PR #3293 i18n'd reality-web realtor/[id] page without updating docs/screens/reality/realtor-detail.md |
