# PPT Project State

_Generated: 2026-10-20 — routine Phase 1.6 upkeep (pm-tech-lead rotation slot; pm-cursor idx 0 → 1) + pm-scrum-master always-on. Coverage `scan_kind=upkeep`; coverage_cursor idx 10 → 11 (epic-85 Build/CI configuration re-checked, both stories still done). 1-day lag since 2026-10-19 — on schedule._

## Delta since last routine run (2026-10-19)

- **2 PRs merged** (#3092, #2902):
  - **#3092** reality-server `PRICE_MAP_CACHE` bounded against anonymous key-flood OOM DoS (+163/-4, inline unit + tokio tests). Resolves backlog `code-review-reality-server-price-map-cache-unbounded` (`status: done`).
  - **#2902** screen-map docs reconcile for `docs/screens/reality/saved-searches.md` after PR #2894 i18n (+2 lines, docs-only). Already-done backlog item carried.
- **3 new draft PRs opened by dispatcher:**
  - **#3130** gh-issue-3126: bump ammonia to 4.2.4 to clear cargo-deny RUSTSEC-2026-0373.
  - **#3129** gh-issue-3128: type the AML assessments list endpoint response (follow-up from merged PR #3122).
  - **#3127** code-review-api-handlers-migration-upload-drops-file: persist migration import upload to storage (fix silent data loss on Epic 66 bulk-import).
- **1 critical new blocker: #3131** — Merge pipeline deadlocked: dispatcher bot cannot self-approve its own PRs; 16 approved PRs stuck in review (RUSTSEC bumps + the migration-upload data-loss fix among them). Labels: `follow-up`, `blocked`.
- **RUSTSEC chain extends:** #3124 (zip) → #3126 (ammonia follow-on; still fails after 4.1.7). Both now have in-flight draft PRs (#3125, #3130).

<sub>Prior upkeep note (archived): 2026-10-19 — pm-integration rotation; coverage_cursor 9 → 10 (epic-84 re-checked).</sub>

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** No status flips this run; `last_checked` refreshed on epic-85 (Build/CI) cursor-slice.
- **2 PRs merged** in the observation window — one SECURITY DoS fix on a public endpoint (price_map cache cap), one docs-only drift reconcile.
- **Merge pipeline is now the top risk**, surpassing the single-ticket RUSTSEC/egress blockers. #3131's approval-identity deadlock has queued 16 approved PRs behind a human-approver bottleneck; every day without resolution extends rebase/conflict exposure.
- **Buffer:** backend merges are triply gated — #2966 (swagger-ui egress), #3124/#3126 (cargo-deny), #3131 (approval deadlock). The IDOR tier remains parked.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 3/5** at sprint level; coverage-level all extended epics complete.

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
| 85 — Build/CI configuration | (extended) | 2/2 done · **last_checked 2026-10-20 (cursor re-check)** | — |
| 9 — 2FA/TOTP | (extended) | 1/1 done | — |
| Cross-cutting | — | — | #3092 (reality-server price_map DoS cap — SECURITY, infra hardening) |

† sprint-status.yaml drift against coverage.json — no action required this run.

## Shipped since last run (2 PRs merged)

- **#3092** — reality-server `PRICE_MAP_CACHE` bounded (SECURITY, DoS mitigation). Mirrors the `INQUIRY_RATE_LIMIT_MAX_IPS` precedent in `state.rs`. Inline unit + tokio tests included.
- **#2902** — docs(reality): saved-searches screen-map reconcile against PR #2894 i18n (docs-only; satisfies the earlier `screen-map-drift-pr-2894-reality` tracking item).

## Blockers (1 new, carried)

- **#3131 (NEW, critical)** — Merge pipeline deadlock: dispatcher bot cannot self-approve its own PRs; 16 approved PRs stuck in review. Owner: pm-devops + tech-lead.
- **#3126 (NEW, high)** — cargo-deny: ammonia 4.1.7 still fails RUSTSEC-2026-0373 after the 4.1.7 bump. Fix in flight (#3130, draft). Owner: pm-security.
- **#3124** — cargo-deny: rust_xlsxwriter → zip 8.6.0 RUSTSEC-2026-0367/0368/0369 (carried, high). Fix in flight (#3125, draft). Owner: pm-security.
- **#2966** — api-server utoipa-swagger-ui egress 403 (carried, high). Blocks backend cloud builds. Owner: pm-devops.
- **#2946 / #2944 / #2945** — cross-tenant IDOR retries, parked behind #2966 + the cargo-deny stack. Owner: pm-security.
- **#2652** — mobile-native/KMP cloud builds blocked. Owner: pm-devops.

## What's next (top 5 from the roadmap)

1. **Resolve #3131 approver-identity deadlock** (GitHub App or dedicated identity for auto-approve, drain the 16-PR queue oldest-first) — owner: pm-devops with tech-lead — **top priority today**.
2. **Land #3130 (ammonia 4.2.4) and #3125 (rust_xlsxwriter zip 9.x)** to clear the cargo-deny advisory wall — owner: pm-security.
3. **Review + land #3127** (migration import upload data-loss fix) — owner: pm-backend; verify storage-key tenancy + size cap + failure cleanup.
4. **Audit remaining public-endpoint in-memory caches** in reality-server for the same unbounded-key pattern as #3092 — owner: pm-security.
5. **Decide on 14-day-old draft #3028** (add_to_compare TOCTOU) — merge with atomic constraint or close with reason — owner: pm-backend.

## Role focus today: pm-tech-lead (+ pm-scrum-master always-on)

- **pm-tech-lead**: Flagged the approver-identity deadlock as the top architectural risk; set 5 concrete next-actions (approver identity, RUSTSEC land, migration-upload review, cache audit, #3028 decision) and 5 risks. Also raised sprint-status staleness vs. actual work focus.
- **pm-scrum-master**: Mild window (2 PRs merged, 3 drafts opened, 1 critical new issue). The merge-pipeline deadlock is now the single most costly blocker — more impactful than any one-ticket RUSTSEC/egress gate because it fans out across 16 PRs.

## Coverage (upkeep this run — 2026-10-20)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated=2026-10-20T<HH:MM>Z`, no re-scan.
- **Epic re-check: epic-85 Build/CI configuration (cursor idx 10)** — both stories still `done`; `last_checked = 2026-10-20` stamped. No PR in the 2026-10-19..2026-10-20 window touched Build/CI story surfaces (CI YAMLs; cargo-deny was patched but it's an advisory feed, not an epic-85 story).
- **Merged-PR evidence**: PR #3092 (price_map cache DoS cap) does not map to a coverage story; it's cross-cutting hardening tracked via backlog only. PR #2902 is docs-only.
- **`coverage_cursor` advances 10 → 11** (epic-85 → epic-8a next run).
- **`pm_cursor` advances 0 → 1** (pm-tech-lead → pm-backend next). `role_last_run["pm-tech-lead"] = 2026-10-20`.
- **Composition unchanged: 49 done · 0 partial · 0 not-started** across 13 epics.
