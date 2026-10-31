# Action List

_Generated: 2026-10-31T12:00:00Z · 36 open / 36 total_

| ID | Priority | Owner | Dependency | Action |
|---|---|---|---|---|
| `pm-backend-violations-add-evidence-authz-gap-3164` | high | pm-backend | none | Gate violations add_evidence sub-resource with authz check — currently ungated allowing any authenticated user to attach evidence to any violation record (gh... |
| `pm-devops-unblock-mobile-native-cloud-builds` | high | pm-devops | none | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimable in cloud; now compounded by #2966 s... |
| `pm-qa-regression-test-audit-on-the-8-merged-pr-2026-10-31` | high | pm-qa | none | Regression-test audit on the 8 merged PRs this window: #3185 (useMemo stale-token), #3186 (WS hasGivenUp), #3188 (cross-tenant IDOR assign), #3189 (ranked ba... |
| `pm-qa-cross-tenant-idor-regression-test-suite--2026-10-31` | high | pm-qa | none | Cross-tenant IDOR regression-test suite — extend the pattern established by PR #3183 (violations_mutation_cross_org_idor_tests) to moderation.rs, aml_dsa.rs,... |
| `pm-scrum-master-violations-xtenant-404-contract-sweep-2026-10-28` | high | pm-scrum-master | pm-security | Sweep remaining violations mutation handlers for the 403-vs-404 cross-tenant contract (#3187 revealed assign still returns 403) — pattern audit across modera... |
| `pm-scrum-master-author-new-coverage-enumerate-phase-2-3--2026-10-31` | high | pm-scrum-master | none | Author new coverage: enumerate phase-2/3 epics beyond the 13 currently in coverage.json (49/49 done) — the planner is out of claimable backlog (dispatcher pa... |
| `code-review-mobile-native-kmp-create-listing-not-wired` | medium | pm-backend | None | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded |
| `code-review-mobile-native-kmp-inquiries-response-contract` | medium | pm-backend | None | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries + /realtors/inquiri... |
| `pm-frontend-mobile-rn-lint-gates-adopt-2026-10-28` | medium | pm-frontend | pm-qa | Adopt eslint-plugin-react-hooks + i18next/no-literal-string in frontend/apps/mobile (blocking, warn-only for one sprint) — closes the chronic class-of-defect... |
| `pm-qa-flaky-skip-audit-enumerate-ignore-skip-x-2026-10-31` | medium | pm-qa | none | Flaky-skip audit: enumerate `#[ignore]` + `.skip`/`xit` + `test.todo` across backend tests, frontend vitest, and jest-expo — produce a dashboard of silently-... |
| `pm-qa-define-release-readiness-checklist-for-t-2026-10-31` | medium | pm-qa | none | Define release-readiness checklist for the 'all-stories-done' state — because coverage is exhausted, the next release candidate decision needs an explicit QA... |
| `pm-qa-resolve-churn-hotspot-qa-findings-now-3--2026-10-31` | medium | pm-qa | none | Resolve churn-hotspot QA findings now 3+ runs stale: violations_role_authz_tests.rs, violations.rs, compliance_wave1b_tests.rs — convert each into a positive... |
| `pm-scrum-master-promote-4-non-dependabot-draft-prs-3171--2026-10-31` | medium | pm-scrum-master | none | Promote 4 non-dependabot draft PRs (#3171 / #3075 / #3108 / #3192 / #3193) — pick next-steps for each: close, revive, or convert to a plan |
| `pm-scrum-master-clear-remaining-violations-xtenant-404-s-2026-10-31` | medium | pm-scrum-master | none | Clear remaining violations xtenant-404 sweep across moderation.rs / aml_dsa.rs (parent action still open after #3188 fixed assign_violation only) |
| `gh-issue-3104` | medium | pm-tech-lead | None | Portfolio/owner analytics: comparisons + dashboard summary + owner portfolio runtime drift (500s) (Closes #3104) |
| `gh-issue-3116` | medium | pm-tech-lead | None | Follow-up: NewsScreen category badge lacks the i18n defaultValue fallback that ArticleDetailScreen uses (PR #3110) (Closes #3116) |
| `gh-issue-3164` | medium | pm-tech-lead | None | Follow-up: violations add_evidence left ungated while delete_evidence is manager-gated (PR #3072) (Closes #3164) |
| `code-review-api-handlers-violations-number-count-race` | low | pm-backend | None | violation_number / appeal_number generated via COUNT(*)+1 on the pool (no tx/sequence) collides with UNIQUE(org, number) under concurrency → opaque 500 on cr... |
| `code-review-mobile-native-kmp-http-client-no-timeout` | low | pm-backend | None | Shared Ktor HttpClient has no HttpTimeout - mobile API calls can hang indefinitely |
| `code-review-mobile-native-kmp-my-listings-pagination-cap` | low | pm-backend | None | Reality mobile MyListings + portfolio analytics silently cap at 100 listings (no pagination) |
| `code-review-api-handlers-appeal-no-status-guard` | low | pm-backend | None | api-handlers file_appeal has no status guard → owner can re-file after a final appeal decision |
| `code-review-api-handlers-moderation-report-flood` | low | pm-backend | None | api-handlers /moderation/report open-to-all + no dedup/rate-limit → unbounded case rows per content |
| `code-review-ppt-web-ui-invoice-vat-rate-payload-drift` | low | pm-backend | None | ppt-web AccountingInvoiceForm persists stale item.vatRate=21 while UI shows country/type-derived rate — financial drift |
| `code-review-ppt-web-ui-toast-setstate-after-unmount` | low | pm-backend | None | ppt-web Toast ToastItem schedules setCopied timer with no unmount cleanup — dangling timer / stale state |
| `code-review-reality-server-portal-listing-price-unvalidated` | low | pm-backend | None | reality-server create/update_listing accepts price without positivity / NaN guard (DECIMAL has no CHECK) |
| `code-review-reality-web-comparison-parse-shape` | low | pm-backend | None | reality-web comparison-context JSON.parse(localStorage) with no shape check — tampered value crashes .some(...) |
| `code-review-reality-web-listing-edit-i18n-sk` | low | pm-backend | None | reality-web listing-edit page hardcodes Slovak user-facing strings — i18n gap on cs/de/en locales |
| `code-review-reality-server-reports-status-filter-unvalidated` | low | pm-devops | None | reality-server list_my_reports binds unvalidated status filter — silent empty page vs documented enum |
| `code-review-mobile-rn-vote-detail-dead-i18n-fb` | low | pm-frontend | None | VoteDetailScreen uses `t('key') ?? 'English'` throughout (L180-354) — i18next t() never returns nullish (returns the key string on a miss), so the English fa... |
| `screen-map-drift-pr-2994-ppt` | low | pm-qa | None | ppt-web route refactor (PR #2994) landed without touching docs/screens/ppt |
| `churn-hotspot-backend/servers/api-server/tests/suites/violations_role_authz_tests.rs` | low | pm-qa | None | Churn hotspot: backend/servers/api-server/tests/suites/violations_role_authz_tests.rs — merged PR touched this file heavily this run |
| `churn-hotspot-backend/servers/api-server/src/routes/violations.rs` | low | pm-qa | None | Churn hotspot: backend/servers/api-server/src/routes/violations.rs — 3 touches in 14d (role/role-authz/IDOR patches) |
| `churn-hotspot-backend/servers/api-server/tests/suites/compliance_wave1b_tests.rs` | low | pm-qa | None | Churn hotspot: backend/servers/api-server/tests/suites/compliance_wave1b_tests.rs — 3 touches in 14d (iterative compliance hardening) |
| `pm-qa-screen-map-drift-pr-2994-ppt-web-route-r-2026-10-31` | low | pm-qa | none | Screen-map drift PR #2994 (ppt-web route refactor) — carried from previous rotation; the frontmatter still needs reconciling against the current routes |
| `code-review-ppt-web-ui-connstatus-div-as-button-role` | low | pm-tech-lead | None | ppt-web ConnectionStatus renders <div role="status"> as the reconnect button — a11y role mismatch |
| `code-review-ppt-web-ui-invoice-feature-no-i18n` | low | pm-tech-lead | None | ppt-web Accounting Invoice feature ships entirely in hardcoded English — no i18n hook, uses native alert/confirm |
