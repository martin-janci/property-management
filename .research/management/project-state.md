# PPT Project State

_Generated: 2026-10-31 — routine Phase 1.6 incremental upkeep. Role focus: **pm-qa** (67-day stale rotation slot; pm-cursor idx 3 → 4). Coverage `scan_kind=upkeep`; coverage_cursor epic-10a re-check (idx 0 → 1 next run). pm-scrum-master always-on._

> **BUFFER-LOW ALERT (dispatcher payload):** `claimable=17/72 — coverage exhausted: 0 non-done stories, backlog-refill avail=0, retry-remint avail=0`. Planner needs **new coverage authored**. Queued as the top high-priority pm-scrum-master action this run.

## Delta since last routine run (2026-10-28)

- **8 PRs merged** (#3185, #3186, #3188, #3189, #3190, #3191, #3197, #3198) — mostly code-review promotions landing on already-shipped surfaces. None flip a coverage story (49/49 remain `done`).
  - **#3188** (`gh-issue-3187 fix cross-org IDOR assign_violation`) **closes blocker #3187** that was fail-the-world-on-dev-CI. Backend CI un-blocked.
  - **#3190** (`pm-frontend-ppt-web-authed-roots-session-leak-2026-10-28`) resolves the TanStack cache-leak audit raised in the 2026-10-28 pm-frontend rotation.
  - Code-review promotions span 4 surfaces: ppt-web (×4: neighbors, iot ws, invoice NaN, authed-roots), reality-web (×1: listing-edit price), reality-server (×1: OS RNG), mobile-rn (×1: vote-detail ranked), backend/security (×1: #3188).
- **1 issue closed**: #3187 (closed by PR #3188 — dev CI unblocked).
- **14 open PRs** — 10 dependabot (likely gated by #3168 cargo-deny), 4 non-dependabot drafts (#3171, #3075, #3108, #3192, #3193) — long-lived; need triage decisions.
- **Carried blockers unchanged**: #3168 cargo-deny RUSTSEC advisories, #2652 KMP cloud builds, #2951 RN jest-expo, #3164 violations add_evidence authz, #3131 merge pipeline deadlock.

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** No status flips this run; `last_checked` refreshed on epic-10a (idx 0, OAuth provider — stories 10a-1/10a-2/10a-3 still done, nothing in the merge window touched OAuth surfaces).
- **8 PRs merged, 1 CI-blocking issue cleared** — heavy, productive window. BUT this is the planner's problem: all 8 PRs are code-review promotions on already-shipped surfaces (bug-class fixes), and coverage has no non-done stories left. The dispatcher's claimable queue is now at **17/72**, with every refill-feeder (backlog, retry-remint) also at zero. **The pipeline is running on hotspot/churn candidates alone**.
- **pm-qa rotation came up 67 days stale** — last run 2026-08-25. In that gap the system shipped 8 code-review bug-class fixes without a cross-cutting regression-test audit. QA raises **three high-impact concerns**: (1) IG3-failing-on-main-test gate was not consistently enforced on the 8 PRs; (2) same defect pattern-classes (cross-tenant 403-vs-404, unchecked numeric parse, WS state, authed-cache leak) likely repeat elsewhere → pattern-sweep needed; (3) #2951 jest-expo blocker is 90+ days unresolved and silently disables mobile RN tests on every PR.
- **No new blocker surfaced this run.** The #3187 blocker from the previous window was cleared by #3188.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 4/4** at sprint level. Coverage holds 13 epics all at done.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | done (6/6) | 6/6 done | — |
| 7A — Basic Document Management | done (5/5) | 5/5 done | — |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done · **last_checked 2026-10-31 (cursor re-check)** | — |
| 10B — Platform Administration | done (7/7) | 7/7 done | #3188 (cross-org IDOR assign_violation 404) |
| 80 — Dispute Resolution | done (3/3) | 3/3 done | — |
| 9 — 2FA/TOTP | done (1/1) | 1/1 done | — |
| 79/82/83/84/85 (extended) | — | all done | — |

## Shipped since last run (8 PRs merged · 1 issue closed)

- **#3185** — code-review-ppt-web-core-neighbors-stale-token-usememo (frontend / ppt-web)
- **#3186** — code-review-ppt-web-core-iot-ws-hasgivenup-dropped (frontend / ppt-web)
- **#3188** — gh-issue-3187 fix cross-org IDOR assign_violation (backend / security) **→ closes #3187, unblocks dev CI**
- **#3189** — code-review-mobile-rn-vote-detail-ranked-no-order (mobile-rn)
- **#3190** — pm-frontend-ppt-web-authed-roots-session-leak-2026-10-28 (frontend / ppt-web)
- **#3191** — code-review-ppt-web-ui-invoice-qty-price-nan (frontend / ppt-web)
- **#3197** — code-review-reality-web-listing-edit-price-zero (frontend / reality-web)
- **#3198** — code-review-reality-server-os-rng-expect-panics (backend / reality-server)

Issues closed: **#3187** (closed by #3188).

## Blockers (0 new · 5 carried · 1 planner-level)

- **PLANNER (NEW, high)** — **Coverage exhausted**: dispatcher claimable=17/72, backlog-refill avail=0, retry-remint avail=0. Needs new coverage authored. Owner: **pm-tech-lead** (+ pm-scrum-master).
- **#3168** — cargo-deny RUSTSEC advisories gate every backend PR merge (carried, high). Owner: pm-security.
- **#3164** — violations `add_evidence` sub-resource ungated (carried, high). Owner: pm-backend.
- **#3131** — merge pipeline deadlocked (carried, medium). Owner: pm-backend.
- **#2652** — mobile-native/KMP cloud builds blocked (carried, high). Owner: pm-devops.
- **#2951** — mobile RN jest-expo version rot (carried, medium). Owner: pm-devops. (QA calls this out as release-gate-silent — tests disabled.)

## What's next (top 5 from the roadmap)

1. **Author new coverage** (phase-2/3 epic enumeration into `coverage.json`) — owner: **pm-scrum-master + pm-tech-lead** — **unblocks the planner**; dispatcher is running on hotspot/churn candidates alone until this lands.
2. **Regression-test audit on the 8 merged PRs this window** — confirm every PR has a failing-on-main test (IG3 gate) — owner: **pm-qa** — guards against the ship-and-pray risk the heavy merge window hides.
3. **Cross-tenant IDOR regression harness** — extend PR #3183's pattern to moderation.rs / aml_dsa.rs / add_evidence sub-routes / announcement+document attachment mutations — owner: **pm-qa + pm-backend** — one-shot pattern coverage.
4. **Resolve #3168 cargo-deny RUSTSEC advisories** (carried) — owner: pm-security.
5. **Triage the 4 non-dependabot drafts** (#3171 / #3075 / #3108 / #3192 / #3193) — close, revive, or convert each to a plan — owner: pm-scrum-master.

## Role focus today: pm-qa (+ pm-scrum-master always-on)

- **pm-qa** (67-day stale rotation): First QA pass since 2026-08-25. Six next_actions queued (two high, three medium, one low) + three risks + two decisions_needed. Headline: coverage exhaustion is masking regression debt — "0 open stories" looks green while 8 bug-class fixes shipped in 3 days without a cross-cutting regression suite. QA proposes mandating the IG3-failing-on-main-test gate on code-review PRs, and promoting the regression-test audit to a numbered epic in coverage so pm-qa work shows up in the planner (today it does not — hence the stale rotation).
- **pm-scrum-master**: 8-PR window is NOT quiet, but the real signal is the **buffer-low** state — the dispatcher payload says `claimable=17/72` with all refill-feeders at zero. Three next_actions queued (one high, two medium); one high/medium risk on candidate-quality degradation under hotspot-only feed. Decisions needed: coverage refill scope (pm-tech-lead), and whether to batch-merge the 10 open dependabot PRs under the current cargo-deny #3168 state (pm-devops).

## Coverage (upkeep this run — 2026-10-31)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated=2026-10-31T12:00:00Z`, no re-scan.
- **Epic re-check: epic-10a OAuth Provider Foundation (cursor idx 0)** — stories 10a-1/10a-2/10a-3 still `done`; no merged PR in the 2026-10-29..2026-10-31 window touched OAuth provider surfaces. `last_checked = 2026-10-31` stamped.
- **Merged-PR evidence check**: #3188 touches `violations.rs` + repo (part of epic-10b/moderation); 10b stories all remain `done`. No story tracks the violations sub-feature granularly. All other 7 PRs are UI-layer bug fixes on already-done story surfaces.
- **`coverage_cursor` advances 0 → 1** (epic-10a → epic-10b next run).
- **`pm_cursor` advances 3 → 4** (pm-qa → pm-devops next). role_last_run["pm-qa"] = 2026-10-31.
- **Composition unchanged: 49 done · 0 partial · 0 not-started** across 13 epics. Missing UC links: 3 (UC-33.1/33.2/33.3, already queued elsewhere).
- **BUFFER STATE** (dispatcher-reported this run): `claimable=17/72, backlog-refill avail=0, retry-remint avail=0`. **Coverage is exhausted as a candidate source.** New coverage authoring is the only non-degenerate refill path.
