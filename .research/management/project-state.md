# PPT Project State

_Generated: 2026-11-13 — routine Phase 1.6 incremental upkeep. Role focus: **pm-devops** (73-day stale rotation slot; pm-cursor idx 4 → 5). pm-scrum-master always-on. Coverage `scan_kind=upkeep`; coverage_cursor idx 1 → 2 (epic-10b re-checked, 7/7 done; no OAuth/platform-admin surface changes this window)._

## Delta since last routine run (2026-11-09)

- **58 PRs merged** in ~4 days — the dispatcher pipeline has gone from "stalled 10+ runs" to a sustained burst. Composition: ~47 `code-review-*` findings landed (ppt-web-ui, ppt-web-core, reality-server, reality-web, mobile-rn, api-core) + 3 `gh-issue-*` follow-ups.
- **3 follow-up issues closed** (#3466 ppt-web de/pl/hu locale drift, #3431 reality-server branding COALESCE-no-clear, #3425 PortfolioDashboardPage error translation). One (#3397 last-admin guard) also marked resolved.
- **1 new security issue open**: #3427 — Harden first-party JWT: emit + validate `iss`/`aud` across api-server & reality-server (labels: `security`, `follow-up`, `blocked`). Operator triage needed.
- **9 Dependabot PRs open** across backend + frontend. Majors stacking: #3447 vitest 4→5, #3444 jsonwebtoken 10→11, #3443 utoipa-swagger-ui 9→10.
- **3 dispatcher drafts in flight**: #3475, #3476, #3477 (all `code-review-*`, auth/MFA/logout cache flavors — all updated 2026-11-13 by the dispatcher).
- **Routine lag: 4 days** (last_run_iso 2026-11-09 vs today 2026-11-13). Cloud routine scheduler has drifted silently while the dispatcher kept running — new pm-devops watch item and risk.
- **2 action-list items resolved**: `code-review-mobile-rn-features-faults-orphaned` (PR #3473), `code-review-mobile-rn-person-months-no-i18n` (PR #3474). `pm-scrum-master-refill-non-kmp-buffer-2026-11-09` also closed (refill self-resolved via burst).

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** No status flips this run; `last_checked` refreshed on epic-10b (idx 1, Platform Administration — 7/7 stories done, nothing touched).
- **Dispatcher pipeline health: fully recovered.** 10+ runs of 0-claimed/0-merged (per 2026-11-09 brief) → 58 PRs merged in the subsequent 4-day window. The stall described in issue #3357 has resolved itself, but the underlying failure mode (dispatcher cannot self-heal from blocked PRs) is unchanged.
- **Routine scheduler drift is the new concern.** The research routine lagged 4 days behind the dispatcher — surfacing this as its own risk + action.
- **Major-version Dependabot stack is thickening.** 3 cross-stack majors open — vitest, jsonwebtoken, utoipa-swagger-ui — need human review within the week.
- **pm-devops rotation came up 73 days stale** (last run 2026-08-31). Three fresh findings: routine scheduler drift, Dependabot major-review window, and the long-standing mobile-native-kmp cloud starvation (carried; issue #2652 CLOSED but gap persists).

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 4/4** at sprint level · 49/49 stories done in coverage.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | done (6/6) | 6/6 done | — |
| 7A — Basic Document Management | done (5/5) | 5/5 done | — |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done | — |
| 10B — Platform Administration | done (7/7) | 7/7 done · **last_checked 2026-11-13 (cursor re-check)** | — |
| 80 — Dispute Resolution | done (3/3) | 3/3 done | — |

## Shipped since last run (top of 58 PRs merged)

- **#3473** — code-review-mobile-rn-features-faults-orphaned: remove orphaned features/faults dead code. Closes action-list `code-review-mobile-rn-features-faults-orphaned`.
- **#3474** — code-review-mobile-rn-person-months-no-i18n: route MoreMenu person-months label through i18n. Closes action-list `code-review-mobile-rn-person-months-no-i18n`.
- **#3472** — gh-issue-3466: complete criticalNotifications + outages.errors i18n for de/pl/hu.
- **#3471** — code-review-ppt-web-ui-toast-persistent-overflow-leak: bound ppt-web toast queue (persistent-toast memory leak fix).
- **#3454** — code-review-mobile-rn-notifications-still-mock: wire RN notifications to real inbox API (eliminates fabricated-data section on the mobile RN app).
- **#3463** — code-review-ppt-web-ui-2fa-qr-silent-fail: surface 2FA QR render failure instead of swallowing it.
- **#3451** — code-review-mobile-rn-profile-local-only-save: persist mobile profile edits to the api-server.
- **#3424** — code-review-reality-server-favorites-limit-toctou: make favorites cap check+insert atomic (correctness fix).
- **#3410** — code-review-api-core-cors-localhost-prod-default: don't trust localhost CORS origins in production by default (security hardening).
- **#3416** — code-review-api-core-pushfanout-queue-drops-transient: retry transient push-fanout failures instead of dropping.

(Full list: see .research/briefs/2026-11-13.md Shipped section.)

## What's next (top 5 from the ranked plan)

1. **[high] pm-scrum-master** — Triage open security issue #3427 (`Harden first-party JWT: emit + validate iss/aud across api-server & reality-server`). Resolve the `blocked` label: real dependency or awaiting prioritisation? Promote to a plan or defer with justification.
2. **[high] pm-devops** — Unblock mobile-native/KMP builds in the cloud runner (long-standing structural starvation; issue #2652 CLOSED but gap persists). 5+ backlog rows remain permanently unclaimable in cloud without this.
3. **[medium] pm-devops** — Review + land the 3 major Dependabot bumps sitting since 2026-11-12 (vitest 4→5, jsonwebtoken 10→11, utoipa-swagger-ui 9→10). Minor-group PRs can auto-merge after; majors need a human read.
4. **[medium] pm-devops** — Investigate the 4-day routine lag (last_run_iso=2026-11-09 vs today 2026-11-13). Check cloud routine scheduler for silent drift and add a lag-floor check that fails the next run loudly instead of catching up.
5. **[medium] pm-qa** — Confirm the 3 dispatcher drafts #3475/#3476/#3477 carry regression tests before they un-draft and auto-merge (continuing pm-qa's standing test-shadow discipline check).

## Blockers

- **mobile-native-kmp cloud-landability starvation** (carries indefinitely until pm-devops unblocks local-toolchain-in-cloud; issue #2652 CLOSED but structural gap unchanged — 5+ backlog items permanently unclaimable in cloud).
- **routine scheduler drift** (4-day gap with no error surfaced; may silently compound).

## Decisions needed

- **Lag-floor policy for the daily research routine** — should the routine hard-fail (or emit a dispatcher-visible alert) when `(now − last_run_iso) > 48h`, rather than silently catching up? Owner: pm-devops.
- **Dependabot major-review SLA** — define a WIP cap + review window (e.g. 72h for majors) so cross-stack majors don't stack. Owner: pm-devops.
- **#3357 dispatcher-stall operator action: now actually unneeded?** — the 58-PR burst has absorbed the backlog that triggered #3357. Owner: pm-scrum-master. (Likely closes as "self-resolved".)

## Role summaries (this run)

- **pm-scrum-master** — Dispatcher pipeline went from stalled to productive (58 PRs merged). Backlog refill ask resolved itself. One new open security issue (#3427) needs triage. Routine scheduler lag is the new concern. No status flips in coverage; all 4 sprint epics stay `done`.
- **pm-devops** — 33 workflows; no CI churn this window. Three fresh asks: investigate 4-day routine-scheduler lag; review the 3 stacked major Dependabot bumps; keep the mobile-native-kmp cloud build gap on the radar (issue #2652 CLOSED but gap unchanged). No deploy/rollback events since last run.
