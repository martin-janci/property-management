# PPT Project State

_Generated: 2026-11-08 — routine Phase 1.6 incremental upkeep. Role focus: **pm-qa** (75-day stale rotation slot; pm-cursor idx 3 → 4). Coverage `scan_kind=upkeep`; coverage_cursor idx 0 → 1 (epic-10a OAuth re-checked, all 3 stories still done, no PRs touched OAuth surface). pm-scrum-master always-on._

## Delta since last routine run (2026-11-06)

- **5 PRs merged** (since last_pr_seen=3349): #3350, #3351, #3353, #3354, #3355 — all small, all against already-shipped surfaces, none flip a coverage story. Mix: 1 security (#3353 session revocation on password change, closes #3352), 1 boundary-validation (#3350 shared-comparison body parse), 2 i18n (#3351 + #3355 registry pet/vehicle forms + chrome), 1 auth-context (#3354 registry).
- **1 new critical infra issue: #3357** — "dispatcher: pipeline fully stalled — 3 approved PRs blocked on non-mechanical conflicts + Tier-1d signals are a dead-end". Pre-run dispatcher state is **0 claimed / 0 merged** (per-stack). Dominant delivery-health signal today.
- **Tier-1d signals write-and-forget pipeline broken** — 7 files written 2026-11-07 to `.research/signals/*-tier1d.json` were never ingested; every ppt-dev-review Phase 1.5 pass since the pipeline broke has produced write-only output.
- **Closed-not-merged dispatcher cleanup** this run: #3301 (reality-server inquiry dead FK classify), #3333 (screen-map drift PR #3293), #3356 (ppt-web registry mutation silent-fail). The #3356 orphan means the RegistryPage mutation-error-surface bug is **unfixed in dev** while sibling i18n/auth fixes (#3351/#3354/#3355) landed around it — action-list item flipped back to `open` this run.
- **1 new closed issue**: #3352 (change_password session revocation) — resolved by #3353.
- **Carried blockers unchanged**: #3168 (cargo-deny RUSTSEC advisories), #2652 (KMP cloud builds), #2951 (RN jest-expo), #3164 (violations add_evidence authz), #3131 (merge pipeline deadlock), #3187 (cross_org_assign_violation_is_not_found).

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** No status flips this run; `last_checked` refreshed on epic-10a (idx 0, OAuth Provider Foundation — stories 10a-1/10a-2/10a-3 still done).
- **5 PRs merged** — all follow-ups, no new feature work. A frozen pipeline (#3357) is masking test-shadow debt rather than revealing it; QA angle: without merged PRs test-shadow growth drops to zero but so does real coverage growth.
- **pm-qa rotation came up 75 days stale** — last run 2026-08-25. Six pm-qa next_actions queued (two high, three medium, one low). Five new risks appended (two high-high: Tier-1d dead pipeline + dispatcher stall).
- **Dominant delivery health: #3357 dispatcher stall.** Next unblocking step is either (a) planner refresh with non-mobile-native vectors or (b) operator unblock of the 2 UNRESOLVED depends_on sentinels (pm-qa/pm-security role-token poison in the depends_on normalization).

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 4/4** at sprint level (all four named sprint epics are status: done in sprint-status.yaml after the 2026-10-26 reconciliation). Coverage holds 13 epics all at done.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | done (6/6) | 6/6 done | — |
| 7A — Basic Document Management | done (5/5) | 5/5 done | — |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done · **last_checked 2026-11-08 (cursor re-check — epic-10a)** | — |
| 10B — Platform Administration | done (7/7) | 7/7 done | — |
| 80 — Dispute Resolution | done (3/3) | 3/3 done | — |
| 9 — 2FA/TOTP | done (1/1) | 1/1 done | — |
| 79/82/83/84/85 (extended) | — | all done | — |

PR #3353 touches `auth/password_reset.rs` (Property-Management password-reset flow), which is a sibling of but NOT the OAuth authorization server — no 10a contract change.

## Shipped since last run (5 PRs merged)

- **#3353** — security: revoke other sessions on authenticated password change (closes #3352). Owner surface: auth/password_reset.rs. Multi-session revocation regression test is a QA gap this run.
- **#3354** — bug: send auth context on registry requests (RegistryPage/RegistryRulesPage auth). Same class as the gate #486 that was cleared 2026-05-26.
- **#3355** — i18n: translate RegistryPage chrome (title/subtitle/Pets/Vehicles tabs/load-error). Resolves `code-review-ppt-web-ui-registry-no-i18n`.
- **#3351** — i18n: wire registry pet/vehicle forms + lists through react-i18next. Resolves `code-review-ppt-web-ui-registry-page-i18n-gap`.
- **#3350** — bug: validate shared-comparison listing bodies at API boundary (reality-web). Resolves `code-review-reality-web-share-cmp-unchecked-json`.

## Blockers (1 new critical, 6 carried)

- **#3357 (NEW, high)** — dispatcher pipeline fully stalled: 3 approved PRs blocked on non-mechanical conflicts + Tier-1d signals dead-end; pre-run 0 claimed / 0 merged. Owner: pm-scrum-master (operator-unblock recommended).
- **#3187** — Failing test on dev blocks backend CI: `cross_org_assign_violation_is_not_found` (assign returns 403, expects 404) (carried, high). Owner: pm-security.
- **#3168** — cargo-deny RUSTSEC advisories gate every backend PR merge (carried, high). Owner: pm-security.
- **#3164** — violations `add_evidence` sub-resource ungated (carried, high). Owner: pm-backend.
- **#3131** — merge pipeline deadlocked (carried, medium). Owner: pm-backend.
- **#2652** — mobile-native/KMP cloud builds blocked (carried, high). Owner: pm-devops.
- **#2951** — mobile RN jest-expo version rot (carried, medium). Owner: pm-devops.

## What's next (top 5 from the roadmap)

1. **Operator-unblock #3357 dispatcher stall** — either planner refresh with non-mobile-native vectors or clear the 2 UNRESOLVED depends_on sentinels. Everything downstream is gated on a non-zero claimable pool — owner: **pm-scrum-master**.
2. **Wire tier-1d signal ingestion OR stop emitting** — the 7 orphaned signals from 2026-11-07 are a write-only loop; add a CI/cron guard that fails on >48h stale signals — owner: **pm-qa** / **pm-devops**.
3. **Add multi-session revocation regression test (PR #3353 surface)** — assert OTHER refresh tokens revoked, CURRENT session preserved; close the "did not log me out of my own browser tab" failure mode before it ships — owner: **pm-qa**.
4. **Re-open and re-land RegistryPage mutation-error-surface fix (orphaned #3356)** — the user-facing silent-fail-on-destructive-action bug is unfixed in dev; the sibling i18n/auth fixes went in around it — owner: **pm-frontend**.
5. **Fix #3187 cross-tenant assign-404 contract** (carried; gates backend CI on dev) — owner: **pm-security**.

## Role focus today: pm-qa (+ pm-scrum-master always-on)

- **pm-qa** (75-day stale rotation): Dominant finding is NOT merged-PR test shadow (window is light) but pipeline-level dead integration — Tier-1d signals write-only + dispatcher stall at 0/0 claimed/merged. Six next_actions queued (two high: multi-session revocation regression for #3353, and tier-1d ingestion gate; three medium: shared-comparison body validator unit test for #3350, registry auth-context regression for #3354, dispatcher-fix-PR-requires-failing-test gate; one low: registry i18n missing-key guard). Five new risks: tier-1d dead pipeline (high/high), dispatcher pipeline stalled (high/high), PR #3353 other-session-revoke regression gap (medium/high), dispatcher-cleanup-orphan-items silent (medium/medium), registry mutation-silent-fail unfixed (medium/medium). The 2026-08-25 "dispatcher fix PRs require failing-on-main test" decision proposal is now 75 days unresolved — re-queued.
- **pm-scrum-master**: Mild 5-PR window of follow-ups BUT #3357 dominates — delivery health is pinned at 0/0 pre-run, which is an urgent planning signal, not a quiet day. Run is explicitly `quiet: false`. One next_action carried (`violations-xtenant-404-contract-sweep`).

## Coverage (upkeep this run — 2026-11-08)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated=2026-11-08T00:00:00Z`, no re-scan.
- **Epic re-check: epic-10a OAuth Provider Foundation (cursor idx 0)** — stories 10a-1/10a-2/10a-3 all still `done`; `last_checked = 2026-11-08` stamped. No merged PR in the 2026-11-07..2026-11-08 window touched the OAuth authorization server (routes/oauth.rs). PR #3353 touches routes/auth/password_reset.rs — the password-reset flow is a sibling of, not an instance of, the OAuth authorization server, so no 10a contract change.
- **Merged-PR evidence check:** 5 PRs touch registry UI (#3351/#3354/#3355), reality-web comparison boundary (#3350), and auth/password_reset.rs (#3353); none map to a story's granular coverage delta since the relevant stories are all `done` and the fixes hit sub-features.
- **`coverage_cursor` advances 0 → 1** (epic-10a → epic-10b next run).
- **`pm_cursor` advances 3 → 4** (pm-qa → pm-devops next). role_last_run["pm-qa"] = 2026-11-08.
- **Composition unchanged: 49 done · 0 partial · 0 not-started** across 13 epics. Missing UC links: 3 (UC-33.1/33.2/33.3, already queued elsewhere).
