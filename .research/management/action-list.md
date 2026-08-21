# PPT Action List

_Generated: 2026-08-21 from `action-list.json`. Sorted by priority then age._

| Priority | Status | Owner | Action | Source |
|---|---|---|---|---|
| high | open | pm-tech-lead | dispatcher: two previously-fixed defects have recurred — archive push ceiling (#1162) + retry-remint ghost retry (#2460) (Closes #2743) | dispatcher-issue-ingest 2026-08-13 |
| high | in-progress | pm-security | cargo-deny advisories FAILED on dev: RUSTSEC-2026-0258 (h2 empty-DATA-frame DoS) blocks every backend PR (Closes #2797) — partially scoped by #2805 | dispatcher-issue-ingest 2026-08-18 |
| medium | in-progress | pm-tech-lead | voice device dedup — enforce (org,user,platform) uniqueness at DB level (PR #2793) (Closes #2794) | dispatcher-issue-ingest 2026-08-18 |
| medium | open | pm-backend | mobile-native-kmp: InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every /inquiries call | dispatcher-backlog-refill 2026-08-16 |
| medium | open | pm-qa | reality-server /inquiries/{id} detail: add regression test asserting persisted messages returned (PR #2813) | pm-qa 2026-08-21 |
| medium | open | pm-qa | voice-webhook HMAC empty-secret fail-closed: add explicit regression test (PR #2806 retry2) | pm-qa 2026-08-21 |
| low | open | pm-backend | mobile-native-kmp: getPortfolioAnalytics() truncates realtor portfolio at 100 listings | dispatcher-backlog-refill 2026-08-04 |
| low | open | pm-backend | mobile-native-kmp: getPortfolioAnalytics() unbounded fanout — up to 100 parallel GETs [retry 1/2] | dispatcher-retry-remint 2026-08-11 |
| low | open | pm-backend | mobile-native-kmp: shared repositories swallow CancellationException, breaking coroutine cancellation | dispatcher-backlog-refill 2026-08-11 |
| low | open | pm-qa | mobile-native-kmp: SsoService has zero direct tests (deep-link exchange, login, password reset, session restore) | dispatcher-backlog-refill 2026-08-11 |
| low | open | pm-backend | mobile-native-kmp: shared Ktor HttpClient installs no HttpTimeout — suspend calls can hang indefinitely | dispatcher-backlog-refill 2026-08-16 |
| low | in-progress | pm-backend | share access log records proxy IP without CF-Connecting-IP / X-Forwarded-For unwind | dispatcher-backlog-refill 2026-08-17 |
| low | in-progress | pm-backend | reality-web AgencyErrorState has untranslated English messages on 4-locale portal | dispatcher-backlog-refill 2026-08-17 |
| low | in-progress | pm-backend | reality-web ComparisonView.tsx has no i18n on 4-locale portal | dispatcher-backlog-refill 2026-08-17 |
| low | in-progress | pm-tech-lead | repeated-churn: backend/servers/api-server/src/routes/voice_webhooks.rs (runs_seen bumped to 2) | dispatcher-backlog-refill 2026-08-17 |
| low | in-progress | pm-backend | mobile RN: NFC credentials in one SecureStore value can exceed ~2KB cap and silently drop | dispatcher-backlog-refill 2026-08-17 |
| low | in-progress | pm-backend | reality-server inquiry-detail hardcodes messages: [] (fix landed via PR #2813 — verify closes) | dispatcher-backlog-refill 2026-08-20 |
| low | in-progress | pm-backend | reality-server saved-search alert loop swallows watermark-advance error (fix landed via PR #2812 — verify closes) | dispatcher-backlog-refill 2026-08-20 |
