# PPT Project State

_Generated: 2026-08-21 — routine Phase 1.6 lightweight upkeep (pm-qa rotation slot). Coverage `scan_kind=upkeep`; pm_cursor idx 3 → 4 (pm-qa → next role), coverage_cursor idx 5 → 6 (epic-79 re-checked, no material change). Sprint window 2026-08-19..08-21 shipped 10 PRs — almost all reality-server hardening from post-merge review findings; dispatcher stack cleanly cycling review→approve→merge._

## Executive summary

- **Delivery still at 47/49 stories done, 2 partial** (84-1 direct-to-S3 wiring and 84-2 sign page unchanged). The 2026-08-19→08-21 window shipped **10 PRs** — 7 reality-server / api-server hygiene fixes from post-merge review, 2 CI/security posture PRs (#2803 apt-timeouts, #2805 h2 advisory scope), and 1 voice-device consolidation (#2811).
- **Reality-server hardening batch:** #2799 (cap create_review body), #2800 (rate-limit anonymous POST /reports per IP), #2801 (list_my_reports true row count), #2812 (saved-search watermark-advance error propagation), #2813 (inquiry-detail persisted messages), #2815 (portal_saved_searches.match_count → bigint). All 6 close code-review findings from the dispatcher's reality-server surface sweep.
- **Security / infra:** #2805 scopes RUSTSEC-2026-0258 (h2 empty-DATA-frame DoS) ignore to h2 0.3.x line so it stops blocking every backend PR (partial mitigation of gh-issue-2797, which stays open until h2 0.3.x is dropped). #2806 makes voice-webhook HMAC fail closed on empty secret (retry2 of an earlier failed attempt).
- **Voice-device consolidation:** #2811 routes ai/voice link path through the single-writer voice-device upsert (closes gh-issue-2807), continuing the dedup thread whose DB uniqueness enforcement is still open as gh-issue-2794.
- **QA metric on this batch:** 7 / 10 PRs shipped with a touched test file; 3 shipped without (#2803 CI-only, #2805 config, and two hot spots: **#2813 inquiry-detail messages** and **#2806 voice-webhook empty-secret fail-closed** — both behavior changes worth explicit regression coverage). Two QA-owned regression-test actions added this run.
- **Open PRs:** 1 active dispatcher assignment (PR #2744, gh-issue-2743 archive/retry-remint fixes, verdict=approve but formal APPROVE blocked on self-PR; awaiting human merge).

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 3/5** unchanged.

| Epic | Sprint status | Coverage status (13 epics) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6/6 stories done |
| 7A — Basic Document Management | in-progress | 5/5 stories done |
| 8A — Basic Notification Preferences | done | 3/3 done |
| 10A — OAuth Provider Foundation | done | 3/3 done |
| 10B — Platform Administration | in-progress | 7/7 done |
| 80 — Dispute Resolution | partial | 3/3 done in coverage (sprint reconcile pending) |
| 84 — Documents / e-signature | (extended) | 3/5 done, 2 partial (84-1, 84-2) |
| 82 / 83 / 85 / 79 / 81 / 8a / 9 | (extended) | all done in coverage; **79 re-checked this run — all 4 stories still done, no ppt-web auth changes in window** |

## Shipped since last run (10 PRs > #2798)

- **#2815** — fix(reality-server): widen portal_saved_searches.match_count to bigint
- **#2813** — code-review reality-server: inquiry-detail returns persisted messages (was hardcoded `[]`)
- **#2812** — fix(reality-server): stop discarding saved-search watermark-advance error
- **#2811** — gh-issue-2807: route ai/voice link path through single-writer voice-device upsert
- **#2806** — code-review api-handlers: voice webhook HMAC fails closed on empty secret (retry2)
- **#2805** — fix(api-server): scope RUSTSEC-2026-0258 h2 DoS ignore to h2 0.3.x line
- **#2803** — gh-issue-2802: bound apt-get network waits in CI to unwedge test-shard
- **#2801** — code-review reality-server: list_my_reports returns true total row count
- **#2800** — code-review reality-server: rate-limit anonymous POST /api/v1/reports per IP
- **#2799** — code-review reality-server: cap create_review body length

## What's next (top 3 actions from ranked backlog)

1. **[high] Land gh-issue-2797** — replace / retire h2 0.3.x so RUSTSEC-2026-0258 stops being a chronic advisory carrier — **owner: pm-security**. #2805 only scoped the ignore.
2. **[medium] Add QA regression tests** for #2813 (inquiry-detail persisted messages) and #2806 (voice-webhook empty-secret fail-closed) — both are silent behavior changes with no dedicated test — **owner: pm-qa**.
3. **[medium] Merge PR #2744** (gh-issue-2743 archive-routing + retry-remint guard) — reviewer-approved, self-PR blocks formal APPROVE, awaits human — **owner: pm-tech-lead**.

## Blockers

- **PR #2744 stuck on self-PR approval** — reviewer verdict=approve posted as comment; formal APPROVE blocked. Needs human merge or approval-policy exception.
- **h2 0.3.x still on the tree** — RUSTSEC-2026-0258 is scoped-ignored (#2805) but not fixed at root (gh-issue-2797 still open).

## Role focus today: **pm-qa** (rotation idx 3; last 2026-06-15, 67d stale) + pm-scrum-master always-on

- **pm-scrum-master** (always-on): synthesis above. Headline = the dispatcher is now cleanly cycling small, well-scoped reality-server fixes from post-merge review through review→approve→merge; average PR size is small (<5 files, 1-2 file test companion). No epic-scale motion this window.
- **pm-qa** (rotation): Test-coverage discipline held at ~70% (7 / 10 PRs shipped with a touched test file). Two behavior-changing patches (#2813, #2806) landed without dedicated tests — added as `qa-regression-test-*` actions. No drift in the RLS / cross-tenant test surface; the reality-server hardening batch stayed within its guarded test perimeter. Recommend a follow-up sweep on all *code-review-…-retry2* fixes to confirm each retry has its own regression assertion.

## Coverage (upkeep this run — 2026-08-21)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated` bumped, no re-scan.
- **Epic re-check: epic-79** — cursor idx 5. All 4 stories still `done`; upkeep note appended to each evidence[] confirming no ppt-web auth / error / WS layer changes in PRs #2799-#2815. `last_checked = 2026-08-21` stamped.
- **No status flips.**
- **Composition unchanged: 47 done · 2 partial · 0 not-started** across 13 epics. Same missing UC links (UC-33.1/33.2/33.3). Zero orphan screens, zero validation errors.
