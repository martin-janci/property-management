# PPT Project State

_Generated: 2026-11-11 — routine Phase 1.6 incremental upkeep. Role focus: **pm-devops** (72-day stale rotation slot; pm-cursor idx 4 → 5). pm-scrum-master always-on. Coverage `scan_kind=upkeep`; coverage_cursor idx 1 → 2 (epic-10b re-check queued)._

## Delta since last routine run (2026-11-09)

- **37 PRs merged** in ~28h (#3371–#3409). The dispatcher cleared most of its code-review queue in one burst.
- **3 action-list `code-review-*` items moved to `done`** by this run's merges:
  - `code-review-reality-web-ssr-apibase-resolver-triplicated` ← PR #3407
  - `code-review-ppt-web-ui-portfolio-dashboard-dead-buttons` ← PR #3408
  - `code-review-api-core-rls-ctx-mw-dead-xtenant-only` ← PR #3409
- **Security-adjacent work shipped**: argon2 env-cost floor (#3394), cors policy shared across prod+test routers (#3400), sanitized DB error leaks in legal.rs (#3383), sanitized upstream SSO-callback leak (#3384), fault-list SQL injection + mobile/Reality fixes (#3395 "audit round 8"). All with CI green on `dev`.
- **1 issue closed**: #3397 ("last-admin guard's admin role_type list can drift from TenantRole") via PR #3405.
- **1 open PR trending**: #3410 (draft, `api-core` CORS localhost prod-default) — companion hardening to #3400; author-owned; 24h old.
- **Backlog nearly drained**: only 2 open + 2 ready non-triage items (all mobile-native-kmp), 0 open triage rows. No new vectors required this run.

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** `coverage_cursor` was at idx 1 (epic-10b "Platform Administration"); upkeep re-check this run — nothing in the 28h window touched platform-admin surfaces. Advances to idx 2.
- **CI is healthy** on `dev` for backend, frontend, and mobile-native workflows after the burst. No red workflows on `main`.
- **pm-devops rotation came up 72 days stale** — last run 2026-08-31. The Sept `utoipa-swagger-ui` block that stalled api-server cloud builds **has not re-triggered**; backend security PRs compiled and shipped. Mobile-native cloud-build starvation (issue #2652) is still structurally unresolved but the dispatcher pressure has eased dramatically.
- **Main remaining drag**: 4 PRs open 100+ days without a reviewer (#2555 invoice lifecycle, #2559 PAY by square QR, #2588/#2590 expo-57 dependabot now superseded). Three new action-list items force a decision.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 4/4** at sprint level · 49/49 stories done in coverage.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|------|---------------|-----------------|-------------------------|
| epic-6 Announcements | done | 6/6 done | — |
| epic-7a Documents | done | 5/5 done | #3376 (ui doc-search i18n), #3393 (folder-tree i18n) |
| epic-8a Notifications | done | 3/3 done | — |
| epic-10a OAuth Provider | done | 3/3 done | — |
| epic-10b Platform Admin | *in-progress coverage re-check* | — | — |

## Shipped since last run (summary, 37 PRs)

- **Security** (#3383, #3384, #3394, #3395, #3400, #3404, #3405): DB-error leak sanitization, SSO callback upstream-error sanitization, Argon2 env-cost floor, SQL-injection audit round 8, CORS policy single-sourced, IoT WS JWT moved from URL query to subprotocol, last-admin guard role-type derivation.
- **Frontend bugs / UX** (#3373, #3375, #3401, #3402, #3403, #3408, #3409): payment-match silent-fail surfaced, buildings error panel translated, vote time-remaining NaN guard, portfolio alert mutation errors surfaced, AI acknowledge errors surfaced, portfolio dashboard dead buttons wired/disabled, dead rls_context middleware removed.
- **Mobile RN fixes** (#3385, #3386, #3389, #3390, #3391, #3396, #3398, #3401): meter submit endpoint, neighbors screen i18n, voting i18n keys, NFCAccessController characterization tests, NFC overnight day boundary fix, feedback storage JSON.parse guard, push-isRegistered gating.
- **reality-web i18n sweep** (#3381, #3382, #3387, #3388): locale-aware date formatting, realtor CRUD error strings, realtor workspace agency i18n, report page i18n.
- **Infra / refactors** (#3371, #3372, #3377, #3378, #3379, #3392, #3406, #3407): import-jobs total count fix, realtor-api JSON array coercion, price_map mode validation, password-reset timing oracle hardening, OAuth-admin DB error sanitization, voting WS cache invalidation, resolved-layout endpoint caching, SSR api-base resolver centralized.

## What's next (top 5 actions)

1. **[high · pm-tech-lead]** Decide the mobile-native cloud-build policy — fix #2652, split queue, or close `pm-devops-unblock-mobile-native-cloud-builds` as won't-fix.
2. **[medium · pm-tech-lead]** Drive reviewer assignment on the 100+-day stale PRs (#2555 invoice lifecycle, #2559 PAY by square).
3. **[medium · pm-devops]** Land the nightly `verify-all.sh --quick` cron on `dev` so silent infra regressions surface without waiting for a PR (re-emit from 2026-08-31).
4. **[medium · pm-scrum-master]** Address issue #3357 (dispatcher pipeline stall + Tier-1d signal dead-end) — carried from 2026-11-09 pm-qa run.
5. **[low · pm-devops]** Close or rebase #2588 and #2590 (expo-57 dependabot superseded by the sprint's sibling PRs).

## Blockers

- None acute. The two carried blockers (#3357 dispatcher stall; #2966 utoipa-swagger-ui egress) are both quiet this run — the 37-PR merge burst suggests at least #2966's build-blocking effect is no longer active.

## Decisions needed

- **Mobile-native cloud-build policy** (owner: pm-tech-lead) — see roles/pm-devops.md.
- **Stale-PR eviction policy** (owner: pm-tech-lead) — adopt a 60-day no-review auto-close rule?
- Carried: **Dispatcher Tier-1d signal sourcing** (owner: pm-scrum-master) — issue #3357.

## Role focus this run

- `pm-scrum-master` (always-on): synthesis above.
- `pm-devops` (rotation idx 4 → 5): see `.research/management/roles/pm-devops.md`.
