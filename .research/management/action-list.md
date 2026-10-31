# Action list

_Generated: 2026-10-31T03:14:30Z_

| ID | Owner | Priority | Status | Action |
|---|---|---|---|---|
| `pm-devops-unblock-mobile-native-cloud-builds` | pm-devops | high | open | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimable in cloud; now compounded by #2966 so claimable=9/72 (buffer-low) |
| `screen-map-drift-pr-2994-ppt` | pm-qa | low | open | ppt-web route refactor (PR #2994) landed without touching docs/screens/ppt |
| `code-review-api-handlers-violations-number-count-race` | pm-backend | low | open | violation_number / appeal_number generated via COUNT(*)+1 on the pool (no tx/sequence) collides with UNIQUE(org, number) under concurrency → opaque 500 on create |
| `gh-issue-3104` | pm-tech-lead | medium | open | Portfolio/owner analytics: comparisons + dashboard summary + owner portfolio runtime drift (500s) (Closes #3104) |
| `gh-issue-3116` | pm-tech-lead | medium | open | Follow-up: NewsScreen category badge lacks the i18n defaultValue fallback that ArticleDetailScreen uses (PR #3110) (Closes #3116) |
| `code-review-mobile-native-kmp-create-listing-not-wired` | pm-backend | medium | open | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded |
| `code-review-mobile-native-kmp-inquiries-response-contract` | pm-backend | medium | open | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries + /realtors/inquiries call |
| `code-review-mobile-native-kmp-http-client-no-timeout` | pm-backend | low | open | Shared Ktor HttpClient has no HttpTimeout - mobile API calls can hang indefinitely |
| `code-review-mobile-native-kmp-my-listings-pagination-cap` | pm-backend | low | open | Reality mobile MyListings + portfolio analytics silently cap at 100 listings (no pagination) |
| `gh-issue-3164` | pm-tech-lead | medium | open | Follow-up: violations add_evidence left ungated while delete_evidence is manager-gated (PR #3072) (Closes #3164) |
| `pm-backend-violations-add-evidence-authz-gap-3164` | pm-backend | high | open | Gate violations add_evidence sub-resource with authz check — currently ungated allowing any authenticated user to attach evidence to any violation record (gh #3164) |
| `churn-hotspot-backend/servers/api-server/tests/suites/violations_role_authz_tests.rs` | pm-qa | low | open | Churn hotspot: backend/servers/api-server/tests/suites/violations_role_authz_tests.rs — merged PR touched this file heavily this run |
| `code-review-mobile-rn-vote-detail-dead-i18n-fb` | pm-frontend | low | open | VoteDetailScreen uses `t('key') ?? 'English'` throughout (L180-354) — i18next t() never returns nullish (returns the key string on a miss), so the English fallbacks are dead code and a missing locale  |
| `pm-frontend-mobile-rn-lint-gates-adopt-2026-10-28` | pm-frontend | medium | open | Adopt eslint-plugin-react-hooks + i18next/no-literal-string in frontend/apps/mobile (blocking, warn-only for one sprint) — closes the chronic class-of-defect behind the carried mobile-rn lint-gap risk |
| `pm-scrum-master-violations-xtenant-404-contract-sweep-2026-10-28` | pm-scrum-master | high | done | Sweep remaining violations mutation handlers for the 403-vs-404 cross-tenant contract (#3187 revealed assign still returns 403) — pattern audit across moderation.rs/aml_dsa.rs after PR #3183 fixed mos |
| `code-review-api-handlers-appeal-no-status-guard` | pm-backend | low | open | api-handlers file_appeal has no status guard → owner can re-file after a final appeal decision |
| `code-review-api-handlers-moderation-report-flood` | pm-backend | low | open | api-handlers /moderation/report open-to-all + no dedup/rate-limit → unbounded case rows per content |
| `code-review-ppt-web-ui-invoice-vat-rate-payload-drift` | pm-backend | low | open | ppt-web AccountingInvoiceForm persists stale item.vatRate=21 while UI shows country/type-derived rate — financial drift |
| `code-review-reality-server-os-rng-expect-panics` | pm-backend | low | open | reality-server OS-RNG failure panics password-reset + SSO authorize paths (.expect on try_fill_bytes) |
| `code-review-reality-web-listing-edit-price-zero` | pm-backend | low | open | reality-web listing edit handleSave coerces empty price to 0 via `Number(form.price) \|\| 0` — silent price=0 persist |
| `churn-hotspot-backend/servers/api-server/src/routes/violations.rs` | pm-qa | low | open | Churn hotspot: backend/servers/api-server/src/routes/violations.rs — 3 touches in 14d (role/role-authz/IDOR patches) |
| `churn-hotspot-backend/servers/api-server/tests/suites/compliance_wave1b_tests.rs` | pm-qa | low | open | Churn hotspot: backend/servers/api-server/tests/suites/compliance_wave1b_tests.rs — 3 touches in 14d (iterative compliance hardening) |
| `code-review-ppt-web-ui-connstatus-div-as-button-role` | pm-tech-lead | low | open | ppt-web ConnectionStatus renders <div role="status"> as the reconnect button — a11y role mismatch |
| `code-review-ppt-web-ui-invoice-feature-no-i18n` | pm-tech-lead | low | open | ppt-web Accounting Invoice feature ships entirely in hardcoded English — no i18n hook, uses native alert/confirm |
| `code-review-ppt-web-ui-toast-setstate-after-unmount` | pm-backend | low | open | ppt-web Toast ToastItem schedules setCopied timer with no unmount cleanup — dangling timer / stale state |
| `code-review-reality-server-portal-listing-price-unvalidated` | pm-backend | low | open | reality-server create/update_listing accepts price without positivity / NaN guard (DECIMAL has no CHECK) |
| `code-review-reality-server-reports-status-filter-unvalidated` | pm-devops | low | open | reality-server list_my_reports binds unvalidated status filter — silent empty page vs documented enum |
| `code-review-reality-web-comparison-parse-shape` | pm-backend | low | open | reality-web comparison-context JSON.parse(localStorage) with no shape check — tampered value crashes .some(...) |
| `code-review-reality-web-listing-edit-i18n-sk` | pm-backend | low | open | reality-web listing-edit page hardcodes Slovak user-facing strings — i18n gap on cs/de/en locales |
| `pm-scrum-master-human-review-sweep-6-prs-2026-10-31` | pm-scrum-master | high | open | Human review pass on the 6 merged needs-human-review PRs #3185/#3186/#3188/#3189/#3190/#3191, prioritising #3188 (security IDOR contract change) and #3190 (session leak) |
| `pm-scrum-master-violations-prs-sequence-3171-3075` | pm-scrum-master | high | open | Land #3171 (add_evidence manager role gate), then rebase and finish #3075 (violations number-count race) sequentially; serialize merges on violations.rs |
| `pm-scrum-master-triage-security-drafts-3192-3193-3198` | pm-scrum-master | high | open | Triage security-flavoured drafts #3192 (appeal no status guard), #3193 (moderation report flood), #3198 (os-rng expect panics): mark ready and request review |
| `pm-scrum-master-resolve-3108-portfolio-summary-500` | pm-scrum-master | medium | open | Resolve #3108 (owner-analytics portfolio-summary 500 schema drift): fix migration/query mismatch and un-draft |
| `pm-scrum-master-decision-3119-and-finish-3197` | pm-scrum-master | medium | open | Get reviewer decision on #3119 (NewsScreen i18n defaultValue) and un-draft/finish #3197 (reality-web listing-edit price-zero) |
| `pm-scrum-master-churn-hotspot-investigation-2026-10-31` | pm-scrum-master | low | open | Investigate churn hotspots (reality-web agency/[slug]/page.tsx, deny.toml, migration.rs) for instability; run coverage scan to refresh planning |
| `pm-qa-mobile-native-kmp-runway-decision` | pm-qa | high | open | Route the two ready mobile-native-kmp plans (inquiries-response-contract, create-listing-not-wired) to a runner with Android egress, OR explicitly park them as 'operator-gated' so they stop counting a |
| `pm-qa-inquiries-jvm-contract-test` | pm-qa | high | open | Add a JSON-contract test for /inquiries (decode recorded server fixture with kotlinx-serialization) that runs in commonTest/JVM layer with no AGP; or a backend OpenAPI-vs-DTO contract check |
| `pm-qa-triage-3119-newsscreen-i18n` | pm-qa | medium | open | Triage #3119 (NewsScreen i18n defaultValue, 13d non-draft, no reviewDecision): request a review, merge, or convert to draft/close with a reason within 2 days |
| `pm-qa-regression-test-gating-drafts-3192-3193-3197-3198` | pm-qa | medium | open | Gate drafts #3192/#3193/#3197/#3198 on a regression-test checklist before ready-for-review (each needs failing-before/passing-after test) |
| `pm-qa-authz-matrix-violations-and-agency-page-tests` | pm-qa | medium | open | Add role×endpoint authz matrix tests for violations.rs (incl. 404-not-403 existence-leak); add page-level tests for reality-web agency/[slug]/page.tsx (SSR, locale, slug 404) |
| `pm-qa-schedule-open-test-gap-items` | pm-qa | low | open | Schedule open test-gap backlog (comparison-parse-shape localStorage tamper first; then toast-setstate-after-unmount, invoice-feature-no-i18n, listing-edit-i18n-sk) |
