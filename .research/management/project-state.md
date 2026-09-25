# PPT Project State

_Generated: 2026-09-25 — routine Phase 1.6 lightweight upkeep (pm-security rotation slot; last run 2026-07-21, 66d stale) + pm-scrum-master always-on. Coverage `scan_kind=upkeep`; pm_cursor idx 5 → 6 (pm-security → pm-data next), coverage_cursor idx 7 → 8 (epic-82 re-checked, no material change, advances to epic-83). Zero application-code merges this window — dispatcher heartbeats only._

## Executive summary

- **Delivery stalled on reviewer throughput, not on cloud capacity.** 0 application PRs merged in the 31h window since the last routine run (2026-09-24T03:19Z); all 20 commits on `dev` are `.research/` dispatcher heartbeats.
- **Two live cross-tenant IDOR fixes sit unmerged in draft on `needs-human-review`:**
  - **PR #2977** (gh-issue-2944-retry1) — `violations.rs` `list_evidence` / `list_payments` / `list_comments` drop `AuthUser` entirely, and the underlying repo methods filter with `&self.pool` (no org, no RLS) — cross-org read of violation evidence, fine payments, and internal-only comments.
  - **PR #2976** (gh-issue-2945-retry1) — `portfolio_performance.rs` property sub-resource endpoints skip the `org_id` check that sibling portfolio-level handlers apply — write-capable cross-tenant IDOR (list / get / update / remove of any tenant's portfolio-property financial records).
  Both were originally issues #2944 / #2945 closed on 2026-09-23 while the underlying vulnerable code was still on `dev` — the closure criteria did not require merge; the retry1 cycle recovers exposure. See `roles/pm-security.md` for full details.
- **`needs-human-review` queue depth: 7 PRs.** #2977, #2976 (new today, security IDOR retries) + #2969, #2968, #2967 (reality-web silent-fail mutations, 3–4d old) + #2902 (screen-map-drift reconcile, 25d open / 7d idle) + #2744 (dispatcher fix, 43d open / 43d idle). Reviewer throughput is the delivery bottleneck this cycle.
- **Cloud claimable buffer starved (8/72).** All remaining backlog is cloud-unlandable per three infra blockers: issue #2966 (utoipa-swagger-ui build-script egress-blocked — gates api-server verify), issue #2951 (jest-expo 56 vs RN 0.87 rot — mobile RN jest suite), issue #2652 (KMP AGP egress-blocked — mobile-native, closed but still governing). Every dispatcher tick this run recorded 0 claimed / 0 reviewed / 7 active.
- **Rotating expert-review yield (Tier-1d, ppt-web-core segment, 2026-09-25):** 2 net-new findings — `code-review-ppt-web-core-fault-detail-mutations-swallow-errors` (bug, score 2, high confidence — 7 fault-workflow mutations fire with no onError; UX regression on manager-critical actions) and `code-review-ppt-web-core-org-provider-value-not-memoized` (bug, score 1, medium confidence — render-perf smell).
- **No plans promoted this run.** New backlog rows are all score 1–2; Phase 3 threshold is score ≥ 3 (or ≥ 2 for high-confidence `security` vector). The 3 existing open reality-web silent-fail items have live PRs in flight; the 2 ready mobile-native KMP plans stay cloud-unlandable.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 2/6** per sprint-status.yaml.

| Epic | Sprint status | Coverage status (13 epics) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6/6 stories done in coverage |
| 7A — Basic Document Management | in-progress | 5/5 stories done in coverage |
| 8A — Basic Notification Preferences | done | 3/3 stories done |
| 10A — OAuth Provider Foundation | done | 3/3 stories done |
| 10B — Platform Administration | in-progress | 7/7 stories done |
| 80 — Dispute Resolution | partial | 3/3 stories done in coverage |
| 82 — iOS SwiftUI | (extended) | 5/5 stories done in coverage; **re-checked this run (idx 7), no material change, last_checked=2026-09-25** |
| 84 — Documents / e-signature | (extended) | 3/5 done, 2 partial (84-1, 84-2) — unchanged |

**Reconciliation ask (pm-scrum-master action):** epics 6 / 7A / 10B / 80 show `in-progress` / `partial` in sprint-status.yaml with stale `stories_completed` counts while every constituent story is `development_status: done`. Either roll the epics block forward or document the gap.

## Blockers

- **PRs #2976 / #2977** (live cross-tenant IDORs, drafts on `needs-human-review`) — human reviewer / tech-lead. Fast-track ahead of dependabot + correctness backlog.
- **PRs #2969 / #2968 / #2967 / #2902 / #2744** — remaining `needs-human-review` queue. Human reviewer.
- **Issue #2966** (utoipa-swagger-ui build-script egress) — blocks api-server verify in cloud runner. Infra / rust-backend.
- **Issue #2951** (jest-expo 56 vs RN 0.87 rot) — mobile RN jest suite. Mobile / react-native.
- **Issue #2652** (mobile-native/KMP AGP egress, closed but governing) — 2 ready KMP plans (`code-review-mobile-native-kmp-inquiries-response-contract`, `code-review-mobile-native-kmp-create-listing-not-wired`) sit cloud-unlandable. Mobile-native / kotlin.

## Role focus today: **pm-security** (rotation idx 5; last 2026-07-21, 66d stale) + pm-scrum-master always-on

- **pm-scrum-master** (always-on): delivery is bottlenecked on human review, not on cloud dispatcher capacity — 0 merges in 31h while 7 PRs (2 new security IDORs) pile up on `needs-human-review`. Sprint velocity is invisible from the routine's perspective this run; refill-planner ask is real. See `roles/pm-scrum-master.md`.
- **pm-security** (rotation): both retry PRs #2977 / #2976 target confirmed, still-live cross-org IDORs on `dev`. Same discarded-AuthUser anti-pattern surfaced independently in two unrelated features this sprint (violations, portfolio_performance) — repo-wide audit surfaced as this run's medium-priority action. See `roles/pm-security.md`.

## Coverage (upkeep this run — 2026-09-25)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated` bumped to 2026-09-25T10:53 UTC, no re-scan.
- **Epic re-check: epic-82 (iOS SwiftUI)** — cursor idx 7. All 5 stories (SwiftUI project setup, Navigation/Routing, Home/Search, Listing Detail/Favorites, Inquiries/Account) still `done`. No PR in the 31h window touched iOS or mobile-native surface; negative check recorded. `last_checked = 2026-09-25` stamped on all 5 stories.
- **Merged-PR evidence:** none — zero merges this window; no status flips.
- **`coverage_cursor` advances 7 → 8** (epic-82 → epic-83 next run).
- **`pm_cursor` advances 5 → 6** (pm-security → pm-data next run). `role_last_run["pm-security"] = 2026-09-25`.
- **Composition unchanged: 47 done · 2 partial · 0 not-started** across 13 epics. No new orphan screens; no coverage validation errors.
