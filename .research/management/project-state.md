# PPT Project State

_Generated: 2026-10-21 — routine Phase 1.6 upkeep (pm-tech-lead rotation slot; pm-cursor idx 0 → 1) + pm-scrum-master always-on. Coverage `scan_kind=upkeep`; coverage_cursor idx 10 → 11 (epic-85 Build/CI configuration re-checked, both 2 stories still done, no material change). 2-day lag since 2026-10-19 run — on schedule. Trigger: buffer-low (claimable=12/72 — refill planner)._

## Delta since last routine run (2026-10-19)

- **5 PRs merged** — the most active window since early October:
  - **#3125** (zip 9.x via rust_xlsxwriter) — clears RUSTSEC-2026-0367/0368/0369; the carried #3124 cargo-deny blocker from the 2026-10-19 run is now **resolved**.
  - **#3127** (migration import upload persist) — fixes a silent dead-store (upload buffer dropped before S3 persist); regression test `migration_import_upload_persists_file.rs` fails before the fix.
  - **#3129** (typed AML assessments list) — replaces untyped `serde_json::Value` with `AmlAssessmentListItem` / `ListAmlAssessmentsResponse` + `From<AmlRiskAssessment>` + utoipa `ToSchema`.
  - **#3130** (ammonia 4.2.4) — clears RUSTSEC-2026-0373 (CSS url case-insensitive).
  - **#3135** (rustls-webpki) — clears RUSTSEC-2026-0382 and formally reopens the cargo-deny merge gate.
- **1 new critical blocker: #3131** — _dispatcher merge pipeline fully deadlocked_. The dispatcher token acts as `martin-janci`, so it can only COMMENT, never APPROVE, which means `reviewDecision` cannot reach APPROVED and `ppt-pr-merge` aborts on all 16 approved-by-bot PRs. **This is now the single structural blocker on everything downstream** — the 5 stalled drafts (#3028/#3070/#3072/#3074/#3075) all share this root cause.
- **2 new follow-up issues** — #3132 (PRICE_MAP_CACHE DoS: O(n log n) sort-on-insert + attacker-controlled keys) and #3136 (migration-upload StorageError typed match + happy-path test).
- **1 closed-not-merged** — #2981 superseded by dependabot #3133.
- **PM-tech-lead rotation note:** pm-tech-lead was last run 2026-10-01 — three weeks stale. The rotation hits it today precisely as the architectural root-cause (#3131) emerges; the role's analysis names dispatcher-identity split as the top priority.

<sub>Prior upkeep note (archived): 2026-10-19 — pm-integration rotation; coverage_cursor 9 → 10 (epic-84 re-checked).</sub>

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** No status flips this run; `last_checked` refreshed on epic-85 stories (85-1 environment-variables, 85-2 build-configuration) via cursor re-check — negative, nothing in the 5 merged PRs touched mobile config surfaces.
- **5 PRs merged** in the observation window: 4 security advisory bumps + 1 silent-data-loss fix + 1 typed-response refactor. All test-coveraged, all cleanly merged.
- **Merge gate rotated**: #3124 (RUSTSEC advisories) is cleared; #3131 (bot self-approval) is the new, worse blocker. #2966 (swagger-ui egress) and #2652 (KMP cloud builds) remain carried.
- **Buffer stays low**: claimable=12/72 (up from 9/72 but still well under the 30% refill target). The dispatcher sent this buffer-low trigger because 16 approved PRs are frozen downstream of #3131.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 3/5** at sprint level (sprint-status.yaml); coverage-level all extended epics are complete.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | in-progress (3/6)† | 6/6 done | — |
| 7A — Basic Document Management | in-progress (2/5)† | 5/5 done | — |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done | — |
| 10B — Platform Administration | in-progress (7/7)† | 7/7 done | — |
| 80 — Dispute Resolution | partial (1/3)† | 3/3 done | — |
| 79 — API client integration | (extended) | 4/4 done | — |
| 82 — iOS Reality Portal (SwiftUI) | (extended) | 5/5 done | — |
| 83 — Portal Integrations | (extended) | 3/3 done | — |
| 84 — Documents & e-signature | (extended) | 5/5 done | — |
| 85 — Build/CI configuration | (extended) | 2/2 done · **last_checked 2026-10-21 (cursor re-check)** | — |
| 9 — 2FA/TOTP | (extended) | 1/1 done | — |
| Cross-cutting | — | — | #3125, #3127, #3129, #3130, #3135 |

† sprint-status.yaml drift against coverage.json — no action required this run; deep scan will reconcile.

## Shipped since last run (5 PRs merged)

- **#3125** — bump rust_xlsxwriter to zip 9.x, clear zip RUSTSEC-2026-0367/0368/0369 (closes #3124). pm-security.
- **#3127** — persist migration import upload to storage (fix silent data loss); regression test fails-before-fix. pm-backend.
- **#3129** — type the AML assessments list endpoint response (`AmlAssessmentListItem` + `ListAmlAssessmentsResponse` + `From<AmlRiskAssessment>`). pm-backend.
- **#3130** — bump ammonia to 4.2.4 to clear RUSTSEC-2026-0373 (CSS url case-insensitive). pm-security.
- **#3135** — clear rustls-webpki RUSTSEC-2026-0382 to unblock cargo-deny advisories. pm-security.

## Blockers (1 new critical, carried)

- **#3131 (NEW, CRITICAL)** — dispatcher merge pipeline deadlocked: bot cannot self-approve its own PRs. 16 approved-by-bot PRs frozen; all 5 stalled drafts (#3028/#3070/#3072/#3074/#3075) blocked here. Owner: pm-tech-lead (coordinating with pm-devops). This is now the single structural blocker.
- **#2966** — api-server utoipa-swagger-ui egress 403 (carried, high). Blocks backend cloud builds. Owner: pm-devops.
- **#2946 / #2944 / #2945** — cross-tenant IDOR retries, all parked behind #2966. Owner: pm-security.
- **#2652** — mobile-native/KMP cloud builds blocked. Owner: pm-devops.
- **#2951** — mobile RN jest-expo version rot. Owner: pm-devops.

## What's next (top 5 from the roadmap)

1. **Resolve #3131 dispatcher merge deadlock** — owner: pm-tech-lead. Provision a distinct dispatcher-bot GH identity (separate PAT or GH App) or amend `ppt-pr-merge` to accept bot-approvals-of-self. Gates the entire merge pipeline.
2. **Address #3132 PRICE_MAP_CACHE DoS follow-up** — owner: pm-backend. Replace O(n log n) insert_bounded with LRU, constrain cache keys. Follow-up on PR #3092 post-merge review.
3. **Address #3136 migration-upload typed storage-error + happy-path test** — owner: pm-backend. Match on typed `StorageError::InvalidContentType` variant rather than Display string.
4. **Unblock #2966 swagger-ui egress** — owner: pm-devops. Still carried; combined with resolution of #3131 is the two-key lock on the IDOR tier.
5. **Close #3006 — collapse @ppt/api-client triplicated auth-header + MFA-retry logic** — owner: pm-tech-lead. Carried risk; Biome rule to prevent regression.

## Role focus today: pm-scrum-master, pm-tech-lead

- **pm-tech-lead** (3-week stale rotation; last run 2026-10-01): The merge-pipeline deadlock (#3131) is an architectural failure of identity mapping — the dispatcher-bot token's actor binding is incorrect for the `reviewDecision` state machine. Named this as the #1 priority (above #3132 and #3136). Also flagged the typed-response refactor opportunity post-#3129 and reaffirmed the carried #3006 auth-header-triplicate risk. Six concrete follow-ups opened — dispatcher-identity split, #3132 DoS, #3136 typed error, #3006 collapse, typed-response rotation plan, RUSTSEC exception ADR.
- **pm-scrum-master**: Active week — 5 PRs merged, 1 critical new issue (#3131), 2 follow-up issues (#3132, #3136), 5 stalled drafts all tied to the single root cause. Delivery velocity is good (5 merges in 36h) but is now fully gated on resolving the bot-approval deadlock.

## Coverage (upkeep this run — 2026-10-21)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated=2026-10-21T04:00:00Z`, no re-scan.
- **Epic re-check: epic-85 Build/CI configuration (cursor idx 10)** — both 2 stories (85-1 environment-variables, 85-2 build-configuration) still `done`; no PR in the 2026-10-19..2026-10-21 window touched Expo app.config.ts / xcconfig / gradle flavors / env templates. `last_checked = 2026-10-21` stamped on both. Negative-check evidence appended on each.
- **Merged-PR evidence appended** — no coverage story matches the 5 merged PRs (all are backend-only security/typing/data-persistence changes; no mobile config, no frontend screen-map surface). Each PR tracked only in action-list / backlog.
- **`coverage_cursor` advances 10 → 11** (epic-85 → epic-8a next run).
