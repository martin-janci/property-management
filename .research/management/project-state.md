# PPT Project State

_Generated: 2026-10-19 — routine Phase 1.6 upkeep (pm-integration rotation slot; pm-cursor idx 7 → 0, wrapping) + pm-scrum-master always-on. Coverage `scan_kind=upkeep`; coverage_cursor idx 9 → 10 (epic-84 Documents & e-signature re-checked, all 5 stories still done, no material change). 3-day lag since 2026-10-16 run — on schedule._

## Delta since last routine run (2026-10-16)

- **2 PRs merged** (#3121, #3122) — both dispatcher-authored code-review follow-ups on Tier-1d signals:
  - **#3121** reality-web auth-context transient-logout cushion (+159/-2, tests included) — resolves backlog `code-review-reality-web-auth-ctx-transient-logout` (`status: done`).
  - **#3122** ppt-web-core useAmlDashboard AML flags (+117/-6, tests included) — resolves backlog `code-review-ppt-web-core-aml-checks-hardcoded-false` / `code-review-ppt-web-core-aml-risk-factors-unguarded-map` (`status: done`).
  - Zero coverage status flips (both surfaces already shipped). Evidence appended on 10a-1 for #3121 (SSO consumer hardening); #3122 maps to no coverage story (compliance epic not in current coverage map).
- **1 new blocker (critical): #3124** — `cargo-deny advisories FAILED on dev: RUSTSEC-2026-0368 / RUSTSEC-2026-0369 (zip 8.6.0 via rust_xlsxwriter)` — this gates **every backend PR merge** on top of the still-open #2966 (swagger-ui egress) that gates backend cloud builds. Pre-claimed by dispatcher as `gh-issue-3124` (commit 62aaaa5).
- **1 open draft PR: #3123** — api-core TOTP RNG expect panic (dispatcher-authored, follow-up on backlog signal `code-review-api-core-totp-rng-expect-panic`); no contract impact.
- **PM-integration rotation note:** pm-integration was last run 2026-07-24 — nearly 3 months stale. The rotation hits it on wrap-around today; a fresh integration lens on the merge/build gate stack is overdue and timely given #3124.

<sub>Prior upkeep note (archived): 2026-10-16 — pm-data rotation; coverage_cursor 8 → 9 (epic-83 re-checked).</sub>

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** No status flips this run; `last_checked` refreshed on 10a-1 (PR #3121 evidence) and all 5 epic-84 stories (cursor re-check — negative, nothing touched those surfaces).
- **2 PRs merged** in the observation window: both frontend code-review follow-ups, both tests-included, both resolving prior backlog items.
- **Buffer still constrained, now doubly:** #2966 (swagger-ui egress) gates backend builds; **#3124 cargo-deny** (brand new) gates backend merges even on green builds; #2652 (KMP) continues to park mobile-native work. Expect a tier-1 refresh of the IDOR retries only once BOTH #2966 and #3124 are resolved.
- **One new high-priority issue: #3124** — classic RUSTSEC advisory on a transitive dep (`rust_xlsxwriter → zip 8.6.0`); mitigation is either a version bump or a bounded cargo-deny exception. Pre-claimed by dispatcher; needs resolution within 24-48h to avoid merge-queue starvation.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 3/5** at sprint level (sprint-status.yaml); coverage-level all extended epics are complete.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | in-progress (3/6)† | 6/6 done | — |
| 7A — Basic Document Management | in-progress (2/5)† | 5/5 done | — |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done | #3121 (reality-web auth-ctx transient-logout — SSO consumer hardening) |
| 10B — Platform Administration | in-progress (7/7)† | 7/7 done | — |
| 80 — Dispute Resolution | partial (1/3)† | 3/3 done | — |
| 79 — API client integration | (extended) | 4/4 done | — |
| 82 — iOS Reality Portal (SwiftUI) | (extended) | 5/5 done | — |
| 83 — Portal Integrations | (extended) | 3/3 done | — |
| 84 — Documents & e-signature | (extended) | 5/5 done · **last_checked 2026-10-19 (cursor re-check)** | — |
| 85 — Build/CI configuration | (extended) | 2/2 done | — |
| 9 — 2FA/TOTP | (extended) | 1/1 done | — |
| Cross-cutting | — | — | #3122 (ppt-web AML dashboard flags — compliance epic not in coverage) |

† sprint-status.yaml drift against coverage.json — no action required this run; deep scan will reconcile.

## Shipped since last run (2 PRs merged)

- **#3121** — reality-web auth-context transient-logout cushion: don't logout on transient 401 from reality-server /auth/refresh (resolves backlog `code-review-reality-web-auth-ctx-transient-logout`; +159/-2 with tests)
- **#3122** — ppt-web-core useAmlDashboard AML flags: AML risk flags no longer hardcoded false / risk factors map now guarded (resolves backlog `code-review-ppt-web-core-aml-checks-hardcoded-false` + `code-review-ppt-web-core-aml-risk-factors-unguarded-map`; +117/-6 with tests)

## Blockers (1 new, carried)

- **#3124 (NEW, high)** — cargo-deny advisories FAILED on dev: RUSTSEC-2026-0368 / RUSTSEC-2026-0369 on zip 8.6.0 via rust_xlsxwriter. Blocks every backend PR merge. Owner: pm-security. Pre-claimed by dispatcher.
- **#2966** — api-server utoipa-swagger-ui egress 403 (carried, high). Blocks backend cloud builds. Owner: pm-devops.
- **#2946 / #2944 / #2945** — cross-tenant IDOR retries, all parked behind #2966 (and now additionally #3124). Owner: pm-security.
- **#2652** — mobile-native/KMP cloud builds blocked. Owner: pm-devops.
- **#2951** — mobile RN jest-expo version rot. Owner: pm-devops.

## What's next (top 5 from the roadmap)

1. **Resolve #3124 cargo-deny advisories** (bump rust_xlsxwriter or bounded cargo-deny exception for RUSTSEC-2026-0368/0369) — owner: pm-security — **gates every backend merge today**.
2. **Unblock #2966 swagger-ui egress** — owner: pm-devops — gates every backend cloud build; combined with #3124 is the two-key lock keeping the IDOR tier parked.
3. **Land #2946 portfolio_analytics IDOR fix** (retry 2/2 draft open) — owner: pm-security — unblocks on resolution of #2966 + #3124.
4. **#3006 unify triplicated auth-header + MFA-retry** in @ppt/api-client — owner: pm-tech-lead — security-overtones refactor, carried.
5. **Document the reality-server SSO consumer transient-401 contract** (PR #3121 cushion) in docs/api/README.md so client/server treat transient vs permanent 401 symmetrically — owner: pm-integration (new).

## Role focus today: pm-integration (+ pm-scrum-master always-on)

- **pm-integration** (3-month stale rotation): The integration-contract surface is quiet (both merged PRs are client-side cushions, no OpenAPI change). The current merge-queue blocker (#3124) is itself an integration-level failure mode — the external advisory feed vetoes the merge path even when the internal change is clean. Three concrete follow-ups opened:
  1. Audit integration-touching PRs since 2026-10-17 for OpenAPI/TypeSpec drift (medium).
  2. Surface the RUSTSEC-exception policy in the cargo-deny CI gate description (medium, blocked on #3124).
  3. Document the reality-server SSO consumer transient-401 contract (low).
- **pm-scrum-master**: Mild week — 2 PRs merged, 1 draft open, 1 critical new issue. The twin merge-gate (#2966 + #3124) is the only noteworthy change in delivery posture since the 2026-10-16 run.

## Coverage (upkeep this run — 2026-10-19)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated=2026-10-19T04:00:00Z`, no re-scan.
- **Epic re-check: epic-84 Documents & e-signature (cursor idx 9)** — all 5 stories (84-1..84-5) still `done`; no PR in the 2026-10-18..2026-10-19 window touched S3 presigned / e-signature email / price-tracking / notification-trigger / pgvector RAG surfaces. `last_checked = 2026-10-19` stamped on all 5. Negative-check evidence appended on each.
- **Merged-PR evidence appended** on 10a-1-oauth-authorization-server for PR #3121 (SSO consumer hardening). PR #3122 (AML dashboard flags) does not map to a coverage story — compliance epic is not in the current coverage map; backlog signal was the only tracking point and is now done.
- **`coverage_cursor` advances 9 → 10** (epic-84 → epic-85 next run).
- **`pm_cursor` advances 7 → 0** (pm-integration → pm-tech-lead next; rotation wraps). role_last_run["pm-integration"] = 2026-10-19.
- **Composition unchanged: 49 done · 0 partial · 0 not-started** across 13 epics. Missing UC links: 3 (UC-33.1/33.2/33.3, already queued elsewhere).
