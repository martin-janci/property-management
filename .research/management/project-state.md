# PPT Project State

_Generated: 2026-11-03 — routine Phase 1.6 incremental upkeep. Role focus: **pm-qa** (70-day stale rotation slot; pm-cursor idx 3 → 4). Coverage `scan_kind=upkeep`; coverage_cursor idx 0 → 1 (epic-10a re-checked, 3/3 still done, no PRs touched OAuth surfaces). pm-scrum-master always-on._

## Delta since last routine run (2026-11-02)

- **5 PRs merged** (#3216 – #3220) — all quality-track:
  - **#3216** reconciled `docs/screens/reality/account-listing-edit.md` with PR #3212 (docs/screens-only; pm-qa task).
  - **#3217** extracted the silent-zero numeric coercion in `reality-web` listing-edit into a tested `_number` helper (closes a pattern-sibling of the #3197 price-zero bug).
  - **#3218** internationalized `ppt-web` `EvidenceUploader` validate errors + drop-zone + placeholder across 6 locales.
  - **#3219** escaped LIKE metacharacters in reality-server `price_map.rs` `city` filter (public unauthenticated endpoint hardening).
  - **#3220** react-i18next across every `ppt-web` community component + 323 leaf keys × 6 locales.
- All 5 PRs shipped a two-commit `test:`/`fix:` TDD split with explicit fails-before-fix proof. No new critical issues created this run.
- Carried blockers unchanged: **#3187** cross-tenant assign-404 (still red on dev), **#3168** cargo-deny RUSTSEC, **#3164** violations add_evidence, **#3131** merge deadlock, **#2652** KMP cloud, **#2951** jest-expo.
- Open drafts touched this window: **#3221, #3222, #3193** — awaiting review (backend-path gated on #3187).

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** No status flips; `last_checked` refreshed on epic-10a (idx 0 — OAuth surfaces unchanged this window).
- **Quality signal is up**: five merged PRs this run shipped with explicit IG3 TDD evidence (test fails before fix, test green after fix). Discipline is landing.
- **CI gating signal is down**: `backend.yml` on `dev` is still red from last run's #3187 (`cross_org_assign_violation_is_not_found` expects 404, gets 403) compounded by #3168 cargo-deny. Every backend PR is blocked.
- **pm-qa rotation ran today (70 days stale)**. Six next_actions queued (three high, three medium/low) focused on the dev-red CI contract decision, under-asserting mobile-rn voting tests, and the carried announcement fan-out RLS integration-test gap. Two new risks (`votedetail-churn-under-asserting`, `dev-ci-red-broken-windows`) + one meta-risk from pm-scrum-master on rotation skew.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 4/4** at sprint level. Coverage holds 13 epics all at done.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | done (6/6) | 6/6 done | — |
| 7A — Basic Document Management | done (5/5) | 5/5 done | — |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done · **last_checked 2026-11-03 (cursor re-check)** | — |
| 10B — Platform Administration | done (7/7) | 7/7 done | — |
| 80 — Dispute Resolution | done (3/3) | 3/3 done | — |
| 79/82/83/84/85/9 (extended) | — | all done | #3217/#3219 touch reality-web / reality-server surfaces (not epic-scoped); #3218/#3220 touch ppt-web i18n (not epic-scoped) |

## Shipped since last run (5 PRs merged)

- **#3216** — screen-map-drift-pr-3212-reality: reconcile account-listing-edit screen-map (docs/screens-only)
- **#3217** — repeated-churn reality-web listing-edit: extract `_number` numeric-coercion helper + tests (closes silent-zero sibling of #3197)
- **#3218** — ppt-web EvidenceUploader i18n: validate errors + drop-zone + placeholder across 6 locales + evidenceUploaderI18n.test.ts
- **#3219** — reality-server price_map.rs: escape LIKE metacharacters in `city` filter + regression test (public unauth endpoint hardening)
- **#3220** — ppt-web community feature i18n: react-i18next across every component + 323 leaf keys × 6 locales + communityI18n.test.ts

## Blockers (6 carried, 0 new)

- **#3187** — dev `backend.yml` red: `cross_org_assign_violation_is_not_found` expects 404, gets 403. Owner: pm-security.
- **#3168** — cargo-deny RUSTSEC advisories gate every backend PR merge. Owner: pm-security.
- **#3164** — violations `add_evidence` sub-resource ungated. Owner: pm-backend.
- **#3131** — merge-pipeline deadlocked. Owner: pm-backend.
- **#2652** — mobile-native/KMP cloud builds blocked. Owner: pm-devops.
- **#2951** — mobile RN jest-expo version rot. Owner: pm-devops.

## What's next (top 5)

1. **Decide the cross-tenant 404-vs-role-403 contract for violations.assign** and fix either `assign_violation` (return 404 cross-tenant) OR the regression test's role shape — unblocks `backend.yml` on dev (#3187) — owner: **pm-security** (pm-qa validates).
2. **Resolve #3168 cargo-deny RUSTSEC advisories** (bump transitive crate or bounded exception with deadline captured in action-list) — unblocks every backend PR merge — owner: **pm-security**.
3. **Gate violations add_evidence sub-resource (#3164)** with tenant-scoped authz + cross-tenant integration test on the `community_cross_tenant_idor_tests.rs` pattern — owner: **pm-backend**.
4. **Add VoteDetailScreen.test.tsx cast-POST contract assertion** — close the ranked-ballot regression class driving the 2nd consecutive churn run — owner: **pm-qa + pm-frontend**.
5. **Adopt per-screen MSW/Mirage Submit-POST gate** (mobile + reality-web) — closes the chronic fake-submit-pattern cluster — owner: **pm-qa + pm-frontend**.

## Role focus today: pm-qa (+ pm-scrum-master always-on)

- **pm-qa** (70-day stale rotation): Quality signal is strong this window — all 5 merged PRs shipped IG3 TDD evidence — but the dominant release-blocker (#3187) is a test-vs-code contract gap, which is pm-qa territory. Six next_actions queued (three high, three medium/low): (1) violations.assign contract decision, (2) announcement fan-out RLS integration test (carried #2484), (3) VoteDetailScreen cast-POST contract assertion, (4) un-quarantine dispute KPIs test + window validation, (5) MSW Submit-POST gate, (6) sprint-status.yaml drift reconcile vs coverage.json. Two new risks (`votedetail-churn-under-asserting`, `dev-ci-red-broken-windows`).
- **pm-scrum-master**: Productive window (5 PRs merged, all quality-track) but the dominant blockers (#3187, #3168) remain. Two next_actions queued (`rustsec-3168-resolve`, `staleness-bias-pm-cursor`) and one new risk (`rotation-skew-backend-heavy`).

## Coverage (upkeep this run — 2026-11-03)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated=2026-11-03`, no re-scan.
- **Epic re-check: epic-10a OAuth Provider Foundation (cursor idx 0)** — all 3 stories still `done`; no merged PR in the 2026-11-02..2026-11-03 window touched OAuth surfaces (the 5 merged PRs targeted reality-web, reality-server/price_map, ppt-web i18n, and docs/screens — none touched `oauth.rs` or OAuth screens). `last_checked = 2026-11-03` stamped.
- **Merged-PR evidence check:** no PR this run maps to a coverage story — all 5 were quality-track touches on already-shipped surfaces. No coverage status flips.
- **`coverage_cursor` advances 0 → 1** (epic-10a → epic-10b next run).
- **`pm_cursor` advances 3 → 4** (pm-qa → pm-devops next). role_last_run["pm-qa"] = 2026-11-03.
- **Composition unchanged: 49 done · 0 partial · 0 not-started** across 13 epics. Missing UC links: 3 (UC-33.1/33.2/33.3, already queued elsewhere).
