# PPT project state — 2026-08-26

## Executive summary

Sprint **Epic 6, 7A, 8A & 10A** (Announcements, Documents, Notifications, OAuth) reads 100% done at the story level, but the epic-level rollups in `sprint-status.yaml` (epic-6, epic-7a, epic-10b, epic-80) still show partial/in-progress with old `stories_completed` counts — a data-hygiene gap that misrepresents completion. This run's five merged PRs (#2848–#2852) are quality/hardening work layered on top of that finished sprint (AML dashboard extract, moderation overdue affordance, i18n snapshot regression, voice OAuth encryption round-trip suite, test dedup), not new sprint stories. Real open work sits in the 6 mobile-native/KMP action-list items (structurally cloud-unlandable) plus three human-authored accounting PRs stalled since 2026-07-30.

## Sprint progress

- Sprint: **Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth**
- Epics done (rollup, stale): **2 / 6**  · story-level detail: all tracked epics fully closed — reconciliation required.

## Shipped since last run

- **#2848** — churn-hotspot AML dashboard extract (frontend/apps/ppt-web compliance)
- **#2849** — moderation overdue affordance (ContentModerationPage)
- **#2850** — verification-badge i18n snapshot
- **#2851** — voice OAuth token encryption round-trip tests (voice_webhooks.rs)
- **#2852** — AML dashboard test dedup

## What's next (top 5)

1. **[high]** Reconcile epic-level status/`stories_completed` in `sprint-status.yaml` — owner: pm-scrum-master.
2. **[high]** Triage the 3 stalled accounting review PRs (#2555/#2558/#2559) — owner: human maintainer / pm-tech-lead.
3. **[high]** Get a human reviewer to merge or reject PR #2744 (dispatcher un-wedge, approved, 12 days stale) — owner: repo-owner / pm-devops.
4. **[high]** Stand up self-hosted / egress-allow-listed runner for mobile-native AGP builds (all KMP work blocked) — owner: pm-devops / platform.
5. **[medium]** Escalate 6 mobile-native/KMP action-list items to a local-toolchain landing path — owner: pm-backend (local toolchain).

## Blockers

- **PR #2555 / #2558 / #2559 (UC-ACC-05.17/05.9/05.8)** — accounting review queue, open since 2026-07-28, idle since 2026-07-30, outside dispatcher scope.
- **PR #2744 (dispatcher un-wedge, #1162/#2743)** — approved 12 days ago, `needs-human-review` label, still not merged; its own delay is blocking dispatcher throughput.
- **6 mobile-native/KMP action-list items** — cloud runner cannot resolve AGP/Google Maven (egress-403); 0 claimed across 3+ dispatcher runs.
- **Stale epic rollups in `sprint-status.yaml`** — epic-6/7a/10b/80 rollup fields contradict their own story lines.

## Role focus today

- **pm-scrum-master** (always-on): sprint reconciliation + stalled-PR + KMP-debt synthesis.
- **pm-devops** (rotating, last run 2026-06-16 → 71 days stale): mobile-native runner, PR #2744 escalation, stale-pr-guard, dependabot backlog.

## Per-role summaries

- **pm-scrum-master:** Sprint spine reads 100% done at story level; epic rollups stale. Real open work is KMP backlog + 3 stalled accounting PRs. Ask: define next sprint or continue on gap-driven backlog?
- **pm-devops:** Structural cloud-runner gap for mobile-native (AGP egress-403) is the top infra risk. PR #2744 stalled 12 days despite approval — `stale-pr-guard` is flag-only. Dependabot backlog (12+ PRs) needs periodic batch review.
