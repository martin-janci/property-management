# PPT Project State

_Generated: 2026-09-08 — routine Phase 1.6 lightweight upkeep (pm-security rotation slot; 49d stale slot refreshed) + pm-scrum-master always-on. Coverage `scan_kind=upkeep`; pm_cursor idx 5 → 6 (pm-security → pm-data next), coverage_cursor idx 7 → 8 (epic-82 re-checked, no material change; advances to epic-83). Sprint window 2026-09-01..09-08 shipped 14 PRs (mixed code-review + gh-issue security fixes) but the highest-severity finds (3 cross-tenant IDOR handlers) are stranded by infra #2949._

## Executive summary

- **Delivery still at 47/49 stories done, 2 partial** (84-1 direct-to-S3 upload wiring and 84-2 sign page). No status flips this window. 14 PRs merged since 2026-09-01, of which 5 landed real security wins on the frontend/reality-server surface (typed error enum, raw DB-error leak fix in compliance, single-flight 401 refresh + replay, cold-boot JWT-exp validation, allowlist auto-derivation, shared-device cache purge) and the rest were correctness/i18n/test-coverage.
- **Security debt spiked this window (opened 2026-09-06):** three publicly-labelled cross-tenant IDOR bugs — **#2944** (violations comments/evidence/payments + internal-notes privilege leak), **#2945** (portfolio_properties read+write), **#2946** (portfolio_analytics computes `org_id` then discards it; confirmed at `routes/portfolio_analytics.rs:282,314`). All three require api-server changes and **cannot land in the cloud pipeline** because infra **#2949** (utoipa-swagger-ui build-script egress) makes `cargo build -p api-server` red in the sandbox.
- **Two new infra blockers this window:** **#2949** (api-server cloud build — strands all IDOR fixes) and **#2951** (mobile jest — jest-expo@56 ↔ react-native@0.87 version rot; caused PR #2950 to merge red-CI even though the diff itself is IG3-proven).
- **Auto-review loop still working on landable stacks:** every merged PR was originated by the ppt-dev-review generator or a gh-issue implementer and merged post-review with zero regressions detected. But the loop can only ship what the cloud runner can build — currently that excludes api-server, mobile-rn (jest suite), and mobile-native (AGP).
- **Buffer starved (cloud-only):** 8/72 claimable per Phase 1 trigger; 6/9 pre-run open items were mobile-native/KMP infra-blocked. This run adds 7 new items (IDOR trio + swagger-ui unblock + antipattern sweep + jest infra + handoff test + h2 RUSTSEC), bringing open count to 16 but 4 are gated on `pm-devops-vendor-swagger-ui-unblock-api-server-cloud-build`.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 3/5** unchanged this run. Extended-scope epics (10B, 80, 81, 82, 83, 84, 85, 79, 8A, 9) folded into `coverage.json` and largely done.

| Epic | Sprint status | Coverage status (13 epics) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6/6 stories done in coverage |
| 7A — Basic Document Management | in-progress | 5/5 stories done in coverage |
| 8A — Basic Notification Preferences | done | 3/3 stories done |
| 10A — OAuth Provider Foundation | done | 3/3 stories done |
| 10B — Platform Administration | in-progress | 7/7 stories done |
| 80 — Dispute Resolution | partial | 3/3 stories done in coverage |
| 82 — Reality iOS SwiftUI | (extended) | 5/5 stories done in coverage; **re-checked this run (idx 7), no material change, last_checked=2026-09-08** |
| 84 — Documents / e-signature | (extended) | 3/5 done, 2 partial (84-1, 84-2) — unchanged |
| 81 / 83 / 85 / 79 / 7a / 8a / 9 | (extended) | all done in coverage |

## Shipped since last run (14 PRs merged 2026-09-01..2026-09-08)

- **#2922** — code-review reality-server saved-search: derive HTTP status from a typed error enum (pm-backend) — replaces string-copy comparisons; new limit/not-found regression tests.
- **#2925** — code-review api-handlers compliance raw DB leak regression: route `compliance::audit-log count` error through `db_error()` (pm-backend, security-relevant).
- **#2926 / #2927 / #2928 / #2931 / #2932** — code-review batch (per input list; not fully expanded here).
- **#2938** — gh-issue-2937: accounting-server invoice PDF renderer + handler test coverage (`get_invoice_pdf`) (pm-qa).
- **#2940** — code-review ppt-web: externalize `ConfirmationDialog` loading label via i18n (pm-frontend).
- **#2941** — code-review ppt-web `AuthContext`: validate JWT `exp` at cold boot with 30s skew; expired-with-refresh routes silent refresh, expired-without clears tokens (pm-frontend, security).
- **#2942** — code-review ppt-web `lib/api.ts`: single-flight refresh + one-shot 401 replay; concurrent 401s share one refresh promise (pm-frontend, security).
- **#2943** — code-review ppt-web: enforce logout purge allowlist coverage — test derives roots from `queryKeys`, `api-client`, and 4 hand-listed feature factories (pm-qa, security).
- **#2950** — gh-issue-2947 mobile: purge NFC access log / QR scan history / offline feedback drafts / pending feedback / faq_votes on session change; new `TENANT_SCOPED_EXACT_KEYS` registry (pm-frontend, security) — **merged red-CI due to #2951** (mobile jest infra), diff IG3-proven.
- **#2952** — gh-issue-2948 ppt-web: auto-discover feature-local `*Keys` factories in logout-purge coverage test via `import.meta.glob` (pm-tech-lead, security — closes the reviewer-memory gap that let PR #2650 miss `notification-triggers`).

## What's next (top 5 actions from ranked plan)

1. **[high] Land infra #2949 (vendor swagger-ui offline)** — unblocks api-server cloud verify and the entire IDOR trio — **owner: pm-devops**. New this run — highest-leverage single move; every other high-priority item is downstream.
2. **[high] Fix IDOR #2946** — apply `org_id` scope filter in `portfolio_analytics::upsert_property_metrics` + `get_property_metrics` (routes/portfolio_analytics.rs:282,314) — **owner: pm-security** (blocked on #1).
3. **[high] Fix IDOR #2945** — verify org ownership in portfolio_properties handlers — **owner: pm-security** (blocked on #1).
4. **[high] Fix IDOR #2944** — org-scope violations sub-resources; role-gate internal-notes — **owner: pm-security** (blocked on #1).
5. **[high] Sweep `_org_id` / `_tenant_id` compute-then-discard antipattern** across api-server; add a `just verify` grep gate — **owner: pm-security**. Independent of #1 (grep + docs, no build required).

## Blockers

- **NEW #2949 (api-server cloud build):** utoipa-swagger-ui build script downloads swagger-ui/main from GitHub — cloud proxy 403s. All 3 IDOR fixes stranded. Owner: pm-devops.
- **NEW #2951 (mobile jest infra):** jest-expo@56 ↔ react-native@0.87 version rot; mobile jest suite can't load, PR #2950 landed red-CI. Owner: pm-devops.
- **Standing #2652 (mobile-native/KMP):** AGP/Gradle egress in cloud sandbox — 6 backlog items unclaimable. Owner: pm-devops.
- **Standing RUSTSEC-2026-0258 (h2 empty-DATA-frame DoS):** waivered since 2026-08-18, no upstream patch tracked. Owner: pm-security.
- **Aging 84-1 + 84-2:** 5th consecutive upkeep window without dispatcher pickup; backend shipped, frontend slice pending. Owner: pm-frontend.

## Role focus today: **pm-security** (rotation idx 5; last 2026-07-21, 49d stale) + pm-scrum-master always-on

- **pm-scrum-master** (always-on): delivery mechanism is healthy for frontend/reality-server; api-server, mobile-rn (jest), and mobile-native (AGP) are cloud-verify-blocked by three separate infra issues. The two-day-old public IDOR bugs are the top delivery risk this run — not because the code is hard, because the pipeline can't ship it.
- **pm-security** (rotation): confirmed IDOR #2946 by direct code read (`let _org_id = user.tenant_id.ok_or_else(...)` on lines 282 and 314 of `portfolio_analytics.rs`, followed by a repo call that never passes `org_id`); flagged 3 more compute-then-discard sites (migration.rs:1472, faults.rs:642) as the same antipattern. Requested a `just verify` grep gate to prevent future reintroductions. See `roles/pm-security.md` for the full 6 next-actions and 5 risks.

## Coverage (upkeep this run — 2026-09-08)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated` bumped to 2026-09-08T00:00:00Z, no re-scan.
- **Epic re-check: epic-82** — cursor idx 7. All 5 stories (Reality iOS SwiftUI: 82-1..82-5) still `done`. No PR in the 2026-09-01..09-08 window touched SwiftUI screens; `last_checked = 2026-09-08` stamped on all five stories.
- **Merged-PR evidence:** none of the 14 merged PRs match a coverage story by keyword — all were correctness/security hardening of already-shipped surfaces. No status flips.
- **`coverage_cursor` advances 7 → 8** (epic-82 → epic-83 next run).
- **`pm_cursor` advances 5 → 6** (pm-security → pm-data next run). role_last_run["pm-security"] = 2026-09-08.
- **Composition unchanged: 47 done · 2 partial · 0 not-started** across 13 epics. **Missing UC links: 3** (UC-33.1/33.2/33.3 already queued). Zero orphan screens, zero validation errors.
