# PPT Roadmap — upkeep 2026-08-25

⚠ Buffer below half — consider running `/ppt-project-management scan` to refresh coverage (only 5 gap candidates remain in the ranked pool because 47/49 stories are already `done`; the queue is genuinely draining, not stale).

## State of the project

- Stories: **47 done / 2 partial / 0 not-started** of 49 (13 epics). Unchanged since 2026-07-15 deep scan; auto-review loop is closing follow-ups faster than they arrive.
- Delta vs 2026-08-06 upkeep: **13 PRs merged**, **7 previously-open action-list items resolved** (4 follow-up issues #2822/#2823/#2831/#2832, dispatcher meta #2743, plus 2 code-review items ppt-web-facilities-booking + aml-prompt-alert). Zero new open PRs — dispatcher stack is drained.
- Remaining gaps (the last 2 partial stories, both frontend slices on shipped APIs):
  1. **84-1** — ppt-web still uploads via server proxy; direct-to-S3 endpoint (#2309) has no frontend consumer.
  2. **84-2** — signer-facing document-sign page not built (screen-map planned, API complete).
- Screen coverage: 0 orphan screens · 0 validation errors · **1 missing UC link left** (UC-33.3 queued this run; UC-33.1/33.2 were queued earlier). Down from 3.
- Merged-PR keyword sweep: no coverage story flipped status — the 13 merged PRs were all follow-up hardening (CSV sanitizer, quiet-hours drain, AML dialog, mobile-rn hooks/i18n, voice OAuth encryption, facilities booking UX + i18n, verification-badge i18n, booking-connect authz).

## Ranked plan

### mvp / finish-what's-started (highest score, 8)

- [high] Wire ppt-web direct-to-S3 upload via POST /api/v1/documents/upload-url — api-client binding + UploadDocument integration + regression test (84-1 partial) — owner: pm-frontend
- [high] Build signer-facing document-sign page in ppt-web against shipped signing API; flip screen-map ppt/document-sign buildStatus planned→shipped; verify signature-request email delivery (84-2 partial) — owner: pm-frontend
- [high] Shepherd 84-1 + 84-2 to done as a paired implementer window — both frontend-only on shipped APIs; closes 49/49 delivery — owner: pm-frontend / pm-scrum-master

### post-merge follow-ups from this window (score 6-7, high/medium)

- [high] Add >1-replica concurrency integration test for quiet-hours drain atomic claim (#2834 / closed #2831) — assert at-most-once — owner: pm-qa
- [high] Add authz regression test for direct-connect OTA credential writes (#2821) — assert non-manager rejected — owner: pm-qa
- [medium] Add regression test for VoteDetailScreen conditional-hooks fix (#2835) — owner: pm-qa
- [medium] Add fuzz/property test for CSV export sanitizer (#2827 / closed #2822) — CR/LF/CRLF + formula-injection prefixes — owner: pm-qa
- [medium] Add unit + integration test for voice OAuth token encryption round-trip after centralization (#2838) — owner: pm-qa
- [medium] Add ppt-web test asserting AML EDD/Review dialog state resets per assessment (#2833 / closed #2832) — owner: pm-qa
- [low] Add visual-diff / snapshot test for VerificationBadge expiry copy across sk/cs/de/en (#2825) — owner: pm-qa

### security / carried (score 7)

- [high] cargo-deny advisories FAILED on dev: RUSTSEC-2026-0258 (h2 empty-DATA-frame DoS) — every backend PR blocked (#2797) — owner: pm-security

### quality / lint prevention (score 5-6)

- [medium] Adopt eslint-plugin-react-hooks + no-hardcoded-strings ESLint config in frontend/apps/mobile — 3 mobile-rn PRs this window (#2835/#2836/#2837) all fixed defects a lint would catch — owner: pm-frontend
- [medium] Draft a refactor plan for backend/servers/api-server/src/routes/voice_webhooks.rs — 3-hotspot-windows-running; PR #2838 chipped at token encryption but scheduler/token-refresh paths still cluster — owner: pm-tech-lead

### post-merge follow-ups (carried from prior windows, score 5-6)

- [medium] Follow-up gh-issue-2794: voice device dedup DB-level (org,user,platform) uniqueness (PR #2793) — owner: pm-tech-lead
- [medium] Follow-up gh-issue-2816: overflow-hardening left a parallel orphaned `saved_searches` int4 path (PR #2815) — owner: pm-tech-lead

### bug / mobile-native-kmp reliability (score 3-4)

- [medium] mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` (MissingFieldException on every /inquiries + /realtors/inquiries call) — owner: pm-backend
- [low] mobile-native-kmp shared Ktor HttpClient installs no HttpTimeout — every suspend API call can hang indefinitely — owner: pm-backend
- [low] getPortfolioAnalytics() truncates realtor portfolio at 100 listings — owner: pm-backend
- [low] getPortfolioAnalytics() unbounded fan-out (retry 1/2) — owner: pm-backend
- [low] Shared repositories swallow CancellationException in catch(e: Exception) — owner: pm-backend
- [low] SsoService has zero direct tests — owner: pm-qa

### reality-web + reality-server bug drift (score 2-3)

- [low] Share access log records proxy IP without XFF/CF-Connecting-IP unwind — owner: pm-backend
- [low] reality-web AgencyErrorState + ComparisonView have no i18n on 4-locale portal — owner: pm-backend
- [low] reality-server inquiry-detail hardcodes messages: [] — realtors never see persisted inquiry thread — owner: pm-backend
- [low] reality-server saved-search alert loop swallows watermark-advance error — re-enqueues duplicate alerts — owner: pm-backend
- [low] NFC credentials stored as one SecureStore value can exceed ~2KB cap — owner: pm-backend

### Screen-map drift (score 3)

- [medium] Link UC-33.3 to a dispute screen-map (last of 3 UC-33.x residual from coverage.screen_gaps) — owner: pm-frontend

Buffer: **17/36 open** · 9 in-progress · 7 items resolved this run · project at 47/49. Auto-review loop is doing its job: every issue closed this run resolved by a merged PR in the same window (#2827→#2822, #2826→#2823, #2833→#2832, #2834→#2831, #2828→facilities-silent-errors, #2829→aml-prompt-alert, dispatcher fix→#2743). Next lever is finishing 84-1/84-2 to close MVP delivery.
