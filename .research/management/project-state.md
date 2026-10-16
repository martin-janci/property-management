# PPT Project State

_Generated: 2026-10-16 — routine Phase 1.6 upkeep (pm-integration rotation slot; pm-cursor idx 7 → 0 wraps) + pm-scrum-master always-on. Coverage `scan_kind=upkeep`; coverage_cursor idx 9 → 10 (epic-84 S3/notifications/e-sig/RAG re-checked, 5/5 stories still done, no material change). ~3h lag — on schedule._

## Delta since last routine run (2026-10-16 06:51)

- **3 PRs merged** this window: #3103 (security — NAT64 RFC 8215 `/48` added to the SSRF filter, closes issue #3099), #3105 (mobile-rn — back-button labels moved into locale JSONs + regression test), #3106 (mobile-rn — Outages status badge translated + regression test). All three carry regression tests; two directly resolve 2026-10-15 backlog rows (`code-review-mobile-rn-detail-back-button-hardcoded-en` → done, `code-review-mobile-rn-outage-status-untranslated` → done).
- **1 new follow-up issue:** #3104 "Portfolio/owner analytics: comparisons + dashboard summary + owner portfolio runtime drift (500s)" — umbrella replacing the dangling `BIT-622` references; four analytics 500-paths named. Draft PR #3107 opened against it.
- **Phase 1.5 (reality-web) surfaced 2 Next.js SSR regressions** — `code-review-reality-web-realtor-mock-data` (+2 bug: `/realtor/[id]/page.tsx` serves `MOCK_AGENT` for every id) and `code-review-reality-web-agency-slovak-strings` (+1 refactor: `/agency/[slug]/page.tsx` hardcodes Slovak under `/[locale]/`). Both verified at file:line in this run.
- **Buffer still thin.** Dispatcher fire payload: `claimable=7/72 (open=14 dep_blocked=7)` — dominated by mobile-native-gated items (#2652) and score=1 noise. Phase 1.6 added 8 new action-list items to refill (6 new plus 2 duplicates of existing ids skipped).
- **One plan promoted (Phase 3):** `plans/code-review-api-handlers-violations-missing-role-authz.md` — SECURITY/bug (score=3, confidence=medium): the `violations.rs` router gates every mutation only on tenant-membership, letting any org member issue fines and approve their own appeal. Adversarial pass: fixed-in-place (test-file paths corrected to `tests/suites/violations_*.rs`).

<sub>Prior upkeep note (archived): 2026-10-15 → 2026-10-16 06:51 — pm-data rotation; coverage_cursor 8 → 9 (epic-83 Airbnb/Booking/portal re-checked).</sub>

## Executive summary

- **Delivery:** coverage.json still records 49/49 stories done across 13 epics; sprint-status.yaml remains stale on epic-6/7A/10B/80 — flagged for reconciliation.
- **This run:** 3 PRs merged, 1 security SSRF hardening + 2 mobile i18n fixes, all with tests. Zero coverage status flips.
- **Buffer-low alarm:** `claimable=7/72` (floor 36). Root cause unchanged — #2966 (utoipa-swagger-ui egress) blocks every api-server build in the cloud runner; #2652 blocks every KMP build. Phase 1.6 added +8 actions this run; buffer still structurally thin until the two infra unblocks land.
- **Analytics drift (new):** issue #3104 lists 4 runtime 500s between repositories and the migrated schema (property_comparisons.name, PortfolioPerformanceRepository::get_dashboard_summary, GET /owner-analytics/portfolio). Draft PR #3107 covers portfolio_benchmarks; the other three bullets need their own slices.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · sprint-status.yaml `epics_done = 2/6` at sprint level (stale — coverage shows all extended epics complete).

## Shipped since last run

- **PR #3103** gh-issue-3099: block NAT64 RFC 8215 local-use prefix in SSRF filter (security hardening with regression tests)
- **PR #3105** fix(mobile): externalize detail-screen back-button labels to i18n
- **PR #3106** fix(mobile): translate OutagesScreen status badge via i18n

## What's next (top 5)

1. **[high]** Unblock utoipa-swagger-ui crates.io egress for api-server in the cloud runner (#2966) — owner: `pm-devops`. Single largest buffer-starvation lever; gates all 3 IDOR retries.
2. **[high]** Retry cross-tenant IDOR fixes #2946, #2944, #2945 once #2966 is cleared — owner: `pm-devops` (hand-off to `pm-security`).
3. **[high]** Land draft PR #3107 (portfolio_benchmarks reconcile) and the other #3104 analytics-drift items; un-ignore the three success-path tests — owner: `pm-backend`.
4. **[high]** Make OpenAPI the source of truth for reality-server inquiries responses — align TypeSpec, utoipa, @ppt/reality-api-client and KMP openapi-generator on `limit` vs `page_size`; CI contract check in `api-validation.yml` — owner: `pm-backend`.
5. **[medium]** Audit outbound HTTP callers after #3103 NAT64 SSRF fix — confirm e-signature, webhook, S3 and embedding integrations all route through the filtered client — owner: `pm-security`.

## Blockers (unchanged)

- **#2966** utoipa-swagger-ui egress — gates every api-server backend PR in the cloud runner (`pm-devops`).
- **#2652** mobile-native/KMP cloud builds — 7 open backlog items `dep_blocked` on this (`pm-devops`).
- **PR #2559 (PAY by Square)** and **PR #2555 (invoice lifecycle)** — stalled ~75d waiting for a reviewer slot (`pm-tech-lead`).

Role focus today: **pm-scrum-master**, **pm-integration**.

## Role notes this run

- **pm-scrum-master:** 3 small PRs shipped (2× mobile i18n + 1× SSRF). Buffer-low is structural — the two infra unblocks (#2966, #2652) dominate. Four decisions queued for the human owner (sprint rename; swagger-ui vendor-vs-egress; #3104 priority; #2559/#2555 close-or-reassign).
- **pm-integration:** Analytics drift is the dominant integration risk — repository SQL has drifted from migrations in 4 places (umbrella #3104, draft PR #3107 covers one). KMP `InquiriesResponse` throws `MissingFieldException` on every real `/inquiries` + `/realtors/inquiries` call (required `page_size` vs server-sent `limit`). No single owner for paging envelope across api-server / reality-server / KMP.
