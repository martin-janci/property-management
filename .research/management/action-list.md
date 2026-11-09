# PPT Action List

_Generated: 2026-11-09T12:00:00Z (routine Phase 1.6 — pm-qa rotation, dispatcher buffer-low refill via #3357)_

**Open: 47** · In-progress: 8 · Done: 4 · Total: 59

## Open

| ID | Owner | Priority | Action |
|---|---|---|---|
| `code-review-api-core-accounting-matcher-ignores-currency` | pm-backend | high | run_payment_matcher compares line.amount == invoice.amount purely numerically — never compares currencies; EUR 1000 corroborates CZK 1000 and can auto-suggest a cross-currency matc |
| `code-review-api-core-accounting-nonatomic-statement-ingest` | pm-backend | high | bank-statement ingest process_statement_upload() is non-atomic — parent bank_statement row persists before CSV line-insert loop; mid-loop failure leaves orphan + partial rows; wrap |
| `code-review-api-handlers-delegation-xorg-delegate` | pm-security | high | SECURITY: delegation.create — delegate_user_id validated only against global user_repo; owner can grant voting/financial scope to a user from ANY other org |
| `code-review-api-handlers-refresh-rotation-nonatomic` | pm-security | high | Refresh-token rotation: revoke_token runs before the new-token insert without a tx — crash window leaves user without a valid refresh token; wrap rotate in sqlx transaction |
| `code-review-api-handlers-unit-residents-add-xorg-user` | pm-security | high | SECURITY: unit.add_resident validates user_id globally only; manager can attach a user from a DIFFERENT org as resident of their unit — receives unit communications |
| `code-review-listings-rs-raw-db-error-leak` | pm-backend | high | listings.rs — ~36 call sites interpolate raw sqlx/repo error directly into client body via Json(ErrorResponse) — sweep to generic error + tracing log |
| `code-review-ppt-web-ui-formrenderer-required-bypass` | pm-frontend | high | forms/FormRenderer.tsx validateForm() required check accepts empty string / empty array as "present" (only null/undefined rejected) — required-field bypass; fix predicate |
| `code-review-reality-server-readiness-raw-db-error-leak` | pm-backend | high | reality-server check_database() returns raw sqlx::Error Display in DependencyHealth.error — unauthenticated /readiness; can expose DB host:port, user, SQLSTATE; mask + log |
| `gh-issue-3357` | pm-tech-lead | high | CRITICAL OPS: dispatcher pipeline stalled — 3 approved PRs orphaned from dev, Tier-1d signals never ingested into action-list.json; re-ingest signals, land orphans, add watchdog |
| `pm-devops-unblock-mobile-native-cloud-builds` | pm-devops | high | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimable in cloud; now compounded by #2966 so claimable=9/72 (buffe |
| `pm-qa-dispatcher-3357-watchdog-test-2026-11-09` | pm-qa | high | Add CI assertion / dispatcher self-test that every closed tier1d signal file has either a backlog row OR an action-list row (prevents the #3357 orphan-signal silent-stall from recu |
| `pm-qa-tier1d-security-no-regression-tests-2026-11-09` | pm-qa | high | Add cross-org regression test fixtures for delegation xorg + unit_residents xorg findings before implementers land them — same pattern as the 2026-07 inquiry/mark_as_read regressio |
| `pm-scrum-master-tier1d-ingestion-sweep-2026-11-09` | pm-scrum-master | high | Sweep 2026-11-04..08 tier1d signal files for stale orphans; ensure every signal has either a resolved backlog row OR a live action-list row (buffer-low #3357 root cause — ingestion |
| `pm-scrum-master-violations-xtenant-404-contract-sweep-2026-10-28` | pm-scrum-master | high | Sweep remaining violations mutation handlers for the 403-vs-404 cross-tenant contract (#3187 revealed assign still returns 403) — pattern audit across moderation.rs/aml_dsa.rs afte |
| `code-review-api-core-accounting-date-proximity-todo` | pm-backend | medium | run_payment_matcher TODO: date-proximity scoring factor missing — recurring same-VS/same-amount invoices have no tie-breaker; implement date-proximity weight |
| `code-review-api-core-workflow-exec-unbounded-spawn` | pm-backend | medium | execute_workflow() does an unbounded tokio::spawn per workflow — handle_event loops over list_by_trigger_type with no batch bound; gate behind a Semaphore |
| `code-review-api-handlers-rentals-raw-db-error-leak` | pm-backend | medium | rentals.rs report_err() interpolates raw sqlx::Error into INTERNAL_SERVER_ERROR body (generate_report, by-portfolio, by-property) — same class as listings.rs fix; mask + log |
| `code-review-api-handlers-signatures-raw-db-err-leak` | pm-backend | medium | Signature-request handlers return raw sqlx::Error Display in DATABASE_ERROR responses — same leak class as listings.rs; sweep to generic body + log detail |
| `code-review-listings-rs-unstructured-error-body` | pm-backend | medium | listings.rs 21 handlers return Result<T, (axum::http::StatusCode, String)> instead of the shared ErrorResponse envelope — UNIFY to typed envelope |
| `code-review-mobile-native-kmp-create-listing-not-wired` | pm-backend | medium | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded |
| `code-review-mobile-native-kmp-inquiries-response-contract` | pm-backend | medium | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries + /realtors/inquiries call |
| `code-review-ppt-web-core-privacy-route-unguarded` | pm-frontend | medium | /settings/privacy route renders PrivacySettingsPage directly with NO ProtectedRoute wrapper — anonymous hits 401 on GDPR fetches instead of login redirect; wrap route like siblings |
| `code-review-ppt-web-ui-aml-threshold-errors-swallowed` | pm-frontend | medium | compliance/useAmlDashboard.ts — three useAml* queries destructured without their error fields; threshold-breach alerts silently swallowed on failure; surface errors to AML page |
| `code-review-ppt-web-ui-forms-feature-i18n` | pm-frontend | medium | features/forms entire UI untranslated (FormBuilder, FormRenderer, FormCard, FormList, SubmissionList + four pages) — sweep i18n before shipping to sk/cs/de locales |
| `code-review-ppt-web-ui-portfolio-perf-i18n` | pm-frontend | medium | features/portfolio-performance entire UI untranslated (PortfolioListPage, PortfolioDashboardPage, PropertyForm, FinancialMetricsTable, BenchmarkComparisonCard, PortfolioCard) — swe |
| `code-review-ppt-web-ui-registry-page-silent-mutation-errors` | pm-frontend | medium | RegistryPage pet/vehicle delete+review mutations catch into console.error only — failures invisible to manager; already partially addressed by #3359, but handleReview paths still r |
| `code-review-reality-server-sso-cb-db-err-leak` | pm-security | medium | SSO callback upsert_sso_user() returns raw sqlx error Display in SsoError("user_creation_failed", ...) body — mask + tracing log |
| `pm-frontend-mobile-rn-lint-gates-adopt-2026-10-28` | pm-frontend | medium | Adopt eslint-plugin-react-hooks + i18next/no-literal-string in frontend/apps/mobile (blocking, warn-only for one sprint) — closes the chronic class-of-defect behind the carried mob |
| `pm-qa-ppt-web-i18n-golden-test-2026-11-09` | pm-qa | medium | Add a frontend golden test that fails on untranslated user-facing string literals in new feature folders (features/portfolio-performance, features/forms, features/registry) — stops |
| `pm-qa-raw-sqlx-error-leak-contract-test-2026-11-09` | pm-qa | medium | Add contract test on error-body shape that fails if any handler interpolates raw sqlx::Error Display (regex on response body) — curb the recurring raw-DB-error-leak findings (6 ope |
| `code-review-api-core-accounting-decimal-literal-parse` | pm-backend | low | refactor: replace Decimal::from_str("0.5"\|"0.8"\|"0.15"\|"0.05").unwrap() in run_payment_matcher with compile-time dec!/const — removes four prod .unwrap()s |
| `code-review-api-core-rng-expect-panic` | pm-backend | low | AuthService::generate_token() does rng.try_fill_bytes().expect("OS rng failed") — boot-time only normally, but on long-running process OS rng failure crashes auth; downgrade to err |
| `code-review-api-handlers-tenant-lifecycle-path-leak` | pm-backend | low | tenant restore/export endpoints return raw internal error strings (multipart/filesystem) in client body — exposes server filesystem layout; platform-admin-gated so low severity but |
| `code-review-mobile-native-kmp-http-client-no-timeout` | pm-backend | low | Shared Ktor HttpClient has no HttpTimeout - mobile API calls can hang indefinitely |
| `code-review-mobile-native-kmp-my-listings-pagination-cap` | pm-backend | low | Reality mobile MyListings + portfolio analytics silently cap at 100 listings (no pagination) |
| `code-review-mobile-native-kmp-sso-error-body-unguarded` | pm-backend | low | KMP realtor SSO error-body parsing is unguarded — crashes on non-JSON response |
| `code-review-ppt-web-core-errorhandler-i18n-leak` | pm-frontend | low | parseValidationDetails() stores raw English detail.message into per-field validationErrors; surfaced via getFieldError() + formatValidationError — key through i18n catalogue |
| `code-review-ppt-web-core-refresh-token-localstorage` | pm-security | low | SECURITY: ppt-web refresh-token persisted in localStorage — XSS-reachable |
| `code-review-ppt-web-ui-moderation-full-reload-nav` | pm-frontend | low | ContentModerationPage.handleViewContent uses window.location.href for internal SPA route — full reload; replace with useNavigate() |
| `code-review-ppt-web-ui-reading-form-ocr-silent-fail` | pm-frontend | low | meters/ReadingForm.tsx enableOcr path awaits OCR with no catch — rejection throws out of handler; wrap + user feedback |
| `code-review-ppt-web-ui-registry-page-hard-navigation` | pm-frontend | low | RegistryPage uses window.location.href for all pet/vehicle view/edit/create navigations — full document reload inside SPA; replace with useNavigate() |
| `code-review-ppt-web-ui-registry-rules-form-i18n` | pm-frontend | low | RegistryRulesForm.tsx (322 lines) ships NO useTranslation — manager form reachable from RegistryRulesPage; sweep hardcoded EN strings through i18n keys |
| `code-review-reality-server-pm-api-client-build-expect` | pm-backend | low | PmApiClient::new() reqwest::Client::builder().build().expect() — boot-time .expect that will panic the server on any unlikely TLS/Hyper build failure; return Result and bubble |
| `code-review-reality-server-readiness-internal-detail-public` | pm-backend | low | reality-server /readiness exposes git commit SHA + build timestamp + cache internals to anonymous callers — fingerprinting / CVE mapping surface; scrub public body |
| `code-review-reality-server-sso-cb-upstream-err-leak` | pm-security | low | SSO exchange_code_for_tokens error path returns raw e.to_string() in SsoError("token_exchange_failed", ...) — leaks upstream provider detail; sanitize |
| `code-review-reality-web-pricealerts-raw-err-log` | pm-frontend | low | PriceAlerts.tsx L98/L108 log the RAW rejection carrier via console.error — can print PII / auth header traces; swap to structured describeError() + toast |
| `code-review-reality-web-realtor-listing-i18n` | pm-frontend | low | reality-web realtor/listings/new page has no useTranslations; hardcoded EN UI in locale-aware app; sweep i18n |

## In-progress

| ID | Owner | Priority | Action |
|---|---|---|---|
| `gh-issue-3283` | pm-tech-lead | medium | Follow-up: realtor public profile page renders hardcoded Slovak UI on a 6-language portal (PR #3225) (Closes #3283) |
| `gh-issue-3304` | pm-tech-lead | medium | Follow-up: member-role escalation gate reimplements role parsing instead of reusing common::TenantRole::from_role_type (PR #3291) (Closes #3304) |
| `gh-issue-3305` | pm-tech-lead | medium | Follow-up: fold IoT org-scope check into the insert on the hot add_reading path instead of a separate SELECT EXISTS (PR #3289) (Closes #3305) |
| `gh-issue-3306` | pm-tech-lead | medium | Follow-up: consolidate per-file require_manager(rls) helpers onto the shared rls_authz::require_manager_rls (PR #3287) (Closes #3306) |
| `gh-issue-3307` | pm-tech-lead | medium | Follow-up: fault update/triage/status handlers manually guard org on an id-only query instead of using find_by_id_for_org (PR #3286) (Closes #3307) |
| `gh-issue-3308` | pm-tech-lead | medium | Follow-up: add RLS-aware repo variants for vote sub-resource reads instead of pool-bypass + manual vote_in_org guard (PR #3281) (Closes #3308) |
| `gh-issue-3309` | pm-tech-lead | medium | Follow-up: cs.json/sk.json were reordered (~9k-line churn) instead of appended; enforce stable key order for messages/*.json (PR #3260) (Closes #3309) |
| `gh-issue-3310` | pm-tech-lead | medium | Follow-up: community feed shows every author as "Neighbor" (API returns ids only); formatMoney coerces Decimal via Number() (PR #3269) (Closes #3310) |

## Done (this run)

- `code-review-reality-web-share-cmp-unchecked-json` — merged in PR #3350
- `code-review-ppt-web-ui-registry-no-i18n` — merged in PR #3351
- `code-review-ppt-web-ui-registry-page-i18n-gap` — merged in PR #3355
- `code-review-ppt-web-ui-registry-mutation-silent-fail` — merged in PR #3359

_Buffer: 47/36 open. Refill source: 2026-11-04..08 tier1d signals re-ingested + pm-qa + pm-scrum-master next_actions (post-#3357 planner refresh)._
