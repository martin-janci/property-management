# PPT Project State

_Generated: 2026-10-28 — routine Phase 1.6 incremental upkeep. Role focus: **pm-frontend** (83-day stale rotation slot; pm-cursor idx 2 → 3). Coverage `scan_kind=upkeep`; coverage_cursor idx 12 → 0 (epic-9 TOTP re-checked, story 9-1 still done, no PRs touched 2FA). pm-scrum-master always-on._

## Delta since last routine run (2026-10-26)

- **1 PR merged** (#3183) — **cross-tenant IDOR regression suite + 404 fix for violation mutations**.
  - `violations.rs` (routes + repo) hardened: cross-tenant mutations now 404 instead of 2xx/500; new regression suite `violations_mutation_cross_org_idor_tests` (14 cases, Org B Manager token on Org A seed) locks in the contract.
  - Does NOT map to a specific coverage story (violations is a sub-feature of epic-10b Platform Administration, which is already 7/7 done); no coverage-status flip.
  - Does NOT close the open `pm-backend-violations-add-evidence-authz-gap-3164` action — that's a role/authz gating gap on the `add_evidence` sub-route, distinct from cross-tenant 404 contract.
- **1 new critical issue: #3187** — `Failing test on dev blocks backend CI: cross_org_assign_violation_is_not_found (assign returns 403, expects 404)`. Surfaced by the test suite PR #3183 added — `assign` mutation still returns the role-gate 403 instead of 404 cross-tenant. Already queued as `gh-issue-3187` (high, owner pm-security). **Blocks backend CI on dev.**
- **6 open drafts** (#3184–#3189) — all dispatcher-authored, awaiting review/CI (gated on #3187 if backend-merge path).
- **Carried blockers unchanged**: #3168 (cargo-deny RUSTSEC advisories), #2652 (KMP cloud builds), #2951 (RN jest-expo), #3164 (violations add_evidence authz), #3131 (merge pipeline deadlock).

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** No status flips this run; `last_checked` refreshed on epic-9 (idx 12, TOTP 2FA — story 9-1 still done, nothing in the window touched 2FA surfaces).
- **1 PR merged** — a test-hardening + bug-fix on an already-shipped surface; good signal that the compliance/moderation surface is tightening but surfaced a brand-new blocker (`assign` cross-tenant 404 contract).
- **pm-frontend rotation came up 83 days stale** — last run 2026-08-06. The backlog has accumulated **12 open frontend/mobile code-review findings** in that window, including three "wire-the-button-but-not-the-POST" bugs (ReportFault setTimeout, reality-web /sell no-persist, VoteDetail ranked ballot) that reveal a chronic UI-pattern defect class. Six pm-frontend next_actions queued (three high, three medium); one new risk (`pm-frontend-mobile-rn-fake-submit-pattern-cluster-2026-10-28`, high/high) and one meta-risk on the rotation itself.
- **New blocker on dev CI**: #3187 fails cross-tenant-assign-404 — needs either the backend fix in `assign_violation` or the test's role shape adjusted to admit 403-when-manager-role-wrong-tenant.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 4/4** at sprint level (all four named sprint epics are status: done in sprint-status.yaml after the 2026-10-26 reconciliation). Coverage holds 13 epics all at done.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | done (6/6) | 6/6 done | — |
| 7A — Basic Document Management | done (5/5) | 5/5 done | — |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done | — |
| 10B — Platform Administration | done (7/7) | 7/7 done | #3183 (violations xtenant IDOR regression — moderation surface) |
| 80 — Dispute Resolution | done (3/3) | 3/3 done | — |
| 9 — 2FA/TOTP | done (1/1) | 1/1 done · **last_checked 2026-10-28 (cursor re-check)** | — |
| 79/82/83/84/85 (extended) | — | all done | — |

## Shipped since last run (1 PR merged)

- **#3183** — code-review-api-handlers-violations-no-xtenant-idor-tests: cross-tenant IDOR regression suite + 404 fix for violation mutations (merged 2026-10-27). Surfaced the new #3187 blocker as a side-effect — the new test caught `assign` returning 403 instead of 404 cross-tenant.

## Blockers (1 new critical, 5 carried)

- **#3187 (NEW, high)** — Failing test on dev blocks backend CI: `cross_org_assign_violation_is_not_found` (assign returns 403, expects 404). Owner: pm-security.
- **#3168** — cargo-deny RUSTSEC advisories gate every backend PR merge (carried, high). Owner: pm-security.
- **#3164** — violations `add_evidence` sub-resource ungated (carried, high). Owner: pm-backend.
- **#3131** — merge pipeline deadlocked (carried, medium). Owner: pm-backend.
- **#2652** — mobile-native/KMP cloud builds blocked (carried, high). Owner: pm-devops.
- **#2951** — mobile RN jest-expo version rot (carried, medium). Owner: pm-devops.

## What's next (top 5 from the roadmap)

1. **Fix #3187 cross-tenant assign-404 contract** (either backend `assign_violation` returns 404 on cross-tenant, or the test's role shape is adjusted) — owner: pm-security — **gates backend CI on dev**.
2. **Wire `ReportFaultScreen.tsx` handleSubmit to real `/api/v1/faults` POST** (currently `setTimeout(1500)` fake-submit — fault reports silently discarded) — owner: pm-frontend — user-visible data-loss bug.
3. **Wire reality-web `/sell` wizard Publish to reality-server POST** (every seller submission silently discarded today) — owner: pm-frontend — same pattern as #2, same user-impact.
4. **Audit ppt-web logout AUTHED_QUERY_KEY_ROOTS allow-list** (TanStack caches leak across users on a shared workstation) — owner: pm-frontend — tenant-isolation defect.
5. **Resolve #3168 cargo-deny RUSTSEC advisories** (carried) — owner: pm-security.

## Role focus today: pm-frontend (+ pm-scrum-master always-on)

- **pm-frontend** (83-day stale rotation): The dormant period masked a growing cluster of UI-layer defects (12 code-review backlog items; three "wire-but-no-POST" bugs; dead i18n fallbacks still landing 2+ months after the lint-gap risk was raised). Six next_actions queued (three high, three medium); one new high/high risk (`fake-submit-pattern-cluster`) and one meta-risk on the rotation cadence. The 2026-08-25 lint-gates decision (eslint-plugin-react-hooks + i18next/no-literal-string in `frontend/apps/mobile`) is now 64 days unresolved — queued as a `medium`-priority action.
- **pm-scrum-master**: Mild single-PR window (test-hardening) but the merge surfaced the new #3187 CI-blocker — so the window is **not** quiet. One next_action queued (`violations-xtenant-404-contract-sweep`) to turn #3187 into a one-shot pattern-fix across moderation.rs / aml_dsa.rs instead of per-mutation whack-a-mole.

## Coverage (upkeep this run — 2026-10-28)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated=2026-10-28T11:00:00Z`, no re-scan.
- **Epic re-check: epic-9 2FA/TOTP (cursor idx 12)** — story 9-1 still `done`; no merged PR in the 2026-10-27..2026-10-28 window touched TOTP/MFA surfaces. `last_checked = 2026-10-28` stamped.
- **Merged-PR evidence check:** #3183 touches `violations.rs` + repo (part of epic-10b/moderation); 10b stories all remain `done` — no evidence addition since no story tracks the violations sub-feature granularly.
- **`coverage_cursor` advances 12 → 0** (epic-9 → epic-10a next run).
- **`pm_cursor` advances 2 → 3** (pm-frontend → pm-qa next). role_last_run["pm-frontend"] = 2026-10-28.
- **Composition unchanged: 49 done · 0 partial · 0 not-started** across 13 epics. Missing UC links: 3 (UC-33.1/33.2/33.3, already queued elsewhere).
