# PPT Project State

_Generated: 2026-10-20 — routine Phase 1.6 upkeep (pm-tech-lead rotation slot, 3-month stale cadence) + pm-scrum-master always-on. Coverage `scan_kind=upkeep`; coverage_cursor idx 10 → 11 (epic-85 Build/CI configuration re-checked, both stories still done, no material change). 1-day lag since 2026-10-19 run — on schedule._

## Delta since last routine run (2026-10-19)

- **0 PRs merged** on `dev` since last run — the merge queue is stuck. Pre-approved dispatcher PRs cannot land because the dispatcher cannot self-approve its own PRs (new issue **#3131: Merge pipeline deadlocked** — 16 approved PRs stuck in `review` state).
- **6 open PRs touched** since last run: #3028 (compare-list TOCTOU, draft), #3119 (NewsScreen i18n fallback), #3125 (zip 9.x bump, RUSTSEC-2026-0367/0368/0369), #3127 (migration upload drops file), #3129 (AML list typing), #3130 (ammonia 4.2.4 bump, RUSTSEC-2026-0373).
- **3 new issues**:
  - **#3131 (follow-up, blocked)** — Merge pipeline deadlock: dispatcher bot's own PRs sit in `review` because it can't self-approve. 16 approved PRs blocked. **This is the top delivery blocker right now** — until resolved, the entire dispatcher-driven implementation flow is paused at the merge step.
  - **#3128 (follow-up, from-merged-review)** — type the AML assessments list endpoint. Fix PR #3129 already open.
  - **#3126 (bug, security, backend)** — cargo-deny RUSTSEC-2026-0373 (ammonia 4.1.7 unsound). Fix PR #3130 already open.
- **Buffer status**: Payload surfaced `buffer-low: claimable=6/72 — refill planner`. All open score-≥-2 code-review items are already captured in dispatcher assignments (`in-progress`/`review`/`quarantined`) — the backlog is correctly tracked, the bottleneck is **merge throughput**, not plan supply.

<sub>Prior upkeep note (archived): 2026-10-19 — pm-integration rotation; coverage_cursor 9 → 10 (epic-84 re-checked).</sub>

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** No status flips this run; `last_checked` refreshed on epic-85 (cursor re-check — negative, both build/CI env stories still done since 2026-07-15).
- **Zero PRs merged** this observation window — merge pipeline is deadlocked on **#3131** (dispatcher self-approval). This is now the **#1 operational risk** — all 16 dispatcher-authored tier-1 code-review PRs are sitting approved-but-unmerged.
- **Buffer not refillable from plans path.** The `buffer-low` payload is a symptom of the merge deadlock, not a missing-plan problem. Open score-≥-3 code-review items are already in the active assignment queue; new plans would just stack onto the same stuck pipeline.
- **Two active RUSTSEC blockers resolving in-flight**: #3125 (zip), #3130 (ammonia) — both pre-approved, both stuck behind #3131.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 3/5** at sprint level (sprint-status.yaml); coverage-level all extended epics are complete.

Shipped PRs this window: none.

## Role focus today: pm-tech-lead (+ pm-scrum-master always-on)

- **pm-tech-lead** (3-month stale rotation, last run 2026-07-27 — overdue). Three concrete follow-ups:
  1. **#3131 merge-queue deadlock** is a pm-tech-lead concern — the dispatcher architecture committed to a single bot identity for authorship + merging, and the GitHub protected-branch policy forbids self-approval. The fix shape is architectural: either (a) a second bot identity for merging, (b) merge-queue bypass policy for `auto-fix/*` + dispatcher PRs with CI as the gate, or (c) a human-approval queue batch sweep. Decision needed, not implementation. **Severity: critical** — pipeline cannot ship otherwise.
  2. **Churn-aligned static review of api-core services layer** — this run surfaced one net-new finding (`oauth-rng-expect-panic`) that mirrors the already-tracked `totp-rng-expect-panic`. The same pattern recurs in `auth.rs:97`, suggesting an uncaught cross-service convention. Codify a `TotpError`/`AuthError`-style result at the service boundary, remove `.expect("OS rng failed")` in all three services (totp.rs, auth.rs, oauth.rs). Follow-up: audit shared service crates for the same pattern.
  3. **Review-cursor saturation**: all 8 review-cursor segments were refreshed between 2026-10-17 and 2026-10-19; code-review tier is net new signal–starved except for repeat scans. Consider widening the segment map (split api-handlers by sub-domain) or lengthening the per-segment cadence. Low urgency.
- **pm-scrum-master**: Delivery is paused at the merge step. Nothing shipped. 1 critical blocker (#3131). 2 active RUSTSEC fix PRs in flight. The right message to the team is: fix the merge-queue deadlock before stacking more work.

## Coverage (upkeep this run — 2026-10-20)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated=2026-10-20T04:30:00Z`, no re-scan.
- **Epic re-check: epic-85 Build/CI configuration (cursor idx 10)** — both stories (Environment Variable Setup, Build Configuration by Environment) still `done`; no PR in the 2026-10-19..2026-10-20 window touched env-setup or build-config surfaces. `last_checked = 2026-10-20` stamped on both. Negative-check evidence appended.
- **No merged PRs this window** — no coverage status flips. `last_checked` refresh is cursor-only.
- **`coverage_cursor` advances 10 → 11** (epic-85 → epic-8a next run).
- **`pm_cursor` advances 0 → 1** (pm-tech-lead → pm-backend next; rotation continues forward). `role_last_run["pm-tech-lead"] = 2026-10-20`.
- **Composition unchanged: 49 done · 0 partial · 0 not-started** across 13 epics. Missing UC links: 3 (UC-33.1/33.2/33.3, already queued elsewhere).

## Top risks (unchanged from 2026-10-19 unless noted)

1. **R-merge-deadlock [NEW, critical]** — #3131 blocks every dispatcher PR from merging. Mitigation: pm-tech-lead decision (see above).
2. **R-rustsec-advisories [critical, in-flight]** — PR #3125 (zip 9.x) and PR #3130 (ammonia 4.2.4) are the fixes; both blocked by R-merge-deadlock.
3. **R-#2966-swagger-ui-egress [critical]** — unchanged; backend cloud builds still blocked.
4. **R-review-saturation [low]** — all review-cursor segments reviewed within 3 days; new findings slowing.

## Top next actions (today)

1. **Resolve #3131 merge-queue deadlock** — owner: pm-tech-lead (needs human adjudication on policy).
2. **Merge PR #3125 (zip 9.x RUSTSEC)** once #3131 unblocks — owner: pm-security.
3. **Merge PR #3130 (ammonia 4.2.4 RUSTSEC)** once #3131 unblocks — owner: pm-security.
4. **Audit auth.rs + oauth.rs for the same `OS rng failed` pattern** flagged in this run's code-review slice — owner: pm-tech-lead / api-core maintainer.
