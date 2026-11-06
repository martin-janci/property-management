# PPT Action List

_Generated: 2026-11-06T03:15:20Z (routine Phase 1.6)_

**Open: 29** · Total: 43

| ID | Owner | Priority | Status | Action |
|---|---|---|---|---|
| `pm-devops-unblock-mobile-native-cloud-builds` | pm-devops | high | open | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimable in cloud; now c... |
| `pm-qa-accounting-by-id-regression-tests` | pm-qa | high | open | Add regression tests for accounting-server by-id endpoints — invoice, contact, match reads must return 404 on cross-tenant id (matching t... |
| `pm-qa-onboarding-resolved-tenant-regression` | pm-qa | high | open | Add onboarding ResolvedTenant regression — POST /onboarding must consult ResolvedTenant, not the X-Tenant header; a foreign org header re... |
| `pm-qa-webhook-headers-auth-smuggling-tests` | pm-qa | high | open | Add webhook-headers auth-smuggling test — EnhancedWebhookSubscription create/update must reject or strip Authorization/Cookie/X-API-Key e... |
| `pm-scrum-master-accounting-server-rls-only-by-id-retire` | pm-scrum-master | high | open | Retire the accounting-server RLS-only by-id methods (noted 'Still open' in #3332) — move accounting reads onto the tenant-id-predicated b... |
| `pm-scrum-master-onboarding-resolved-tenant-rule` | pm-scrum-master | high | open | Apply the ResolvedTenant rule to onboarding so role selection cannot be influenced by a foreign tenant header (noted 'Still open' in #3332) |
| `pm-scrum-master-violations-xtenant-404-contract-sweep-2026-10-28` | pm-scrum-master | high | open | Sweep remaining violations mutation handlers for the 403-vs-404 cross-tenant contract (#3187 revealed assign still returns 403) — pattern... |
| `pm-scrum-master-webhook-headers-auth-smuggling` | pm-scrum-master | high | open | Harden EnhancedWebhookSubscription.headers so auth headers cannot be smuggled into the serialized schema (noted 'Still open' in #3332) — ... |
| `code-review-mobile-native-kmp-create-listing-not-wired` | pm-backend | medium | open | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded |
| `code-review-mobile-native-kmp-inquiries-response-contract` | pm-backend | medium | open | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries... |
| `pm-frontend-mobile-rn-lint-gates-adopt-2026-10-28` | pm-frontend | medium | open | Adopt eslint-plugin-react-hooks + i18next/no-literal-string in frontend/apps/mobile (blocking, warn-only for one sprint) — closes the chr... |
| `pm-qa-audit-logs-normal-op-delete-regression` | pm-qa | medium | open | Add db-test suite 4 or 5 regression for audit_logs ON DELETE SET NULL conflict in normal-operation delete paths (not just seed --force) —... |
| `pm-qa-mobile-rn-alert-i18n-regression` | pm-qa | medium | open | Add i18n regression for mobile RN Alert.alert hardcoded strings — DocumentDetailScreen.tsx:118, WidgetSettingsScreen.tsx:66/79, FeedbackS... |
| `pm-scrum-master-audit-logs-on-delete-set-null-adr` | pm-scrum-master | medium | open | Design-decide the audit_logs ON DELETE SET NULL FK conflict with the append-only trigger (noted in #3330) — either drop the SET NULL sema... |
| `pm-scrum-master-round-5-regression-ci-required-checks` | pm-scrum-master | medium | open | Automate the Round-5 regression suite at CI level — add check-rls-enforcement.sh --strict and the round5_sec_b_authz_tests + suite_7 prop... |
| `churn-hotspot-backend/servers/api-server/tests/suites/auth_tests.rs` | pm-tech-lead | low | open | Churn hotspot (new): backend/servers/api-server/tests/suites/auth_tests.rs is a top-3 churn file this run — watch for instability |
| `churn-hotspot-backend/servers/reality-server/src/util/errors.rs` | pm-tech-lead | low | open | Churn hotspot (new): backend/servers/reality-server/src/util/errors.rs is a top-3 churn file this run — watch for instability |
| `code-review-mobile-native-kmp-http-client-no-timeout` | pm-backend | low | open | Shared Ktor HttpClient has no HttpTimeout - mobile API calls can hang indefinitely |
| `code-review-mobile-native-kmp-my-listings-pagination-cap` | pm-backend | low | open | Reality mobile MyListings + portfolio analytics silently cap at 100 listings (no pagination) |
| `code-review-mobile-native-kmp-sso-error-body-unguarded` | pm-backend | low | open | KMP realtor SSO error-body parsing is unguarded — crashes on non-JSON response |
| `code-review-ppt-web-core-refresh-token-localstorage` | pm-security | low | open | SECURITY: ppt-web refresh-token persisted in localStorage — XSS-reachable |
| `code-review-ppt-web-ui-lang-switcher-aria` | pm-tech-lead | low | open | ppt-web LanguageSwitcher has hardcoded English aria-label on the control a non-English screen-reader user would use to find their language |
| `code-review-ppt-web-ui-marketplace-no-i18n` | pm-tech-lead | low | open | ppt-web marketplace feature ships hardcoded English (placeholders, 16 service-category labels) in a 4-locale app — 0 useTranslation acros... |
| `code-review-ppt-web-ui-registry-no-i18n` | pm-tech-lead | low | open | ppt-web registry feature (pet/vehicle forms + lists) ships hardcoded English placeholders in a 4-locale app — 0 useTranslation on 4 files |
| `code-review-reality-server-inquiry-dead-fk-classify` | pm-tech-lead | low | open | InquiriesHandler splits ListingNotFound vs RealtorNotFound by error substring, but both collapse to the same 404 at the route - dead, bri... |
| `code-review-reality-web-share-cmp-unchecked-json` | pm-backend | low | open | reality-web shared-comparison loader casts the listings API body with an unchecked `as Promise<ListingSummary>` — no shape validation, un... |
| `screen-map-drift-pr-3293-reality` | pm-qa | low | open | Screen-map drift: PR #3293 i18n'd reality-web realtor/[id] page without updating docs/screens/reality/realtor-detail.md |
| `screen-map-drift-pr-3317-ppt` | pm-qa | low | open | Screen-map drift: PR #3317 biome-cleaned ppt-web ai-dashboards route test without updating docs/screens/ppt/*.md |
| `screen-map-drift-pr-3317-reality` | pm-qa | low | open | Screen-map drift: PR #3317 biome-cleaned reality-web favorites/report/layout-revalidate route files without updating docs/screens/reality... |
| `gh-issue-3283` | pm-tech-lead | medium | in-progress | Follow-up: realtor public profile page renders hardcoded Slovak UI on a 6-language portal (PR #3225) (Closes #3283) |
| `gh-issue-3304` | pm-tech-lead | medium | in-progress | Follow-up: member-role escalation gate reimplements role parsing instead of reusing common::TenantRole::from_role_type (PR #3291) (Closes... |
| `gh-issue-3305` | pm-tech-lead | medium | in-progress | Follow-up: fold IoT org-scope check into the insert on the hot add_reading path instead of a separate SELECT EXISTS (PR #3289) (Closes #3... |
| `gh-issue-3306` | pm-tech-lead | medium | in-progress | Follow-up: consolidate per-file require_manager(rls) helpers onto the shared rls_authz::require_manager_rls (PR #3287) (Closes #3306) |
| `gh-issue-3307` | pm-tech-lead | medium | in-progress | Follow-up: fault update/triage/status handlers manually guard org on an id-only query instead of using find_by_id_for_org (PR #3286) (Clo... |
| `gh-issue-3308` | pm-tech-lead | medium | in-progress | Follow-up: add RLS-aware repo variants for vote sub-resource reads instead of pool-bypass + manual vote_in_org guard (PR #3281) (Closes #... |
| `gh-issue-3309` | pm-tech-lead | medium | in-progress | Follow-up: cs.json/sk.json were reordered (~9k-line churn) instead of appended; enforce stable key order for messages/*.json (PR #3260) (... |
| `gh-issue-3310` | pm-tech-lead | medium | in-progress | Follow-up: community feed shows every author as "Neighbor" (API returns ids only); formatMoney coerces Decimal via Number() (PR #3269) (C... |
| `churn-hotspot-backend/servers/api-server/src/routes/suspend_cascade.rs` | pm-tech-lead | low | done | Churn hotspot (new): backend/servers/api-server/src/routes/suspend_cascade.rs is a top-3 churn file this run — watch for instability |
| `screen-map-drift-pr-3237-ppt` | pm-qa | low | done | Screen-map drift: PR #3237 restructured ppt-web app shell + landing without docs/screens/ppt/*.md update |
| `screen-map-drift-pr-3242-ppt` | pm-qa | low | done | Screen-map drift: PR #3242 localized financial/buildings/my-unit routes without docs/screens/ppt update |
| `screen-map-drift-pr-3244-ppt` | pm-qa | low | done | Screen-map drift: PR #3244 localized faults/news/settings/error pages without docs/screens/ppt update |
| `screen-map-drift-pr-3254-ppt` | pm-qa | low | done | Screen-map drift: PR #3254 rewired ppt-web rentals routes to /bookings/* without docs/screens/ppt update |
| `screen-map-drift-pr-3313-reality` | pm-qa | low | done | Screen-map drift: PR #3313 added regression tests for reality-web realtor/[id] without updating docs/screens/reality/realtor-detail.md |
