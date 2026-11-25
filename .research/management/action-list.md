# Action list — delivery backlog

<sub>Generated: 2026-11-25T04:15:10Z</sub>

- **Open:** 52
- **In-progress:** 7
- **Done:** 1
- **Failed:** 0

| ID | Priority | Owner | Action | Dep | Status | Deadline |
|----|----------|-------|--------|-----|--------|----------|
| `pm-scrum-master-land-the-2026-12-01-i18n-warn-error-lint-flip-burn-down-merg-2026-11-22` | high | `pm-scrum-master` | Land the 2026-12-01 i18n warn→error lint-flip burn-down: merge or close the 10+ open `code-review-*-i18n-retry1` items + drafts #3676/#3677 within 9 days | — | in-progress | — |
| `pm-devops-unblock-mobile-native-cloud-builds` | high | `pm-devops` | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimable in cloud; now compounded by #2966 so claimable=9/72 (buffer-low) | — | open | — |
| `pm-scrum-master-cloud-runner-kmp-swagger-ui-egress-unblock-2026-11-16` | high | `pm-scrum-master` | Unblock mobile-native/KMP and swagger-ui egress in the cloud runner (#2652, #2966). Claimable work is a small share of the backlog, and backend security PRs cannot compile. | — | open | — |
| `pm-scrum-master-promote-close-mobile-rn-i18n-drafts-2026-11-25` | high | `pm-scrum-master` | Promote or close mobile-RN i18n drafts #3676 and #3677 before the 2026-12-01 warn-to-error flip. | — | open | 2026-11-29 |
| `pm-scrum-master-unblock-kmp-and-swagger-ui-egress-2652-2966-2026-11-24` | high | `pm-devops` | Unblock mobile-native/KMP + swagger-ui egress in the cloud runner (#2652, #2966) — 4 KMP backlog items still structurally unclaimable. | — | open | — |
| `pm-scrum-master-unblock-kmp-and-swagger-ui-egress-in-the-cloud-runner-2652-2-2026-11-20` | high | `pm-scrum-master` | Unblock KMP and swagger-ui egress in the cloud runner (#2652, #2966) | — | open | — |
| `pm-scrum-master-unblock-kmp-swagger-ui-egress-2652-2966-2026-11-21` | high | `pm-scrum-master` | Unblock KMP + swagger-ui egress in the cloud runner (#2652, #2966) — claimable still ≈9/72 | — | open | — |
| `pm-scrum-master-unblock-mobile-native-kmp-swagger-ui-egress-in-the-cloud-run-2026-11-22` | high | `pm-scrum-master` | Unblock mobile-native/KMP + swagger-ui egress in the cloud runner (#2652, #2966) — 4 KMP backlog items still structurally unclaimable | — | open | — |
| `pm-scrum-master-verify-3357-after-3784-2026-11-25` | high | `pm-scrum-master` | Verify dispatcher stall #3357 status after #3784 and resolve remaining non-mechanical conflicts. | — | open | — |
| `pm-security-force-rls-person-month-submissions-2026-11-25` | high | `pm-security` | Add ALTER TABLE person_month_submissions (and person_months if not already) FORCE ROW LEVEL SECURITY; move submission repo methods (submit, find_submission, list_*_submissions, is_active_resident) onto the RLS connection; add cross-tenant integration test. | — | open | — |
| `code-review-ppt-web-core-logout-purge-disputes-leases-iot-retry1` | medium | `pm-security` | ppt web core logout purge disputes leases iot [retry 1/2 of failed code-review-ppt-web-core-logout-purge-disputes-leases-iot] | — | in-progress | — |
| `code-review-mobile-native-kmp-create-listing-not-wired` | medium | `pm-backend` | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded | — | open | — |
| `code-review-mobile-native-kmp-inquiries-response-contract` | medium | `pm-backend` | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries + /realtors/inquiries call | — | open | — |
| `pm-backend-address-mobile-native-stubs-2026-11-21` | medium | `pm-backend` | Address mobile-native stubs: CreateListingScreen onSubmit NotImplementedError + InquiriesResponse page_size/limit contract mismatch + Ktor HttpTimeout default. Package as one KMP PR per issue once #2652 egress unblocks | — | open | — |
| `pm-qa-confirm-non-kmp-medium-confidence-2026-11-09` | medium | `pm-qa` | QA audit: non-mobile-native backlog items with confidence=medium need a one-shot verification step before Phase 3 can promote them | — | open | — |
| `pm-scrum-master-quarantine-burndown-watch-decompose-2026-11-25` | medium | `pm-scrum-master` | Continue quarantine burn-down (201 rows, due 2027-02-19) using #3774 cadence; decompose watch modules (vote.rs, sso.rs, financial.rs, lease.rs). | — | open | 2027-02-19 |
| `pm-scrum-master-triage-open-prs-2026-11-25` | medium | `pm-scrum-master` | Triage open PRs: #3786 epic-gap-audit round 12, drafts #3787/#3788/#3789, stale #2555/#2559 (UC-ACC-05 wiring open since July). | — | open | — |
| `pm-scrum-master-uc-10-followup-watch-2026-11-25` | medium | `pm-scrum-master` | Land UC-10 follow-up tests/i18n/screen-map for PersonMonthsScreen.tsx (444) and person_months.rs (436) hotspots; consider adding them to the watch list. | — | open | — |
| `pm-security-audit-non-force-rls-pool-bypass-2026-11-25` | medium | `pm-security` | Repo-wide audit of the pool-instead-of-RLS-conn pattern and RLS tables that are not FORCE'd. Rank by sensitivity; file a backlog plan. | — | open | — |
| `pm-security-negative-authz-tests-person-months-2026-11-25` | medium | `pm-security` | Add negative authz tests to person_months_happy_path_tests.rs: non-resident submit 403, resident approve 403, cross-building submission_id 404, double-approve 409, out-of-range period 400. | — | open | — |
| `pm-security-restrict-non-manager-person-month-reads-2026-11-25` | medium | `pm-security` | Gate non-manager person-month read endpoints (GET unit/yearly/by-id, POST /calculate, GET building list/summary) on residency of that unit or manager role; add 403-cross-unit test. | — | open | — |
| `pm-security-submit-overwrite-semantics-2026-11-25` | medium | `pm-security` | Fix submit overwrite semantics on person_month_submissions: ON CONFLICT DO UPDATE lets a co-resident overwrite another resident's pending proposal and rewrites submitted_by. Decide: reject or audit trail; cover with test. | pm-product | open | — |
| `code-review-api-core-digest-worker-no-replica-claim` | low | `pm-backend` | api core digest worker no replica claim | — | in-progress | — |
| `code-review-api-handlers-summarize-discards-persist-err` | low | `pm-backend` | api handlers summarize discards persist err | — | in-progress | — |
| `code-review-api-handlers-tenant-config-rls-clear-swallowed` | low | `pm-backend` | api handlers tenant config rls clear swallowed | — | in-progress | — |
| `code-review-mobile-rn-list-search-placeholders-i18n` | low | `pm-backend` | mobile rn list search placeholders i18n | — | in-progress | — |
| `code-review-ppt-web-ui-bookfacility-silent-availability` | low | `pm-backend` | ppt web ui bookfacility silent availability | — | in-progress | — |
| `churn-hotspot-backend-servers-api-server-src-routes-voice-webhooks-handlers-rs` | low | `pm-tech-lead` | Churn hotspot: api-server voice_webhooks/handlers.rs — 265 LoC in PR #3760 (RLS release fix) | — | open | — |
| `churn-hotspot-backend-servers-api-server-src-services-push-apns-rs` | low | `pm-tech-lead` | Churn hotspot: services/push/apns.rs — 2 commits this run (push stack instability under watch) | — | open | — |
| `churn-hotspot-frontend-apps-ppt-web-src-features-government-portal-pages-guest-report-submission-page-tsx` | low | `pm-tech-lead` | Churn hotspot: ppt-web GuestReportSubmissionPage.tsx — 283 LoC in PR #3756 (i18n route-through) | — | open | — |
| `churn-hotspot-frontend-apps-ppt-web-src-i18n-buildings-i18n-test-ts` | low | `pm-tech-lead` | Churn hotspot: ppt-web buildingsI18n.test.ts — 379 LoC in PR #3745 (i18n regression lock) | — | open | — |
| `code-review-api-core-idempotency-tx-across-handler` | low | `pm-backend` | api core idempotency tx across handler | — | open | — |
| `code-review-api-core-require-auth-guest-default` | low | `pm-backend` | api core require auth guest default | — | open | — |
| `code-review-api-handlers-voice-oauth-link-failopen` | low | `pm-backend` | api handlers voice oauth link failopen | — | open | — |
| `code-review-api-handlers-webhook-ssrf-loopback-failopen` | low | `pm-backend` | api handlers webhook ssrf loopback failopen | — | open | — |
| `code-review-mobile-native-kmp-http-client-no-timeout` | low | `pm-backend` | Shared Ktor HttpClient has no HttpTimeout - mobile API calls can hang indefinitely | — | open | — |
| `code-review-mobile-native-kmp-my-listings-pagination-cap` | low | `pm-backend` | Reality mobile MyListings + portfolio analytics silently cap at 100 listings (no pagination) | — | open | — |
| `code-review-mobile-native-kmp-sso-error-body-unguarded` | low | `pm-backend` | KMP realtor SSO error-body parsing is unguarded — crashes on non-JSON response | — | open | — |
| `code-review-mobile-rn-auth-unvalidated-token-body` | low | `pm-backend` | mobile rn auth unvalidated token body | — | open | — |
| `code-review-mobile-rn-hardcoded-button-labels` | low | `pm-backend` | mobile rn hardcoded button labels | — | open | — |
| `code-review-mobile-rn-nfc-untested` | low | `pm-tech-lead` | mobile rn nfc untested | — | open | — |
| `code-review-mobile-rn-raw-status-untranslated` | low | `pm-backend` | mobile rn raw status untranslated | — | open | — |
| `code-review-mobile-rn-widget-apirequest-swallows-errors` | low | `pm-backend` | mobile rn widget apirequest swallows errors | — | open | — |
| `code-review-mobile-rn-widget-storage-unguarded-jsonparse` | low | `pm-backend` | mobile rn widget storage unguarded jsonparse | — | open | — |
| `code-review-ppt-web-core-auth-401-refresh-unwired` | low | `pm-backend` | ppt web core auth 401 refresh unwired | — | open | — |
| `code-review-ppt-web-ui-appshell-usermenu-arrow-keys` | low | `pm-backend` | ppt-web AppShell UserMenu declares role="menu" but missing arrow-key navigation + focus return — SR users cannot operate | — | open | — |
| `code-review-ppt-web-ui-automationrules-mutation-no-catch` | low | `pm-backend` | ppt web ui automationrules mutation no catch | — | open | — |
| `code-review-ppt-web-ui-conditionbuilder-number-coerce-zero` | low | `pm-backend` | ppt web ui conditionbuilder number coerce zero | — | open | — |
| `code-review-ppt-web-ui-confirm-dialog-static-ids` | low | `pm-backend` | ppt-web ConfirmationDialog hardcodes DOM ids — two concurrent instances break aria-labelledby/-describedby pairing | — | open | — |
| `code-review-ppt-web-ui-toast-role-alert-mismatch` | low | `pm-backend` | ppt-web Toast.tsx role="alert" is unconditional — success/info toasts interrupt SR reading | — | open | — |
| `code-review-reality-web-comparison-unvalidated-cast` | low | `pm-backend` | reality web comparison unvalidated cast | — | open | — |
| `code-review-reality-web-layoutsections-dup-key` | low | `pm-backend` | reality web layoutsections dup key | — | open | — |
| `code-review-reality-web-maint-fallback-i18n` | low | `pm-backend` | reality web maint fallback i18n | — | open | — |
| `code-review-reality-web-realtor-account-i18n-gap` | low | `pm-tech-lead` | reality web realtor account i18n gap | — | open | — |
| `code-review-reality-web-realtor-profile-mock-noid` | low | `pm-backend` | reality web realtor profile mock noid | — | open | — |
| `code-review-reality-web-realtormgmt-i18n-gap` | low | `pm-backend` | reality web realtormgmt i18n gap | — | open | — |
| `test-gap-pr-3762-payment-match-no-test-diff` | low | `pm-qa` | PR #3762 (payment-match date tiebreak + admin-web capability gates) added 70 LoC to accounting.rs with no test diff | — | open | — |
| `test-gap-screen-map-drift-reality-journal-pr-3764` | low | `pm-qa` | screen-map drift: PR #3764 changed reality-web /journal route without touching docs/screens/reality/journal.md | — | open | — |
| `test-gap-screen-map-drift-reality-realtor-analytics-pr-3740` | low | `pm-qa` | screen-map drift: PR #3740 changed reality-web /realtor/analytics without touching docs/screens/reality/realtor-analytics.md | — | open | — |
| `pm-scrum-master-dependabot-3441-axum-test-2026-11-24` | low | `pm-scrum-master` | Dependabot #3441 (axum-test 21.0.0 → 21.1.0) open since 2026-11-23 — trivial patch bump, land next cycle. | — | done | — |
