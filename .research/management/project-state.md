# PPT Project State

_Generated: 2026-10-26 — routine Phase 1.6 incremental upkeep. Role focus: **pm-backend** (87-day stale rotation slot; pm-cursor idx 1 → 2). Coverage `scan_kind=upkeep`; coverage_cursor idx 11 → 12 (epic-8a Notification Preferences re-checked, all 3 stories still done, no material change). pm-scrum-master always-on._

## Delta since last routine run (2026-10-22)

- **3 PRs merged** (#3159, #3160, #3166) — all reality-web code-review follow-ups:
  - **#3159** reality-web realtor-api shape validation (defensive parsing)
  - **#3160** reality-web safe-error log redaction (PII hygiene)
  - **#3166** reality-web agency i18n price fix (localisation defect)
  - **Zero coverage status flips** — none of the three map to a coverage story (reality-web agency/realtor-api surfaces aren't tracked as stories; the only reality-web items in coverage are mobile-native epic-82 which is 5/5 done).
- **1 new critical blocker: #3168** — `cargo-deny advisories FAILED on dev: 3 new RUSTSEC advisories blocking backend merge pipeline`. **Second cargo-deny gate in a month** after #3124 cleared on 2026-10-19 — dependency hygiene is becoming a recurring merge-queue tax. Queued as `gh-issue-3168` (high, owner pm-security).
- **2 new follow-up issues: #3163 (unassign 404), #3164 (violations add_evidence ungated)** — both have open drafts (#3167 for unassign; nothing yet for add_evidence authz).
- **3 open drafts** (#3161 push_fanout FCM eviction, #3165 migration_import test coverage, #3167 unassign moderation case) — all backend-touching, all awaiting review or CI (gated on #3168 if backend-merge path).
- **5 follow-ups closed on PR merge:** #3069, #3098, #3109, #3157, #3162 (naturally resolved — no action-list items were tracking them).

<sub>Prior upkeep note (archived): 2026-10-22 — pm-tech-lead rotation; coverage_cursor 10 → 11 (epic-85 re-checked).</sub>

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** No status flips this run; `last_checked` refreshed on all 3 epic-8a stories (cursor re-check — negative, nothing in the window touched notification preferences surfaces).
- **3 PRs merged** — all reality-web follow-ups, all tests-included, all narrow fixes.
- **Buffer still constrained:** the long-carried #2652 (KMP cloud builds), #2951 (RN jest-expo), and the historical merge-queue blockers now stacked with **new #3168 (RUSTSEC advisories)**. The dispatcher backlog scan should refresh the action-list candidate pool externally.
- **pm-backend rotation came up 87 days stale** — last run 2026-07-30; today's lens surfaces a tier-1 authz gap on violations `add_evidence` (#3164) with the same shape as the historical dispute `add_evidence` gap (#2483), hinting at a systemic blind spot in `add_evidence`-pattern sub-routes.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 3/5** at sprint level (sprint-status.yaml); coverage-level all extended epics are complete.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | in-progress (3/6)† | 6/6 done | — |
| 7A — Basic Document Management | in-progress (2/5)† | 5/5 done | — |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done · **last_checked 2026-10-26 (cursor re-check)** | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done | — |
| 10B — Platform Administration | in-progress (7/7)† | 7/7 done | — |
| 80 — Dispute Resolution | partial (1/3)† | 3/3 done | — |
| 79 — API client integration | (extended) | 4/4 done | — |
| 82 — iOS Reality Portal (SwiftUI) | (extended) | 5/5 done | — |
| 83 — Portal Integrations | (extended) | 3/3 done | — |
| 84 — Documents & e-signature | (extended) | 5/5 done | — |
| 85 — Build/CI configuration | (extended) | 2/2 done | — |
| 9 — 2FA/TOTP | (extended) | 1/1 done | — |
| Cross-cutting (reality-web) | — | — | #3159, #3160, #3166 (realtor-api validation / safe-error redaction / agency i18n — none map to coverage stories) |

† sprint-status.yaml drift against coverage.json — queued as `pm-scrum-master-reconcile-sprint-status-vs-coverage` (low); a deep scan would resolve it authoritatively.

## Shipped since last run (3 PRs merged)

- **#3159** — reality-web: realtor-api shape validation (defensive parsing)
- **#3160** — reality-web: safe-error log redaction (PII hygiene)
- **#3166** — reality-web: agency i18n price fix (localisation defect)

## Blockers (1 new critical, carried)

- **#3168 (NEW, critical)** — cargo-deny advisories FAILED on dev: 3 new RUSTSEC advisories gate every backend PR merge. Second cargo-deny gate in a month. Owner: pm-security.
- **#3164 (NEW, high)** — violations add_evidence sub-route ungated (cross-tenant IDOR shape, same as historical #2483). Owner: pm-backend.
- **#3163 (NEW, medium)** — unassign moderation case returns 404 (contract bug). Draft PR #3167 open. Owner: pm-backend.
- **#3131** — merge pipeline deadlocked (carried, medium). Owner: pm-backend.
- **#2652** — mobile-native/KMP cloud builds blocked (carried, high). Owner: pm-devops.
- **#2951** — mobile RN jest-expo version rot (carried, medium). Owner: pm-devops.

## What's next (top 5 from the roadmap)

1. **Resolve #3168 cargo-deny advisories** (bump offending crate(s) or bounded exceptions) — owner: pm-security — **gates every backend merge today**.
2. **Gate violations add_evidence sub-resource (#3164)** with a tenant-scoped authz check + cross-tenant regression test — owner: pm-backend — tier-1 IDOR.
3. **Diagnose #3131 merge-pipeline deadlock** — owner: pm-backend — carried, needs a traced repro, not more guessing.
4. **Sweep `add_*` sub-routes for the same ungated pattern** (announcements/attachments, documents/signatures, mediation/evidence) — owner: pm-backend — one-shot audit before shipping the #3164 fix.
5. **Reconcile sprint-status.yaml drift vs coverage.json** (6/7A/10B/80 all show in-progress/partial despite coverage=done) — owner: pm-scrum-master — low priority, deep scan would resolve.

## Role focus today: pm-backend (+ pm-scrum-master always-on)

- **pm-backend** (87-day stale rotation): Two concrete tier-1 findings — #3168 (RUSTSEC merge gate) and #3164 (violations add_evidence authz gap, same shape as #2483). Six next_actions queued (two high, two medium, two low); one proposed audit (sweep `add_*` sub-routes) to turn #3164 into a one-shot pattern-fix instead of a per-site whack-a-mole. Carried risks `pm-backend-delete-by-file-key-samesorg-ref-gap-2026-07-30` and `pm-backend-disputes-kpis-window-validation-gap-2026-07-30` are both still open — un-quarantining the KPIs test is now a `low` action item.
- **pm-scrum-master**: Mild window — 3 reality-web PRs merged, 3 drafts open, 1 critical new issue. The sprint-status.yaml drift vs coverage is now a chronic condition that only a deep scan can resolve authoritatively; queued as a low-priority action rather than a blocker.

## Coverage (upkeep this run — 2026-10-26)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated=2026-10-26T04:30:00Z`, no re-scan.
- **Epic re-check: epic-8a Basic Notification Preferences (cursor idx 11)** — all 3 stories (8a-1/8a-2/8a-3) still `done`; no merged PR in the 2026-10-23..2026-10-26 window touched notification-preference / critical-override / preference-sync surfaces. `last_checked = 2026-10-26` stamped on all 3.
- **Merged-PR evidence check:** #3159, #3160, #3166 all reality-web; none map to a coverage story (no evidence appended).
- **`coverage_cursor` advances 11 → 12** (epic-8a → epic-9 next run).
- **`pm_cursor` advances 1 → 2** (pm-backend → pm-frontend next). role_last_run["pm-backend"] = 2026-10-26.
- **Composition unchanged: 49 done · 0 partial · 0 not-started** across 13 epics. Missing UC links: 3 (UC-33.1/33.2/33.3, already queued elsewhere).
