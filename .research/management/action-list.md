# PPT Action List

_Generated: 2026-10-31T05:00:00Z (routine Phase 1.6)_

**Open: 31** · Total: 33

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
| `code-review-mobile-rn-vote-detail-dead-i18n-fb` | pm-frontend | low | open | VoteDetailScreen uses `t('key') ?? 'English'` throughout (L180-354) — i18next t() never returns nullish (returns the key |
| `pm-frontend-mobile-rn-lint-gates-adopt-2026-10-28` | pm-frontend | medium | open | Adopt eslint-plugin-react-hooks + i18next/no-literal-string in frontend/apps/mobile (blocking, warn-only for one sprint) |
| `pm-scrum-master-violations-xtenant-404-contract-sweep-2026-10-28` | pm-scrum-master | high | open | Sweep remaining violations mutation handlers for the 403-vs-404 cross-tenant contract (#3187 revealed assign still retur |
| `code-review-api-handlers-appeal-no-status-guard` | pm-backend | low | open | api-handlers file_appeal has no status guard → owner can re-file after a final appeal decision |
| `code-review-api-handlers-moderation-report-flood` | pm-backend | low | open | api-handlers /moderation/report open-to-all + no dedup/rate-limit → unbounded case rows per content |
| `code-review-ppt-web-ui-invoice-vat-rate-payload-drift` | pm-backend | low | open | ppt-web AccountingInvoiceForm persists stale item.vatRate=21 while UI shows country/type-derived rate — financial drift |
| `code-review-reality-server-os-rng-expect-panics` | pm-backend | low | done | reality-server OS-RNG failure panics password-reset + SSO authorize paths (.expect on try_fill_bytes) |
| `code-review-reality-web-listing-edit-price-zero` | pm-backend | low | done | reality-web listing edit handleSave coerces empty price to 0 via `Number(form.price) \\|\\| 0` — silent price=0 persist |
| `churn-hotspot-backend/servers/api-server/src/routes/violations.rs` | pm-qa | low | open | Churn hotspot: backend/servers/api-server/src/routes/violations.rs — 3 touches in 14d (role/role-authz/IDOR patches) |
| `churn-hotspot-backend/servers/api-server/tests/suites/compliance_wave1b_tests.rs` | pm-qa | low | open | Churn hotspot: backend/servers/api-server/tests/suites/compliance_wave1b_tests.rs — 3 touches in 14d (iterative complian |
| `code-review-ppt-web-ui-connstatus-div-as-button-role` | pm-tech-lead | low | open | ppt-web ConnectionStatus renders <div role="status"> as the reconnect button — a11y role mismatch |
| `code-review-ppt-web-ui-invoice-feature-no-i18n` | pm-tech-lead | low | open | ppt-web Accounting Invoice feature ships entirely in hardcoded English — no i18n hook, uses native alert/confirm |
| `code-review-ppt-web-ui-toast-setstate-after-unmount` | pm-backend | low | open | ppt-web Toast ToastItem schedules setCopied timer with no unmount cleanup — dangling timer / stale state |
| `code-review-reality-server-portal-listing-price-unvalidated` | pm-backend | low | open | reality-server create/update_listing accepts price without positivity / NaN guard (DECIMAL has no CHECK) |
| `code-review-reality-server-reports-status-filter-unvalidated` | pm-devops | low | open | reality-server list_my_reports binds unvalidated status filter — silent empty page vs documented enum |
| `code-review-reality-web-comparison-parse-shape` | pm-backend | low | open | reality-web comparison-context JSON.parse(localStorage) with no shape check — tampered value crashes .some(...) |
| `code-review-reality-web-listing-edit-i18n-sk` | pm-backend | low | open | reality-web listing-edit page hardcodes Slovak user-facing strings — i18n gap on cs/de/en locales |
| `pm-qa-8a3-notification-prefs-gates-480-484` | pm-qa | high | open | Reopen or formally defer 8a-3-notification-preference-sync; triage issues #480 (WS token in query-param logged, no re-va |
| `pm-qa-expand-3184-authz-matrix-violations` | pm-qa | high | open | Expand draft #3184 beyond resident-deny: cross-org (tenant-B manager → 404), unauthenticated (401) and wrong-role (403)  |
| `pm-qa-ci-gate-root-cause-red-test-on-dev` | pm-qa | high | open | Root-cause why cross_org_assign_violation_is_not_found reached dev as RED despite backend.yml CI gate; audit suite regis |
| `pm-qa-sprint-status-reconciliation-dup-key` | pm-qa | medium | open | Reconcile _bmad-output/implementation-artifacts/sprint-status.yaml: duplicate '80-2-dispute-filing-flow' key, stale spri |
