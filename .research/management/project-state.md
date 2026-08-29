# PPT Project State

_Generated: 2026-08-29 — routine Phase 1.6 lightweight upkeep (pm-devops rotation slot; 74-day stale slot refreshed) + pm-scrum-master always-on. Coverage `scan_kind=upkeep`; pm_cursor idx 4 → 5 (pm-devops → pm-security next), coverage_cursor idx 6 → 7 (epic-9 re-checked, no material change; advances to epic-7a). Sprint window 2026-08-25..08-29 shipped 6 PRs — the most-significant infra move was #2874 (chacha20 RUSTSEC lockfile bump) unblocking backend CI._

## Executive summary

- **Delivery still at 47/49 stories done, 2 partial** (84-1 direct-to-S3 upload wiring and 84-2 sign page — 4th consecutive upkeep window unchanged; the ranker keeps promoting them but the dispatcher never spawns an implementer — likely plan-file or claimable() predicate blindness). No status flips this window.
- **6 PRs merged** — the AML/moderation slice dominates (5 of 6):
  - #2874 chacha20 RUSTSEC lockfile bump — unblocks backend CI (dispatcher-critical infra)
  - #2868 ContentModerationPage toast dedupe (ppt-web frontend)
  - #2869 decide_appeal validation (aml_dsa backend)
  - #2870 report_content reason bound (aml_dsa backend)
  - #2871 moderation.rs response dedupe (backend, 400-line churn hotspot)
  - #2872 auth_policy.rs email-verification seam (backend)
- **Zero new dispatcher-spawned open PRs** — dispatcher stack drained; but 3 stalled clusters have not moved:
  - **Accounting trio #2555/#2558/#2559** — 30+ days idle, `needs-human-judgement`; the 2026-07-30 reviewer-starvation risk has aged 4× without a decision.
  - **Self-PR draft #2744** — 16 days idle, `needs-human-review`.
  - **Dependabot batches #2865/#2866/#2867** — 2 days idle; CI is now unblocked, so the auto-approve gate needs a manual trigger past its 2-min buffer.
- **Standing block: RUSTSEC-2026-0258 (h2 empty-DATA-frame DoS, gh-issue-2797)** — >11 days blocking every backend PR at cargo-deny. The chacha20 fix this week only unblocked one advisory; h2 still open and needs a workspace-wide bump.
- **Emerging hotspot: aml_dsa/moderation.rs** — 5 of 6 PRs today converged on the same surface; same repeat-churn pattern that preceded the voice_webhooks structural-defect flag. Worth a design read before feature-6 lands.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 3/5** unchanged this run. Extended-scope epics (10B, 79, 80, 81, 82, 83, 84, 85, 8A, 9) folded into `coverage.json` and largely done (47/49).

| Epic | Sprint status | Coverage status (13 epics) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6/6 stories done in coverage |
| 7A — Basic Document Management | in-progress | 5/5 stories done in coverage |
| 8A — Basic Notification Preferences | done | 3/3 stories done |
| 10A — OAuth Provider Foundation | done | 3/3 stories done |
| 10B — Platform Administration | in-progress | 7/7 stories done |
| 9 — TOTP 2FA | (extended) | 1/1 story done; **re-checked this run (idx 6), no material change, last_checked=2026-08-29** |
| 84 — Documents / e-signature | (extended) | 5/5 done in coverage but 84-1/84-2 carry frontend `gaps[]` unchanged for 4 windows |
| 79 / 80 / 81 / 82 / 83 / 85 / 8a / 10b / 7a / 10a | (extended) | all done in coverage |

## Shipped since last run (6 PRs merged 2026-08-25..08-29)

- **#2874** — chacha20 RUSTSEC lockfile bump (unblocks Rust CI after upstream yank of chacha20 0.9.1)
- **#2868** — ContentModerationPage toast dedupe (ppt-web frontend)
- **#2869** — aml_dsa/decide_appeal validation
- **#2870** — aml_dsa/report_content reason bound
- **#2871** — aml_dsa/moderation.rs response dedupe (400-line churn hotspot)
- **#2872** — services/auth_policy.rs email-verification seam

## What's next (top 5 actions from ranked backlog)

1. **[high] Explicitly promote 84-1 direct-to-S3 upload wiring to an implementer window** — 4th consecutive upkeep with no dispatcher spawn; ranker keeps scoring highest but no spawn — **owner: pm-scrum-master / pm-frontend**.
2. **[high] Close RUSTSEC-2026-0258 (h2 DoS, gh-issue-2797)** — >11 days blocking every backend PR at cargo-deny — **owner: pm-devops**.
3. **[high] Break the 30-day log-jam on accounting trio #2555/#2558/#2559** — reviewer-starvation risk aged 4× without a decision — **owner: pm-scrum-master + pm-tech-lead**.
4. **[medium] Land or bulk-close dependabot batches #2865/#2866/#2867** — CI is unblocked; trigger auto-approve past the 2-min buffer — **owner: pm-devops**.
5. **[medium] Read moderation.rs for a structural extraction** — 4 of today's 6 PRs converged on the same file; extract before feature-6 lands — **owner: pm-tech-lead**.

## Blockers

- **Standing:** RUSTSEC-2026-0258 (h2 empty-DATA-frame DoS, gh-issue-2797) blocks every backend PR at cargo-deny — >11 days. Owner: pm-devops.
- **Aging (4 upkeep windows):** 84-1 + 84-2 frontend slices — dispatcher ranks-but-never-spawns. Owner: pm-frontend / pm-scrum-master.
- **Aging (30+ days):** Accounting trio #2555/#2558/#2559. Owner: pm-tech-lead.
- **Aging (16 days):** Self-PR draft #2744 (needs-human-review). Owner: pm-scrum-master.

## Role focus today: **pm-devops** (rotation idx 4; last 2026-06-16, 74d stale) + pm-scrum-master always-on

- **pm-scrum-master** (always-on): produced the delivery synthesis above. Headline = the AML/moderation slice concentrated 5 of 6 PRs, and the 4-window aging on 84-1/84-2 is now a dispatcher-blindness signal, not just a slow ranker. The 30-day accounting trio is the loudest reviewer-capacity signal.
- **pm-devops** (rotation): flagged 6 infra actions — the RUSTSEC-2026-0258 h2 DoS is the critical block; scheduled cargo-deny on dev + AML/moderation E2E CI job + quiet_hours_drain Prometheus counters address prevention gaps; the dispatcher-blindness audit on 84-x is the highest-leverage delivery unblocker.

## Coverage (upkeep this run — 2026-08-29)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated` bumped to 2026-08-29T04:00:00Z, no re-scan.
- **Epic re-check: epic-9** — cursor idx 6. Only story `9-1-totp-2fa-setup` still `done`. No PR in the 2026-08-25..08-29 window touched TOTP/2FA routes/handlers/screens; evidence entry appended noting the negative check. `last_checked = 2026-08-29` stamped.
- **Merged-PR evidence:** 4 of 6 PRs touched aml_dsa surfaces already covered under existing stories; #2874 is infra (no story). No status flips.
- **`coverage_cursor` advances 6 → 7** (epic-9 → epic-7a next run).
- **`pm_cursor` advances 4 → 5** (pm-devops → pm-security next run). role_last_run["pm-devops"] = 2026-08-29.
- **Composition unchanged: 49 done (with 2 carrying frontend `gaps[]`)** across 13 epics. Missing UC links: 3 (UC-33.1/33.2/33.3 residual; matcher artifact, not queued). Zero orphan screens, zero validation errors.
