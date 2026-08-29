# pm-scrum-master — 2026-08-29

_Always-on synthesis role. Delivery lens across sprint + backlog._

## Summary

6 PRs merged in the 4-day window since last upkeep (2026-08-25 → 08-29) — all follow-up hardening; 5 of 6 concentrated in the AML/moderation slice, 1 was the chacha20 RUSTSEC lockfile bump that unblocked backend CI. Zero new dispatcher-spawned open PRs — dispatcher stack drained; but 3 stalled clusters have not moved (accounting trio #2555/#2558/#2559 = 30+ days idle, self-PR draft #2744 = 16 days idle, dependabot batch #2865-2867 = 2 days idle). The 84-1 + 84-2 partials remain unspawned for a 4th upkeep window despite topping the ranked backlog.

## Sprint progress

- **Sprint:** "Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth" · **epics_done = 3/5** unchanged.
- **Coverage:** 47/49 stories delivered (all 49 marked done in coverage.json; 2 carry `gaps[]` that are the 84-1/84-2 frontend slices). No status flips this window.
- **Auto-review loop:** last run closed 7/7 in-window; this window's 6 PRs are mostly next-level hardening on top of that closure — no NEW post-merge review issues opened yet (post-merge review runs later in this same routine cycle).

## Shipped since last run (6 PRs merged 2026-08-25..08-29)

- **#2874** — chacha20 RUSTSEC lockfile bump — unblocks Rust CI (dispatcher-critical infra fix)
- **#2868** — ContentModerationPage toast dedupe (ppt-web frontend)
- **#2869** — decide_appeal validation (aml_dsa backend)
- **#2870** — report_content reason bound (aml_dsa backend)
- **#2871** — moderation.rs response dedupe (aml_dsa backend, 400-line churn hotspot)
- **#2872** — auth_policy.rs email-verification seam (backend, 102-line churn)

## What's next (top actions)

1. **[high] Explicitly promote 84-1 direct-to-S3 upload wiring to an implementer window** — 4th consecutive upkeep with no dispatcher spawn; dispatcher blindness needs a manual push. — owner: pm-scrum-master.
2. **[high] Close RUSTSEC-2026-0258 (h2 DoS, gh-issue-2797)** — >11 days blocking every backend PR at cargo-deny; chacha20 fix this week only unblocked one advisory. — owner: pm-devops.
3. **[high] Break the 30-day log-jam on accounting trio #2555/#2558/#2559** — either land a reviewer decision or close-with-rationale; needs-human-judgement has not attracted action for a month; PRs will bit-rot against dev. — owner: pm-scrum-master + pm-tech-lead.
4. **[medium] Triage stalled draft #2744** (16 days idle, self-PR from operator with needs-human-review label). — owner: pm-scrum-master.
5. **[medium] Read moderation.rs for a structural extraction** — 4 of today's 6 PRs converged on the same file; same pattern that preceded voice_webhooks structural-defect flag. Extract before feature-6 lands. — owner: pm-tech-lead.

## Blockers

- **Standing:** RUSTSEC-2026-0258 (h2 DoS, gh-issue-2797) blocks every backend PR at cargo-deny — >11 days. Owner: pm-devops.
- **Aging:** 84-1 + 84-2 partial-story frontend slices unchanged for 4 upkeep windows — dispatcher ranking-but-not-spawning. Owner: pm-frontend / pm-scrum-master.
- **Aging:** Accounting trio #2555/#2558/#2559 = 30+ days idle, needs-human-judgement. Owner: pm-tech-lead.

## Risks (new this run)

- **[high/medium] Accounting trio 30d idle** — 2026-07-30 pm-scrum-master risk about reviewer starvation aged from 2 days to 30 days without a decision.
- **[medium/medium] AML/moderation hotspot emerging** — 5 of 6 PRs today touched aml_dsa surfaces; same repeat-churn pattern that preceded voice_webhooks flag.

## Decisions needed

- Formally revive 84-1 + 84-2 as an implementer-pair window vs continue waiting on dispatcher — owner: pm-frontend / pm-scrum-master (2026-08-25 decision NOT acted on).
- Close-vs-land decision for accounting trio — owner: pm-tech-lead.
