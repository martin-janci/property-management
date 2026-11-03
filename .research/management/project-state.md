# PPT Project State

_Generated: 2026-11-03 — routine Phase 1.6 (rotating role: **pm-qa**, last-run 70d stale; always-on **pm-scrum-master**). Coverage `scan_kind=upkeep`; coverage_cursor idx 0 → 1 (epic-10a re-checked, no story-status flip)._

## Delta since last routine run (2026-11-02)

- **7 PRs merged** (#3216–#3222) — all dispatcher-cleared post-merge-review findings. Mix of security (#3219 `price_map.rs` LIKE-escape), i18n (#3218 EvidenceUploader, #3220 community, #3222 vote eligibility reasons), refactor/test (#3217 listing-edit numeric, #3221 snapshot decouple), and docs (#3216 screen-map reconcile).
- **1 new issue #3223** — follow-up from PR #3222 post-merge review: mobile `VoteDetailScreen` couples to backend English free-text; proposes stable reason codes or cross-boundary test as interim.
- **4 open drafts #3224–#3227** — dispatcher-opened against reality-web mock profile / feed silent-fail / inquiry-type i18n / violations `add_evidence` authz; all awaiting review.
- **Buffer dropped from 13 → claimable=8/72** — triggered the `buffer-low: claimable=8/72 — refill planner` payload that drove this run.
- **No coverage-status flips** — all 49 stories across 13 epics remain `done`.

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** Nothing shipped against an open coverage story this run; everything merged was a research-surfaced fix on already-done surfaces.
- **Planner buffer refilled** from 13 → 24 open items (pm-scrum-master + pm-qa contributed 11 new next_actions between them). Still below the 36 target — scrum-master flagged this as a structural risk now that coverage is drained.
- **Backend CI is still red on dev** (#3187 `cross_org_assign_violation_is_not_found`). Blocks every backend PR merge; carried from 2026-10-28.
- **pm-qa rotation at 70 days stale** — first run since 2026-08-25. Surfaced a cross-boundary test gap (#3223) and a post-merge test-coverage asymmetry (PR #3220 community i18n: 23 files / 3563 LOC with 1 i18n-key test).

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 4/4** at sprint level. Coverage holds 13 epics all at done.

## Shipped since last run (7 PRs merged)

- **#3216** screen-map reconcile `reality/account-listing-edit`
- **#3217** reality-web listing-edit numeric coercion (+`_number.ts`, 7 tests)
- **#3218** ppt-web EvidenceUploader i18n + 6-locale key-presence test
- **#3219** reality-server `price_map.rs` LIKE-metachar escape (security, with regression test)
- **#3220** ppt-web community-feature i18n (17 UI files + 6 locales + 1 test)
- **#3221** mobile `VoteDetailScreen` branch-test snapshot decouple
- **#3222** mobile `VoteDetailScreen` eligibility reason i18n + regression test — spawned follow-up #3223

## Blockers (1 new critical from prior run, 5 carried)

- **#3187 (high, from 2026-10-28)** — `cross_org_assign_violation_is_not_found` still red; blocks backend CI on dev. Owner: pm-security.
- **#3168** — cargo-deny RUSTSEC advisories (carried, high). Owner: pm-security.
- **#3164** — violations `add_evidence` authz gap (carried, high; PR #3227 open as a draft). Owner: pm-backend.
- **#3131** — merge pipeline deadlock (carried, medium). Owner: pm-backend.
- **#2652** — mobile-native/KMP cloud builds (carried, high). Owner: pm-devops.
- **#2951** — mobile RN jest-expo version rot (carried, medium). Owner: pm-devops.

## What's next (top 5 from the roadmap)

1. **Resolve #3187 cross-tenant assign-404 contract failure** — gate on backend CI going green on dev. Owner: pm-security.
2. **Decide stable eligibility-reason contract (issue #3223)** — enum/code vs free-text + cross-boundary test. Owner: pm-backend.
3. **Drive drafts #3224–#3227 through review to merge** — four dispatcher-authored fixes on silent-UX-failure surfaces. Owner: shared.
4. **Add cross-boundary test pinning Rust `vote.rs` eligibility strings to mobile `ELIGIBILITY_REASON_KEYS`** — interim mitigation for #3223 until the contract decision lands. Owner: pm-qa.
5. **Refill the planner buffer** — now that coverage is drained, source candidates from code-review findings + role next_actions. Target `open_count >= 36`.

## Role focus today: pm-qa (+ pm-scrum-master always-on)

- **pm-qa** (70-day stale rotation): Backend CI red while QA was dark. Surfaced 6 next_actions (2 high, 4 medium) and 4 risks — including a recurring one: rotation staleness is itself a defect-accumulation vector. Decision requested: hard staleness floor on Phase 1.6 (overriding quiet-day skip).
- **pm-scrum-master** (always-on): 7 merged PRs, buffer-drain pattern now structural (coverage at 49/49), four drafts queued, 1 new issue, 1 red CI carried.

## Buffer health

- `action-list.json`: **24/36 open** after this run (+11 added — pm-scrum-master 5, pm-qa 6; dedup + bare-role-token normalization applied).
- `claimable` at run-start: 8 of 72 (payload). Above 0 but far below steady-state. The coverage-drained condition means the refill engine has structurally changed — Phase 1.6 role `next_actions` and code-review findings are now the primary inflow.
