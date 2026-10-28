# PPT Action List

_Generated: 2026-10-28T11:00:00Z (routine Phase 1.6)_

**Open: 24** · Total: 24

| ID | Owner | Priority | Status | Action |
|---|---|---|---|---|
| `pm-devops-unblock-mobile-native-cloud-builds` | pm-devops | high | open | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimabl |
| `screen-map-drift-pr-2994-ppt` | pm-qa | low | open | ppt-web route refactor (PR #2994) landed without touching docs/screens/ppt |
| `code-review-api-handlers-violations-number-count-race` | pm-backend | low | open | violation_number / appeal_number generated via COUNT(*)+1 on the pool (no tx/sequence) collides with UNIQUE(org, number) |
| `gh-issue-3104` | pm-tech-lead | medium | open | Portfolio/owner analytics: comparisons + dashboard summary + owner portfolio runtime drift (500s) (Closes #3104) |
| `gh-issue-3116` | pm-tech-lead | medium | open | Follow-up: NewsScreen category badge lacks the i18n defaultValue fallback that ArticleDetailScreen uses (PR #3110) (Clos |
| `code-review-mobile-native-kmp-create-listing-not-wired` | pm-backend | medium | open | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded |
| `code-review-mobile-native-kmp-inquiries-response-contract` | pm-backend | medium | open | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on ever |
| `code-review-mobile-native-kmp-http-client-no-timeout` | pm-backend | low | open | Shared Ktor HttpClient has no HttpTimeout - mobile API calls can hang indefinitely |
| `code-review-mobile-native-kmp-my-listings-pagination-cap` | pm-backend | low | open | Reality mobile MyListings + portfolio analytics silently cap at 100 listings (no pagination) |
| `gh-issue-3164` | pm-tech-lead | medium | open | Follow-up: violations add_evidence left ungated while delete_evidence is manager-gated (PR #3072) (Closes #3164) |
| `pm-backend-violations-add-evidence-authz-gap-3164` | pm-backend | high | open | Gate violations add_evidence sub-resource with authz check — currently ungated allowing any authenticated user to attach |
| `churn-hotspot-backend/servers/api-server/tests/suites/violations_role_authz_tests.rs` | pm-qa | low | open | Churn hotspot: backend/servers/api-server/tests/suites/violations_role_authz_tests.rs — merged PR touched this file heav |
| `code-review-ppt-web-core-neighbors-stale-token-usememo` | pm-backend | low | open | useNeighbors + usePrivacySettings bake mount-time token into api (useMemo [] deps) — stale Bearer after refresh |
| `code-review-ppt-web-core-iot-ws-hasgivenup-dropped` | pm-devops | low | open | useIotWebSocket.hasGivenUp exposed but IotDashboardPageRoute drops it — give-up state has no UI path |
| `gh-issue-3187` | pm-security | high | open | Failing test on dev blocks backend CI: cross_org_assign_violation_is_not_found (assign returns 403, expects 404) (Closes |
| `code-review-mobile-rn-vote-detail-ranked-no-order` | pm-frontend | low | open | VoteDetailScreen renders `ranked` questions with the same unordered checkbox UI as multiple_choice (isMulti groups both  |
| `code-review-mobile-rn-vote-detail-dead-i18n-fb` | pm-frontend | low | open | VoteDetailScreen uses `t('key') ?? 'English'` throughout (L180-354) — i18next t() never returns nullish (returns the key |
| `pm-frontend-mobile-rn-report-fault-fake-submit-2026-10-28` | pm-frontend | high | open | Wire ReportFaultScreen.tsx handleSubmit to real /api/v1/faults POST — currently fakes with setTimeout(1500), fault repor |
| `pm-frontend-reality-web-sell-wizard-no-persist-2026-10-28` | pm-frontend | high | open | reality-web /sell wizard Publish button never POSTs to reality-server — every seller submission silently discarded; wire |
| `pm-frontend-ppt-web-authed-roots-session-leak-2026-10-28` | pm-frontend | high | open | ppt-web logout() AUTHED_QUERY_KEY_ROOTS allow-list missing mounted route groups — TanStack caches leak across users on a |
| `pm-frontend-mobile-rn-lint-gates-adopt-2026-10-28` | pm-frontend | medium | open | Adopt eslint-plugin-react-hooks + i18next/no-literal-string in frontend/apps/mobile (blocking, warn-only for one sprint) |
| `pm-frontend-reality-web-share-comparison-404-2026-10-28` | pm-frontend | medium | open | reality-web ComparisonUrlHandler hits non-existent /api/listings/${id} — every shared comparison URL 404s; map to canoni |
| `pm-frontend-reality-web-listing-page-ssr-crash-2026-10-28` | pm-frontend | medium | open | reality-web listing detail SSR crashes on partial 200 body — JSON-LD build dereferences undefined fields; add defensive  |
| `pm-scrum-master-violations-xtenant-404-contract-sweep-2026-10-28` | pm-scrum-master | high | open | Sweep remaining violations mutation handlers for the 403-vs-404 cross-tenant contract (#3187 revealed assign still retur |
