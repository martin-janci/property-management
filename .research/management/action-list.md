# Action list

_Regenerated 2026-10-29T03:14:02Z from `action-list.json` — 41 items (41 open)._

| ID | Priority | Owner | Status | Dependency | Action |
|---|---|---|---|---|---|
| pm-devops-unblock-mobile-native-cloud-builds | high | pm-devops | open | none | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ op |
| screen-map-drift-pr-2994-ppt | low | pm-qa | open | none | ppt-web route refactor (PR #2994) landed without touching docs/screens/ppt |
| code-review-api-handlers-violations-number-count-race | low | pm-backend | open | none | violation_number / appeal_number generated via COUNT(*)+1 on the pool (no tx/seq |
| gh-issue-3104 | medium | pm-tech-lead | open | none | Portfolio/owner analytics: comparisons + dashboard summary + owner portfolio run |
| gh-issue-3116 | medium | pm-tech-lead | open | none | Follow-up: NewsScreen category badge lacks the i18n defaultValue fallback that A |
| code-review-mobile-native-kmp-create-listing-not-wired | medium | pm-backend | open | none | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form da |
| code-review-mobile-native-kmp-inquiries-response-contract | medium | pm-backend | open | none | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server |
| code-review-mobile-native-kmp-http-client-no-timeout | low | pm-backend | open | none | Shared Ktor HttpClient has no HttpTimeout - mobile API calls can hang indefinite |
| code-review-mobile-native-kmp-my-listings-pagination-cap | low | pm-backend | open | none | Reality mobile MyListings + portfolio analytics silently cap at 100 listings (no |
| gh-issue-3164 | medium | pm-tech-lead | open | none | Follow-up: violations add_evidence left ungated while delete_evidence is manager |
| pm-backend-violations-add-evidence-authz-gap-3164 | high | pm-backend | open | none | Gate violations add_evidence sub-resource with authz check — currently ungated a |
| churn-hotspot-backend/servers/api-server/tests/suites/violations_role_authz_tests.rs | low | pm-qa | open | none | Churn hotspot: backend/servers/api-server/tests/suites/violations_role_authz_tes |
| code-review-ppt-web-core-neighbors-stale-token-usememo | low | pm-backend | open | none | useNeighbors + usePrivacySettings bake mount-time token into api (useMemo [] dep |
| code-review-ppt-web-core-iot-ws-hasgivenup-dropped | low | pm-devops | open | none | useIotWebSocket.hasGivenUp exposed but IotDashboardPageRoute drops it — give-up  |
| gh-issue-3187 | high | pm-security | open | none | Failing test on dev blocks backend CI: cross_org_assign_violation_is_not_found ( |
| code-review-mobile-rn-vote-detail-ranked-no-order | low | pm-frontend | open | none | VoteDetailScreen renders `ranked` questions with the same unordered checkbox UI  |
| code-review-mobile-rn-vote-detail-dead-i18n-fb | low | pm-frontend | open | none | VoteDetailScreen uses `t('key') ?? 'English'` throughout (L180-354) — i18next t( |
| pm-frontend-ppt-web-authed-roots-session-leak-2026-10-28 | high | pm-frontend | open | none | ppt-web logout() AUTHED_QUERY_KEY_ROOTS allow-list missing mounted route groups  |
| pm-frontend-mobile-rn-lint-gates-adopt-2026-10-28 | medium | pm-frontend | open | pm-qa | Adopt eslint-plugin-react-hooks + i18next/no-literal-string in frontend/apps/mob |
| pm-scrum-master-violations-xtenant-404-contract-sweep-2026-10-28 | high | pm-scrum-master | open | pm-security | Sweep remaining violations mutation handlers for the 403-vs-404 cross-tenant con |
| code-review-api-handlers-appeal-no-status-guard | low | pm-backend | open | none | api-handlers file_appeal has no status guard → owner can re-file after a final a |
| code-review-api-handlers-moderation-report-flood | low | pm-backend | open | none | api-handlers /moderation/report open-to-all + no dedup/rate-limit → unbounded ca |
| code-review-ppt-web-ui-invoice-qty-price-nan | low | pm-backend | open | none | ppt-web AccountingInvoiceForm parseFloat on qty/unitPrice yields NaN, bypasses h |
| code-review-ppt-web-ui-invoice-vat-rate-payload-drift | low | pm-backend | open | none | ppt-web AccountingInvoiceForm persists stale item.vatRate=21 while UI shows coun |
| code-review-reality-server-os-rng-expect-panics | low | pm-backend | open | none | reality-server OS-RNG failure panics password-reset + SSO authorize paths (.expe |
| code-review-reality-web-listing-edit-price-zero | low | pm-backend | open | none | reality-web listing edit handleSave coerces empty price to 0 via `Number(form.pr |
| churn-hotspot-backend/servers/api-server/src/routes/violations.rs | low | pm-qa | open | none | Churn hotspot: backend/servers/api-server/src/routes/violations.rs — 3 touches i |
| churn-hotspot-backend/servers/api-server/tests/suites/compliance_wave1b_tests.rs | low | pm-qa | open | none | Churn hotspot: backend/servers/api-server/tests/suites/compliance_wave1b_tests.r |
| code-review-ppt-web-ui-connstatus-div-as-button-role | low | pm-tech-lead | open | none | ppt-web ConnectionStatus renders <div role="status"> as the reconnect button — a |
| code-review-ppt-web-ui-invoice-feature-no-i18n | low | pm-tech-lead | open | none | ppt-web Accounting Invoice feature ships entirely in hardcoded English — no i18n |
| code-review-ppt-web-ui-toast-setstate-after-unmount | low | pm-backend | open | none | ppt-web Toast ToastItem schedules setCopied timer with no unmount cleanup — dang |
| code-review-reality-server-portal-listing-price-unvalidated | low | pm-backend | open | none | reality-server create/update_listing accepts price without positivity / NaN guar |
| code-review-reality-server-reports-status-filter-unvalidated | low | pm-devops | open | none | reality-server list_my_reports binds unvalidated status filter — silent empty pa |
| code-review-reality-web-comparison-parse-shape | low | pm-backend | open | none | reality-web comparison-context JSON.parse(localStorage) with no shape check — ta |
| code-review-reality-web-listing-edit-i18n-sk | low | pm-backend | open | none | reality-web listing-edit page hardcodes Slovak user-facing strings — i18n gap on |
| pm-qa-land-3187-fix-cross-tenant-assign-404 | high | pm-qa | open | rust-backend | Treat #3187 (cross_org_assign_violation_is_not_found) as the backend CI's single |
| pm-qa-review-queue-aging-alarm | high | pm-qa | open | pm-devops | Add a dispatcher-queue aging alarm — if any status:review row in assignments.jso |
| pm-qa-add-evidence-authz-regression-test-3164 | medium | pm-qa | open | rust-backend | Add a failing-on-main regression test for the add_evidence authz gap (#3164) — R |
| pm-qa-audit-ig3-freshness-11-stacked-prs | medium | pm-qa | open | pm-tech-lead | Audit the 11 in-review auto-impl PRs for IG3-contract drift — each PR must still |
| pm-qa-promote-review-queue-aging-risk-dec002 | medium | pm-qa | open | pm-tech-lead | Promote the review-queue-aging pattern as a persistent risk — the merge pipeline |
| pm-qa-test-shadow-debt-line-in-synthesis | low | pm-qa | open | pm-scrum-master | Add a 'test-shadow debt' line to the Scrum Master synthesis — count of auto-impl |
