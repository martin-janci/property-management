# Project state — 2026-11-28

## Exec summary

Delivery sprint CLOSED 2026-11-21 with all 49 stories done across 13 epics. The
Hardening & Refactor sprint is in its second week: **36 PRs merged on `dev`**
in the last 24h (#3855–#3890), almost entirely dispatcher-driven security &
error-leak sanitation. No workstream counter moved: `modules-under-watch`
remains 0/6, `release-248-quarantine-burndown` 0/201 (deadline 2027-02-19),
`mobile-native-backlog` 0/4 (blocked on pm-devops cloud-runner).

## Active sprint

- **Name:** Hardening & Refactor — Modules Under Watch, release-248 Quarantine Burn-down, Mobile-native Backlog
- **Started:** 2026-11-21
- **Workstreams:**
  - `modules-under-watch` — 0/6 (in-progress; merged hardening PRs not mapped to modules)
  - `release-248-quarantine-burndown` — 0/201 (in-progress, due 2027-02-19 — needs ≈17/week from here to zero)
  - `mobile-native-backlog` — 0/4 (blocked on pm-devops cloud-runner, #2652/#2966)

## Shipped since last run (36 PRs)

- **Security authz/RLS hardening** — #3869 (deny-by-default when platform admin has no resolved role), #3868 (OCR classification history under tenant RLS), #3860 (`review_aml_assessment` cross-org 403/500 → 404), #3856 (reject backslash-obfuscated SSO open-redirect).
- **Error-leak sanitation sweep** (api-server + reality-server) — #3890, #3885, #3880, #3849, #3843, #3854.
- **Robustness** — #3879 bounded concurrent workflow executions via shared semaphore with regression test.
- **Dispatcher/process fixes** — #3861 (bounded Phase 5.6 non-mechanical-conflict escalation) and #3859 (deepened shallow clone in dispatcher Phase 1 preflight).
- **Remainder** — ~20 dispatcher-driven `code-review-*`, `test-gap-*`, `pm-*` implementation PRs (grouped).

## What's next (top 5)

1. **[high]** Map merged hardening PRs to `modules-under-watch` and `quarantine-burndown` counters, update `sprint-status.yaml` — owner `pm-tech-lead`.
2. **[high]** Review and merge dispatcher drafts #3891, #3892, #3893 (listings.rs db-error leak first) — owner `pm-security`.
3. **[high]** Provision/confirm the cloud-runner needed to unblock `mobile-native-backlog` (0/4) — owner `pm-devops`.
4. **[high]** Define grouped-notification retention/reset semantics for nil-id event-type groups (+ pre/post-#3863 cutoff for metrics) — owner `pm-backend` (from `pm-data` 2026-11-28).
5. **[high]** Add a machine-readable Hardening burn-down series from `.github/quarantine/registry.toml`, appended daily — owner `pm-devops` (from `pm-data` 2026-11-28).

## Blockers

- `mobile-native-backlog` (0/4) — waiting on `pm-devops` cloud-runner.
- PRs #2555 and #2559 (July 2026 ACC features) — stalled for months; need owner decision.
- Dependabot PRs #3443, #3444, #3446–#3449, #3634 — open and unmerged across multiple runs.

## Role focus today

- Rotation: **pm-data** (last-run 2026-11-17 → ran today 2026-11-28; next: pm-integration)
- Scrum Master: synthesized the above

## Per-role one-line

- **pm-data** — notification group-key fix (#3863) verified deterministic; flagged the **new edge** that entity-less event types now share one group per user (unbounded `notification_count` growth), and the hardening sprint has **no machine-readable burn-down series**. See `roles/pm-data.md`.
