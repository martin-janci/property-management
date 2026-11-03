# PPT Project State

_Generated: 2026-11-03 — routine Phase 1.6 incremental upkeep. Role focus today: **pm-qa** (70-day stale rotation slot; pm-cursor idx 3 → 4). Coverage `scan_kind=upkeep`; coverage_cursor idx 0 → 1 (epic-10a OAuth re-checked, 3 stories still done, no PRs touched OAuth)._ 

## Delta since last routine run (2026-11-02)

- **5 PRs merged** (#3216–#3220) — all backlog-sourced follow-ups that landed in the same ~12:36 UTC window:
  - **#3216** screen-map-drift reconcile (reality account-listing-edit docs).
  - **#3217** repeated-churn edit/page.tsx — _number helper + tests extracted.
  - **#3218** code-review EvidenceUploader i18n (sk/cs/de/en/hu/pl).
  - **#3219** churn-hotspot price_map LIKE metacharacter escape (user-controlled `city` was interpolated into LIKE pattern — small SQL-injection-adjacent fix).
  - **#3220** code-review community-feature i18n backfill (17 files; dominates this run's hotspot picture).
- **2 new draft PRs** (#3221, #3222) opened by the dispatcher this morning; both are verdict=approve in assignments but stuck in `review` state (merge wedge).
- **No new issues since #3215.** Issue #3215 (dispatcher merge pipeline wedge) remains the acting blocker.
- **Dispatcher state:** buffer 11/13 claimable, dep-blocked=2 (continues above 1/3 threshold but trending down because nothing is draining via merge).

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** No status flips this run; `last_checked` refreshed on epic-10a (idx 0, OAuth 10a-1/10a-2/10a-3 all still done, no PRs touched OAuth surfaces in the window).
- **5 PRs merged** — all backlog-driven follow-ups. Good signal that the dispatcher implementation path works end-to-end.
- **pm-qa rotation came up 70 days stale** — last run 2026-08-25. The window surfaced 3 new api-handlers error-swallowing patterns (unwrap_or_default on Result in list_templates / get_listing photos / suspend_organization cascade). One new pm-qa next_action queued (lint sweep); one new risk (`pm-qa-unwrap-or-default-sprawl-2026-11-03`, high/medium) plus one meta-risk on the pipeline wedge normalising (`pm-scrum-master-pipeline-wedge-normalisation-2026-11-03`, high/high).
- **No new blocker surfaced on dev CI** — all workflows green on HEAD (sha 19cedd8e). The acting blocker remains **#3215 — merge pipeline wedge** (8 assignments in `review` state with verdict=approve, all waiting on a GitHub APPROVED review the bot cannot grant to itself).

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 4/4** at sprint level (reconciled 2026-10-26). Coverage holds 13 epics all at done.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | done (6/6) | 6/6 done | — |
| 7A — Basic Document Management | done (5/5) | 5/5 done | — |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done · **last_checked 2026-11-03 (cursor re-check — no PRs touched OAuth)** | — |
| 10B — Platform Administration | done (7/7) | 7/7 done | — |
| 80 — Dispute Resolution | done (3/3) | 3/3 done | — |
| 9 / 79 / 81 / 82-85 (extended) | — | all done | #3217, #3218, #3219, #3220 (code-review + churn responses on already-shipped surfaces) |

## Shipped since last run (5 PRs — detail)

- **#3216** — `screen-map-drift-pr-3212-reality: reconcile account-listing-edit screen-map with PR #3212` (docs-only).
- **#3217** — `repeated-churn-frontend/apps/reality-web/.../edit/page.tsx: extract listing-edit numeric coercion into tested _number helper` (3 files, +86/-4). Resolves the 2026-11-02 repeated-churn signal.
- **#3218** — `code-review-ppt-web-ui-evidence-uploader-no-i18n: i18n EvidenceUploader validate errors, drop-zone & placeholder` (8 files, +90/-42).
- **#3219** — `churn-hotspot price_map.rs: escape LIKE metacharacters in city filter` (1 file, +38/-1). Addresses the 2026-10-25 churn-hotspot item on price_map.rs; user-controlled `city` is now parameter-escaped before being substituted into the LIKE pattern.
- **#3220** — `code-review-ppt-web-ui-community-feature-no-i18n: internationalize ppt-web community feature` (17 files, +432/-… across 6 locale JSONs plus 11 .tsx + 1 test). Resolves the ppt-web community i18n gap surfaced 2026-11-02.

## What's next (top 5)

1. **Resolve #3215 — merge pipeline wedge** (bot cannot grant GitHub APPROVED review to its own PRs) — unblocks 8 approved assignments. Owner: **pm-tech-lead**. Priority: **high**.
2. **Unblock #2652 — mobile-native/KMP builds in cloud runner** — still blocks 5+ KMP backlog items (incl. ready `create-listing-not-wired` plan now at score 5 after Tier-1d corroboration). Owner: **pm-devops**. Priority: **high** (carried).
3. **Lint sweep: deny `clippy::unwrap_or_default` in `routes/`** — three fresh instances in merged code this week. Owner: **pm-qa**. Priority: **medium**.
4. **Gate violations `add_evidence` sub-resource (#3164)** — tenant-scoped authz check + cross-tenant integration test. Owner: **pm-backend**. Priority: **high** (carried).
5. **#3168 cargo-deny advisories** — still blocking backend merges when the gate is enforced. Owner: **pm-security**. Priority: **high** (carried).

## Role focus today

- **pm-qa** (idx 3 → 4) — full output in `roles/pm-qa.md`. Summary: 5 next_actions (1 medium / 3 low / 1 high), 3 risks (incl. one meta on pipeline wedge normalisation). Decisions needed: resolution path for #3215.
