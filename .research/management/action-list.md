# PPT Action List

_Generated: 2026-08-25 · 33 items · 17 open · 9 in-progress · 7 done this run_

| Priority | Status | Owner | ID | Action | Dep | Source |
|---|---|---|---|---|---|---|
| high | open | pm-frontend | pm-scrum-master-shepherd-84-1-84-2-frontend-finish | Shepherd the 2 remaining partial MVP stories (84-1 direct-to-S3 wiring + 84-2 signer page) to done — both frontend-only... | none | pm-analysis 2026-08-25 |
| high | open | pm-qa | pm-qa-quiet-hours-drain-atomic-claim-concurrency-test | Add >1-replica concurrency integration test for quiet-hours drain atomic claim (#2834 / #2831) — assert at-most-once del... | none | pm-qa 2026-08-25 |
| high | open | pm-qa | pm-qa-booking-connect-non-manager-hijack-regression-test | Add authz regression test for direct-connect OTA credential writes (#2821) — assert non-manager role is rejected at the ... | none | pm-qa 2026-08-25 |
| medium | open | pm-frontend | pm-scrum-master-mobile-rn-lint-hooks-i18n-2026-08-25 | Adopt eslint-plugin-react-hooks + no-hardcoded-strings ESLint config in frontend/apps/mobile — 3 mobile-rn PRs this wind... | none | pm-analysis 2026-08-25 |
| medium | open | pm-tech-lead | pm-scrum-master-voice-webhooks-refactor-plan-2026-08-25 | Draft a refactor plan for backend/servers/api-server/src/routes/voice_webhooks.rs — 3 hotspot windows running | none | pm-analysis 2026-08-25 |
| medium | open | pm-frontend | gap-uc-33-3-link-dispute-screen-map | Link UC-33.3 to a dispute screen-map — add to docs/screens/ppt/disputes*.md use-cases frontmatter (last of 3 UC-33.x res... | none | gap-scan 2026-08-25 |
| medium | open | pm-qa | pm-qa-mobile-rn-vote-detail-conditional-hooks-regression-test | Add regression test for VoteDetailScreen conditional-hooks fix (#2835) | none | pm-qa 2026-08-25 |
| medium | open | pm-qa | pm-qa-csv-export-crlf-sanitizer-fuzz | Add fuzz/property test for CSV export sanitizer (#2827 / #2822) — CR, LF, CRLF, formula-injection prefixes | none | pm-qa 2026-08-25 |
| medium | open | pm-qa | pm-qa-voice-oauth-token-encryption-roundtrip-test | Add unit + integration test for voice OAuth token encryption round-trip after centralization (#2838) | none | pm-qa 2026-08-25 |
| medium | open | pm-qa | pm-qa-aml-dashboard-dialog-state-per-assessment-test | Add ppt-web test asserting AML EDD/Review dialog state resets per assessment (#2833 / #2832) | none | pm-qa 2026-08-25 |
| medium | open | pm-backend | code-review-mobile-native-kmp-inquiries-response-contract | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries + /realtors/inquiries call | none | dispatcher-backlog-refill 2026-08-16 (score=3) |
| low | open | pm-qa | pm-qa-verification-badge-expiry-i18n-visual-diff | Add visual-diff snapshot test for VerificationBadge expiry copy across sk/cs/de/en (#2825) | none | pm-qa 2026-08-25 |
| low | open | pm-backend | code-review-mobile-native-kmp-portfolio-analytics-caps-100 | getPortfolioAnalytics() truncates realtor portfolio at 100 listings | none | dispatcher-backlog-refill 2026-08-04 (score=2) |
| low | open | pm-backend | code-review-mobile-native-kmp-portfolio-analytics-unbounded-fanout-retry1 | getPortfolioAnalytics() fans out one analytics HTTP request per listing with no concurrency limit [retry 1/2] | none | dispatcher-retry-remint 2026-08-11 |
| low | open | pm-backend | code-review-mobile-native-kmp-cancellation-swallowed | Shared repositories swallow CancellationException in catch(e: Exception) | none | dispatcher-backlog-refill 2026-08-11 (score=2) |
| low | open | pm-qa | code-review-mobile-native-kmp-ssoservice-untested | SsoService (deep-link token exchange, login, password reset, session restore) has zero direct tests | none | dispatcher-backlog-refill 2026-08-11 (score=1) |
| low | open | pm-backend | code-review-mobile-native-kmp-httpclient-no-timeout | mobile-native-kmp shared Ktor HttpClient installs no HttpTimeout — every suspend API call can hang indefinitely | none | dispatcher-backlog-refill 2026-08-16 (score=2) |
| high | in-progress | pm-security | gh-issue-2797 | cargo-deny advisories FAILED on dev: RUSTSEC-2026-0258 (h2 empty-DATA-frame DoS) blocks every backend PR (Closes #2797) | none | dispatcher-issue-ingest 2026-08-18 (#2797) |
| medium | in-progress | pm-tech-lead | gh-issue-2794 | Follow-up: voice device dedup — enforce (org,user,platform) uniqueness at DB level (PR #2793) (Closes #2794) | none | dispatcher-issue-ingest 2026-08-18 (#2794) |
| medium | in-progress | pm-tech-lead | gh-issue-2816 | Follow-up: overflow-hardening left a parallel orphaned `saved_searches` path on int4 (PR #2815) (Closes #2816) | none | dispatcher-issue-ingest 2026-08-21 (#2816) |
| low | in-progress | pm-backend | code-review-api-handlers-share-log-proxy-ip | Share access log records proxy IP without CF-Connecting-IP / X-Forwarded-For unwind — misattributes source | none | dispatcher-backlog-refill 2026-08-17 (score=1) |
| low | in-progress | pm-backend | code-review-reality-web-agency-errorstate-i18n | reality-web AgencyErrorState has untranslated English messages on 4-locale portal | none | dispatcher-backlog-refill 2026-08-17 (score=1) |
| low | in-progress | pm-backend | code-review-reality-web-comparison-view-i18n | reality-web ComparisonView.tsx has no i18n — every string renders English on 4-locale portal | none | dispatcher-backlog-refill 2026-08-17 (score=1) |
| low | in-progress | pm-tech-lead | repeated-churn-backend/servers/api-server/src/routes/voice_webhooks.rs | repeated-churn: voice_webhooks.rs (runs_seen bumped to 2; PR #2838 chipped at token encryption but pattern persists) | none | dispatcher-backlog-refill 2026-08-17 (score=1) |
| low | in-progress | pm-backend | code-review-mobile-rn-nfc-securestore-2kb-blob | NFC credentials stored as one SecureStore value can exceed the ~2KB cap | none | dispatcher-backlog-refill 2026-08-17 (score=2) |
| low | in-progress | pm-backend | code-review-reality-server-inquiry-detail-empty-messages | reality-server inquiry-detail endpoint hardcodes messages: [] | none | dispatcher-backlog-refill 2026-08-20 (score=2) |
| low | in-progress | pm-backend | code-review-reality-server-saved-search-watermark-discard | Saved-search alert loop swallows watermark-advance error (let _ =), re-enqueuing duplicate alerts | none | dispatcher-backlog-refill 2026-08-20 (score=1) |
| high | done | pm-tech-lead | gh-issue-2743 | dispatcher: archives again exceed MCP push ceiling / retry-remint ghost retry (Closes #2743) — RESOLVED 2026-08-25 (issue closed upstream) | none | dispatcher-issue-ingest 2026-08-13 |
| medium | done | pm-tech-lead | gh-issue-2822 | CSV export sanitizer CR/LF (Closes #2822) — RESOLVED by PR #2827 | none | dispatcher-issue-ingest 2026-08-22 |
| medium | done | pm-tech-lead | gh-issue-2823 | Held-notification drain per-channel bookkeeping + retry cap (Closes #2823) — RESOLVED by PR #2826 | none | dispatcher-issue-ingest 2026-08-22 |
| medium | done | pm-tech-lead | gh-issue-2831 | Quiet-hours drain double-delivery under >1 replica (Closes #2831) — RESOLVED by PR #2834 (atomic claim) | none | dispatcher-issue-ingest 2026-08-22 |
| medium | done | pm-tech-lead | gh-issue-2832 | AML EDD/Review dialog stale state (Closes #2832) — RESOLVED by PR #2833 | none | dispatcher-issue-ingest 2026-08-22 |
| low | done | pm-backend | code-review-ppt-web-ui-facilities-booking-silent-errors | ppt-web facilities booking swallow errors — RESOLVED by PR #2828 | none | dispatcher-backlog-refill 2026-08-22 |
| low | done | pm-backend | code-review-ppt-web-ui-aml-dashboard-prompt-alert-flow | ppt-web AML dashboard prompt/alert → dialogs + i18n — RESOLVED by PR #2829 | none | dispatcher-backlog-refill 2026-08-22 |
