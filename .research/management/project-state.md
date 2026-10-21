# PPT Project State

_Generated: 2026-10-21 — routine Phase 1.6 upkeep (pm-tech-lead rotation slot; pm-cursor idx 0 → 1) + pm-scrum-master always-on. Coverage `scan_kind=upkeep`; coverage_cursor idx 10 → 11 (epic-85 Infra & Config re-checked, both stories still `done`, no material change). 50h routine lag since 2026-10-19 — flagged `lag_warning`._

## Delta since last routine run (2026-10-19)

- **4 PRs merged**:
  - **#3125** — `rust_xlsxwriter` bump to clear `zip` RUSTSEC-2026-0367/0368/0369 (deny.toml + Cargo.{lock,toml}).
  - **#3127** — fix `/migration/import/upload`: silent dead-store dropped uploaded bytes (dead-store with underscore-prefixed variable, no storage write). +197/-3, test suites included.
  - **#3129** — type AML assessments list response (serde_json::Value → typed utoipa ToSchema structs). Follows up on #3122.
  - **#3130** — bump ammonia 4.1.7 → 4.2.4 (RUSTSEC-2026-0373). Cargo.lock-only.
- **3 new/updated issues:**
  - **#3134 (open, bug/security/backend)** — "cargo-deny advisories FAILED on dev: zip + ammonia" — residual transitive `zip 3.0.0` AFTER #3125 landed; **backend merge pipeline still blocked**.
  - **#3132 (open, follow-up)** — post-merge review of #3092 price_map cache: the 10k cap still allows cache-thrash DoS; eviction amplifies the flood.
  - **#3131 (open, blocked, 1d old)** — "Merge pipeline deadlocked: dispatcher bot cannot self-approve its own PRs (16 approved PRs stuck in review)". Dispatcher's auto-reviewer sets `reviewer_verdict=approve` internally but GitHub `reviewDecision` stays `null` because a bot can't self-approve its own PR. **Systemic deadlock.** Oldest stuck PR: #3024 (16 days).
- **3 Phase 1.5 code-review findings this run** on the `reality-server` segment (churn-aligned):
  - `code-review-reality-server-login-sso-account-oracle` (security, +2) — public /users/login enumerates SSO-provisioned emails via a differential error string.
  - `code-review-reality-server-update-agency-sqlx-leak` (security, +1) — PATCH /agencies leaks raw SqlxError text (constraint/table/column names) to clients.
  - `code-review-reality-server-realtor-respond-no-cap` (bug, +1) — the realtor-side respond-to-inquiry endpoint is missing the 5000-char cap enforced by the inquiries-side twin.
- **1 Tech-lead caveat on #3134:** on the checked-out `dev` tree `backend/deny.toml` already ignores RUSTSEC-2026-0367/0368/0369 for `zip 3.0.0` AND has `[bans]` on `zip 4.x–8.x`. So #3134 is **either a different / new advisory, or the CI deny.toml path is wrong, or it's a stale run** — a diagnostic triage is action #1 for pm-security before any fix lands.
- **pm-tech-lead rotation note:** pm-tech-lead was last run 2026-07-27 (86 days). Fresh architecture lens arrived exactly when the merge-gate stack is 16 days blocked — timely.

<sub>Prior upkeep note (archived): 2026-10-19 — pm-integration rotation; coverage_cursor 9 → 10 (epic-84 re-checked).</sub>

## Executive summary

- **Delivery: 49/49 stories done across 13 coverage epics.** No status flips this run; epic-85 `last_checked` refreshed on the rotation.
- **4 PRs merged** in the observation window: 2 security dep bumps, 1 silent-data-loss fix, 1 API typing refactor.
- **Buffer still constrained, triply so:** #2966 (swagger-ui egress) gates backend cloud builds; **#3134** (new cargo-deny trip) gates backend merges even on green builds; **#3131** (dispatcher self-approval deadlock) prevents the queue draining even when #3134 clears. #2652 continues to park mobile-native work. Expect no backend queue drain until the triplet (#3134 + #3131 + #2966) is addressed.
- **Three fresh backlog items** this run (all `reality-server`), one at `confidence=medium / vector=security / score=2` and two at score=1. The medium-security one (`login-sso-account-oracle`) is below the security fast-track confidence bar (medium, not high) — stays open at score 2 for now.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 2/6** at sprint level (sprint-status.yaml — **stale, epic counts don't match story statuses**); coverage-level all extended epics are complete.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | in-progress (3/6)† | 6/6 done | — |
| 7A — Documents & e-signature | in-progress (2/5)† | 5/5 done | — |
| 8A — Notifications | planned | — | — |
| 10A — OAuth Provider | planned | 7/7 done | — |

† sprint-status.yaml stale — reconcile is action #4 (pm-scrum-master).

## What's next (top 5)

1. **[high · pm-security]** Triage #3134 cargo-deny exact-advisory + crate, then adopt the written ignore-policy. _(pm-tech-lead action #1)_
2. **[high · pm-devops]** Resolve #3131 via scoped GitHub ruleset (`auto-impl` + green CI + dispatcher verdict = auto-merge). _(pm-tech-lead action #2 + pm-scrum-master action #2)_
3. **[high · human maintainer]** Drain the 16-PR dispatcher queue oldest-first (#3024, #3028, #3070, #3072, #3074, #3075, …) while the ruleset is scoped. _(pm-scrum-master action #2)_
4. **[medium · pm-backend]** Decide the PRICE_MAP_CACHE DoS protection strategy (#3132) — rate-limit before cache vs non-amplifying eviction vs drop cache. _(pm-tech-lead action #3)_
5. **[medium · pm-scrum-master]** Reconcile `sprint-status.yaml` (epic counts to story statuses) and record a sprint rollover.

## Blockers

- **#3134** — cargo-deny advisories (residual transitive zip 3.0.0) — blocks every backend PR merge · owner: pm-backend/pm-security.
- **#3131** — dispatcher PR deadlock, WIP=16/16, oldest stuck #3024 (16d) · owner: pm-devops.
- **PRs #3028 / #3070 / #3072 / #3074 / #3075** — approved internally, blocked on human merge + cargo-deny · owner: pm-security.
- **#2652** — mobile-native cloud builds, backlog items structurally unclaimable · owner: pm-devops.
- **#2966** — swagger-ui egress gates backend cloud builds · owner: pm-devops.

## Role focus today

- **pm-scrum-master** (always-on) — delivery synthesis.
- **pm-tech-lead** (rotating, idx 0 → 1) — merge-gate architecture, cargo-deny policy, dispatcher deadlock resolution, price_map cache strategy.
- Role summaries: see `.research/management/roles/pm-scrum-master.md`, `.research/management/roles/pm-tech-lead.md`.
