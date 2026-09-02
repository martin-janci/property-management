# PPT Roadmap — upkeep 2026-09-02

_Static read; no re-scan. Coverage `scan_kind=upkeep`; pm-security rotation + pm-scrum-master always-on. This window shipped 5 PRs — 2 in-window follow-up closures, 3 hygiene/refactor. 47/49 stories done, 2 partial (84-1/84-2) — unchanged for 5th consecutive upkeep window._

## State of the project

- Stories: **47 done / 2 partial / 0 not-started** of 49 (13 epics). Composition unchanged since 2026-07-15 deep scan.
- Delta vs 2026-08-31 upkeep: **5 PRs merged**, **2 follow-up issues opened & closed same-window** (#2924→#2928, #2923→#2927). Zero action-list items resolved by merged PRs this run (all 5 PRs were hygiene/follow-up on already-shipped surfaces, not backlog items).
- Remaining gaps (the last 2 partial stories, both frontend slices on shipped APIs, unchanged for 5 windows):
  1. **84-1** — ppt-web still uploads via server proxy; direct-to-S3 endpoint (#2309) has no frontend consumer.
  2. **84-2** — signer-facing document-sign page not built (screen-map planned, API complete).
- Screen coverage: 0 orphan screens · 0 validation errors · **3 missing UC links** (UC-33.1/33.2/33.3 all queued).
- Merged-PR keyword sweep: no coverage story flipped status — the 5 merged PRs were all correctness / hygiene on already-shipped surfaces (compliance db_error routing, FCM legacy path removal, saved-search typed error enum, report_summary snapshot, EmergencyContact test).
- Auto-review loop confirmed healthy: 2/5 PRs closed same-window follow-up issues.

## Top 3 biggest gaps (state.roadmap)

1. **84-1 direct-to-S3 wiring in ppt-web** — 5-window aging; only real MVP-blocking work left.
2. **84-2 signer-facing document-sign page** — same aging, paired with #1.
3. **RUSTSEC-2026-0258 h2 empty-DATA-frame DoS (gh-issue-2797)** — 15+ days standing; blocks every backend PR through cargo-deny.

## Ranked plan

### mvp / finish-what's-started (highest score, 8)

- [high] Wire ppt-web direct-to-S3 upload via POST /api/v1/documents/upload-url — api-client binding + UploadDocument integration + regression test (84-1 partial) — owner: pm-frontend
- [high] Build signer-facing document-sign page in ppt-web against shipped signing API; flip screen-map ppt/document-sign buildStatus planned→shipped; verify signature-request email delivery (84-2 partial) — owner: pm-frontend
- [high] Direct-promote 84-1 + 84-2 as a paired implementer window — 5 windows without dispatcher pick-up; both frontend-only on shipped APIs; closes 49/49 delivery — owner: pm-frontend / pm-scrum-master

### security / carried (score 7-9)

- [high] cargo-deny advisories FAILED on dev: RUSTSEC-2026-0258 (h2 empty-DATA-frame DoS) — every backend PR blocked (#2797) — owner: pm-security
- [high] Grep-sweep for raw-sqlx-error leak class fixed in PR #2925 across backend/servers/api-server/src/routes/** — every secondary DB call that stringifies sqlx into a 500 body is a candidate; land top 3 sites in one code-review PR — owner: pm-backend
- [high] Regression test for compliance audit-log DB-error path (#2925): inject sqlx failure, assert 500 body contains NO connection-string or SQL fragments — owner: pm-qa
- [medium] Close carried #2485 (layout-webhook replay guard) and #2486 (mobile LAYOUT_CACHE_KEY cross-tenant) — both open 6+ weeks with no PR movement — owner: pm-security
- [medium] Log-scrubbing policy for `user_id = %user_id` in push_fanout.rs (10+ sites at info/warn) — owner: pm-devops (with pm-security concurrence)

### infra (score 7)

- [high] Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — 7/8 open backlog items structurally unclaimable — owner: pm-devops

### post-merge follow-ups this window (score 5-6)

- [medium] Extend reality-server typed error enum pattern from #2922 (saved_searches) to inquiries.rs and reports.rs — 1 more route this sprint — owner: pm-backend

### post-merge follow-ups (carried from prior windows, score 4-6)

- [high] Add >1-replica concurrency integration test for quiet-hours drain atomic claim (#2834 / closed #2831) — assert at-most-once — owner: pm-qa
- [high] Add authz regression test for direct-connect OTA credential writes (#2821) — assert non-manager rejected — owner: pm-qa
- [medium] Add regression test for VoteDetailScreen conditional-hooks fix (#2835) — owner: pm-qa
- [medium] Add fuzz/property test for CSV export sanitizer (#2827 / closed #2822) — CR/LF/CRLF + formula-injection prefixes — owner: pm-qa
- [medium] Add unit + integration test for voice OAuth token encryption round-trip after centralization (#2838) — owner: pm-qa
- [medium] Add ppt-web test asserting AML EDD/Review dialog state resets per assessment (#2833 / closed #2832) — owner: pm-qa
- [medium] Adopt eslint-plugin-react-hooks + no-hardcoded-strings ESLint config in frontend/apps/mobile — owner: pm-frontend
- [medium] Draft refactor plan for backend/servers/api-server/src/routes/voice_webhooks.rs (3-hotspot-windows-running) — owner: pm-tech-lead
- [medium] Follow-up gh-issue-2794: voice device dedup DB-level uniqueness (PR #2793) — owner: pm-tech-lead
- [medium] Follow-up gh-issue-2816: overflow-hardening left parallel orphaned `saved_searches` int4 path (PR #2815) — owner: pm-tech-lead
- [low] Add visual-diff / snapshot test for VerificationBadge expiry copy across sk/cs/de/en (#2825) — owner: pm-qa

### bug / mobile-native-kmp reliability (score 3-4)

- [medium] mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` (MissingFieldException on every /inquiries + /realtors/inquiries call) — owner: pm-backend
- [low] mobile-native-kmp shared Ktor HttpClient installs no HttpTimeout — every suspend API call can hang indefinitely — owner: pm-backend
- [low] getPortfolioAnalytics() truncates realtor portfolio at 100 listings — owner: pm-backend
- [low] getPortfolioAnalytics() unbounded fan-out (retry 1/2) — owner: pm-backend
- [low] Shared repositories swallow CancellationException in catch(e: Exception) — owner: pm-backend
- [low] KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — owner: pm-backend
- [low] SsoService has zero direct tests (dispatcher-visible availability bug per pm-security note on #2574 open_question) — owner: pm-qa

### reality-web + reality-server bug drift (score 2-3)

- [low] reality-server agent review endpoint allows self-review (missing subject != reviewer gate) — owner: pm-backend
- [low] Share access log records proxy IP without XFF/CF-Connecting-IP unwind — owner: pm-backend
- [low] reality-web AgencyErrorState + ComparisonView have no i18n on 4-locale portal — owner: pm-backend
- [low] reality-server inquiry-detail hardcodes messages: [] — realtors never see persisted inquiry thread — owner: pm-backend
- [low] reality-server saved-search alert loop swallows watermark-advance error — owner: pm-backend
- [low] NFC credentials stored as one SecureStore value can exceed ~2KB cap — owner: pm-backend

### Screen-map drift (score 3)

- [medium] Link UC-33.1/33.2/33.3 to dispute screen-maps (residual from coverage.screen_gaps) — owner: pm-frontend
- [low] screen-map-drift: PR #2894 touched reality-web routes without updating docs/screens — owner: pm-qa

Buffer: **17/36 open** · project at 47/49. Auto-review loop is doing its job (2 in-window closures this run). Next lever remains the same 3 items: finish 84-1/84-2 (owner: pm-frontend) + land h2 bump (owner: pm-security). ⚠ Buffer below half — genuine drain (project is nearly delivered), not stale coverage.
