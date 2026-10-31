# PPT Project State

_Generated: 2026-10-31 — routine Phase 1.6 incremental upkeep. Role focus: **pm-qa** (67-day stale rotation slot; pm-cursor idx 3 → 4). Coverage `scan_kind=upkeep`; coverage_cursor idx 0 → 1 (epic-10a OAuth re-checked, all 3 stories still done, nothing in window touched OAuth surfaces). pm-scrum-master always-on._

## Delta since last routine run (2026-10-28)

- **8 PRs merged** (#3185, #3186, #3188, #3189, #3190, #3191, #3197, #3198) — a code-review batch landing simultaneously on 2026-10-30/31. Each PR shipped a test file beside the source fix (good hygiene). Themes: UI input validation (NaN/zero), stale-token hook deps, mobile ranked-ballot UX, OS-RNG panic on failure, cross-tenant 404 contract.
- **Blocker resolved**: issue **#3187** (`cross_org_assign_violation_is_not_found` dev-CI blocker) **closed by #3188** — assign_violation now runs the org-scope 404 check before the assignee role gate.
- **10 open code-review drafts** in flight (#3192-3201) — produced faster than reviewed; draft-rot risk if not triaged this sprint.
- **Carried blockers unchanged**: #3168 (cargo-deny RUSTSEC advisories), #2652 (KMP cloud builds), #2951 (RN jest-expo), #3164 (violations add_evidence authz, now gated for `fix` in #3171 under review), #2966 (swagger-ui egress).

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** No status flips this run; `last_checked` refreshed on epic-10a (OAuth Authorization Server + Client Registration + Token Management; nothing in merged window touched OAuth surfaces).
- **8 PRs merged** — a code-review implementation batch (one dispatcher cycle) ending the previous dev-CI blocker (#3187). Good test-with-fix hygiene across the batch; next check is whether the tests actually fail on the pre-fix code (two of them unverified).
- **pm-qa rotation came up 67 days stale** — last run 2026-08-25. Three next_actions added (one high, two medium); two new QA risks (numeric-input boundary gap cluster, tenant-isolation caught post-merge only).
- **Draft-PR backlog**: 10 open code-review drafts (#3192-3201) + 6 open dependabot PRs — reviewer-side pacing. Several dependabot PRs are stale (#3090 serde ~15 days, #2972 ktor ~40 days).

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 4/4** at sprint level. Coverage holds 13 epics all at done.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | done (6/6) | 6/6 done | — |
| 7A — Basic Document Management | done (5/5) | 5/5 done | — |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done · **last_checked 2026-10-31 (cursor re-check)** | — |
| 10B — Platform Administration | done (7/7) | 7/7 done | #3188 (violations xtenant 404 for assign) |
| 80 — Dispute Resolution | done (3/3) | 3/3 done | — |
| 9 — 2FA/TOTP | done (1/1) | 1/1 done | — |
| 79/82/83/84/85 (extended) | — | all done | #3185/#3186 (ppt-web core hooks), #3189/#3190/#3191 (frontend fixes), #3197/#3198 (reality fixes) |

## Shipped since last run (8 PRs merged)

- **#3185** — ppt-web neighbors hooks: read current auth token (useMemo deps fix).
- **#3186** — ppt-web IoT dashboard: wire WS give-up state into UI.
- **#3188** — fix(UC-violations): assign_violation returns org-scoped 404 before assignee gate (closes #3187).
- **#3189** — mobile-rn vote detail: ranked questions render as ordered ballot.
- **#3190** — ppt-web authed-roots: logout-purge coverage for personMonthKeys.
- **#3191** — ppt-web invoice form: guard qty/unitPrice parse against NaN.
- **#3197** — reality-web listing edit: reject empty/invalid price instead of persisting 0.
- **#3198** — reality-server SSO + password-reset: don't panic on OS RNG failure.

## pm-qa — 2026-10-31

Role-rotation deep-dive (67-day stale). Static read; `gh` GraphQL was blocked, so draft-PR ages and per-PR diff checks are unverified items in the next-actions.

### Test-coverage pulse (merged batch)

All 8 PRs shipped a test file with the fix. Spot-check needed: do #3191 (NaN), #3197 (price 0) and #3189 (ranked order) tests fail on pre-fix code? #3188 is the only security fix in the batch — it should carry a two-org integration test proving cross-tenant assign returns 404 before the assignee gate. #3198 (OS-RNG `expect` panic) is hard to test; stub-style coverage is plausible.

### CI/dev health

#3187 was a cross-tenant IDOR 404 contract gap that blocked dev CI for ~3 days. The test suite caught it on dev but **not pre-merge** — per-PR CI doesn't run the two-org integration seed. 6 open dependabot PRs include two aging numbers (#3090 serde, #2972 ktor) that will not merge-clean after this week's dev churn.

### Draft-PR backlog

10 open code-review drafts (#3192-3201) against 8 merged in the same window suggests drafts are produced faster than reviewed. Several drafts touch the same areas as the merged PRs (ppt-web core, mobile-rn); rebase conflicts likely.

### Regression surface theme

#3191 (invoice qty/price NaN), #3197 (listing price 0), #3189 (ranked ballot order) are all unvalidated numeric/ordering input at the UI boundary. #3198 and #3188 are the same gap on the server side (unchecked failure, authorization-order). Missing invariant: shared input parsers (`parseMoney` / `parsePositive`, order-preserving ranking) with property/boundary tests; each fix is one-off.

### Next actions

- **high** · `pm-qa-cross-tenant-ci-pre-merge-gate-2026-10-31` — verify #3188/#3191/#3197 tests fail on pre-fix code; add two-org IDOR integration test for violations + sibling assign endpoints as a required per-PR status check (not just the dev matrix).
- **medium** · `pm-qa-draft-pr-rot-triage-2026-10-31` — triage drafts #3192-3201 and aging dependabot (#3090, #2972); rebase or close.
- **medium** · `pm-qa-shared-numeric-input-helper-2026-10-31` — shared numeric-input helper with boundary tests (NaN/0/negative/empty) for ppt-web + reality-web; audit rent/fees/deposits forms.

### Risks (new)

- **medium/medium** · `pm-qa-numeric-input-boundary-gap-cluster-2026-10-31` — three one-off UI-input fixes in 7 days; other forms (rent/fees/deposits/meter readings) carry the same shape.
- **medium/high** · `pm-qa-tenant-isolation-caught-post-merge-only-2026-10-31` — two-org integration test lives only in dev matrix; next cross-org gap will again block dev rather than be rejected pre-merge.

## Dispatcher state

Dispatcher ticked at 2026-10-31T12:34Z (last chore commit): A=11 active, B=14/16, dep-blocked=2. Buffer-low payload (`claimable=14/72`) fired this routine run — planner refill expected. 2 ready plans remain for mobile-native-kmp (cloud-build-blocked by #2652).

## Carried blockers (unchanged)

- **#2966** (swagger-ui egress) — api-server unbuildable in cloud sandbox; every backend security PR structurally cloud-build-blocked.
- **#2652** (KMP cloud builds) — mobile-native-kmp tasks cloud-unclaimable; `plans/code-review-mobile-native-kmp-*` ready but structurally unclaimable.
- **#3168** (cargo-deny RUSTSEC advisories) — gates every backend merge.
- **#3164 → #3171** (violations add_evidence authz) — fix now under review (#3171).
