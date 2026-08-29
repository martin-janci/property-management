# PPT Roadmap — upkeep 2026-08-29

Buffer: **14/36 open** · 6 PRs merged this window · project at 47/49 (2 partials aging 4 upkeep windows).

⚠ Buffer below half — the ranker has only 5 gap-source candidates and the queue is drained. Next lever is finishing 84-1/84-2 to close MVP delivery + closing the standing h2 DoS advisory to unblock backend CI.

## State of the project

- Stories: **47 done / 2 partial / 0 not-started** of 49 (13 epics). Unchanged since 2026-07-15 deep scan; auto-review loop keeps closing follow-ups faster than they arrive.
- Delta vs 2026-08-25 upkeep: **6 PRs merged** (5 of 6 in AML/moderation; 1 is chacha20 RUSTSEC lockfile bump unblocking backend CI). 2 issues closed today (#2873/#2875, both by PR #2874). Zero new dispatcher-spawned open PRs.
- Remaining gaps (the last 2 partial stories, both frontend slices on shipped APIs):
  1. **84-1** — ppt-web still uploads via server proxy; direct-to-S3 endpoint (#2309) has no frontend consumer.
  2. **84-2** — signer-facing document-sign page not built (screen-map planned, API complete).
- **Aging alert:** 84-1 + 84-2 unchanged for 4 upkeep windows (2026-07-30 / 08-06 / 08-25 / 08-29). Dispatcher ranks-them-highest-but-never-spawns → likely plan-file or claimable() predicate blindness. Escalate as a manual implementer window.
- Screen coverage: 0 orphan screens · 0 validation errors · 3 missing UC links (UC-33.1/33.2/33.3 — matcher artifact, not queued).
- Merged-PR keyword sweep: no coverage story flipped status — the 6 merged PRs were all follow-up hardening (AML dashboard + validation, auth-policy seam, response dedup, RUSTSEC lockfile).

## Ranked plan

### mvp / finish-what's-started (highest score, 8)

- [high] Wire ppt-web direct-to-S3 upload via POST /api/v1/documents/upload-url — api-client binding + UploadDocument integration + regression test (84-1 partial) — owner: pm-frontend
- [high] Build signer-facing document-sign page in ppt-web against shipped signing API; flip screen-map ppt/document-sign buildStatus planned→shipped; verify signature-request email delivery (84-2 partial) — owner: pm-frontend
- [high] Promote 84-1 + 84-2 as an explicit implementer-pair window — dispatcher blindness confirmed (4 upkeep windows without a spawn); manual shepherd required — owner: pm-scrum-master

### infra / CI unblock (score 8, security-boost)

- [high] Close RUSTSEC-2026-0258 (h2 empty-DATA-frame DoS, gh-issue-2797) — bump h2 workspace-wide; standing >11 days blocking every backend PR — owner: pm-devops
- [medium] Audit dispatcher plan-matcher for 84-1/84-2 blindness — either the plan files are missing under `.research/plans/` or claimable() is silently rejecting them — owner: pm-devops
- [medium] Add scheduled cargo-deny --lockfile-only on dev — chacha20 was surfaced only via a red PR check one run late — owner: pm-devops

### reviewer bandwidth / delivery unblocks (score 6-7)

- [high] Break the 30-day log-jam on accounting trio #2555/#2558/#2559 — reviewer-starvation risk aged 4× since 2026-07-30 without a decision — owner: pm-tech-lead + pm-scrum-master
- [medium] Triage stalled draft #2744 (16 days idle, self-PR, needs-human-review) — owner: pm-scrum-master
- [medium] Land or bulk-close dependabot batches #2865/#2866/#2867 — CI unblocked today, trigger auto-approve past the 2-min buffer — owner: pm-devops

### quality / observability (score 5-6)

- [medium] AML/moderation E2E CI job (report → moderate → appeal → decide) — 5 of 6 PRs today touched this surface; regressions should surface pre-merge — owner: pm-devops
- [medium] Emit quiet_hours_drain Prometheus counters (claim/deliver/skip) — makes the atomic-claim invariant from PR #2834 visible in prod — owner: pm-devops
- [medium] Draft moderation.rs structural extraction — same repeat-churn pattern that preceded voice_webhooks flag — owner: pm-tech-lead

### post-merge follow-ups (carried from 2026-08-25 window)

- [high] Add >1-replica concurrency integration test for quiet-hours drain atomic claim (#2834) — owner: pm-qa
- [high] Add authz regression test for direct-connect OTA credential writes (#2821) — owner: pm-qa
- [medium] Add regression test for VoteDetailScreen conditional-hooks fix (#2835) — owner: pm-qa
- [medium] Add fuzz/property test for CSV export sanitizer (#2827) — owner: pm-qa
- [medium] Add unit + integration test for voice OAuth token encryption round-trip after centralization (#2838) — owner: pm-qa
- [medium] Add ppt-web test asserting AML EDD/Review dialog state resets per assessment (#2833) — owner: pm-qa
- [low] Add visual-diff / snapshot test for VerificationBadge expiry copy across sk/cs/de/en (#2825) — owner: pm-qa

### post-merge follow-ups (carried from prior windows, score 5-6)

- [medium] Follow-up gh-issue-2794: voice device dedup DB-level (org,user,platform) uniqueness (PR #2793) — owner: pm-tech-lead
- [medium] Follow-up gh-issue-2816: overflow-hardening left a parallel orphaned `saved_searches` int4 path (PR #2815) — owner: pm-tech-lead
- [medium] Adopt eslint-plugin-react-hooks + no-hardcoded-strings ESLint config in frontend/apps/mobile — owner: pm-frontend

### bug / mobile-native-kmp reliability (score 3-4)

- [medium] mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — owner: pm-backend
- [low] mobile-native-kmp shared Ktor HttpClient installs no HttpTimeout — owner: pm-backend
- [low] getPortfolioAnalytics() truncates realtor portfolio at 100 listings — owner: pm-backend
- [low] getPortfolioAnalytics() unbounded fan-out (retry 1/2) — owner: pm-backend
- [low] Shared repositories swallow CancellationException — owner: pm-backend
- [low] SsoService has zero direct tests — owner: pm-qa

### reality-web + reality-server bug drift (score 2-3)

- [low] Share access log records proxy IP without XFF/CF-Connecting-IP unwind — owner: pm-backend
- [low] reality-web AgencyErrorState + ComparisonView have no i18n on 4-locale portal — owner: pm-backend
- [low] reality-server inquiry-detail hardcodes messages: [] — owner: pm-backend
- [low] reality-server saved-search alert loop swallows watermark-advance error — owner: pm-backend

### Screen-map drift (score 3)

- [medium] Link UC-33.1/33.2/33.3 to dispute screen-maps — matcher artifact carried since prior scan — owner: pm-frontend

Buffer: **14/36 open** · project at 47/49 · Next lever = manual implementer-pair window for 84-1/84-2 + h2 workspace-wide bump. The dispatcher stack is drained not because the backlog is empty, but because the ranker's top items are stuck in a plan-matcher blindspot.
