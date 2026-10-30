# pm-qa — QA / Test lens (2026-10-30)

_Rotation idx 3 of 8. Read-only static analysis of sprint-status + open PRs + action-list + risks.json. Previous run: 2026-08-25 (66 days stale — overdue by ~36d past the ~30d-per-role cadence)._

## Role JSON

```json
{
  "role": "pm-qa",
  "summary": "66-day staleness coincides with a growing wave of auto-fix PRs (7 open `code-review-*` + 3 `gh-issue-*` + 1 `churn-hotspot` in the queue) and three carried QA risks still unaddressed (test-shadow debt, add_evidence pattern sweep, announcement-fanout SQL fidelity). The dispatcher's fix loop is landing patches faster than regression tests catch up — specifically #3188 (cross-tenant assign-404 fix for #3187) and #3192/#3193 (appeal re-filing + moderation report flood dedup) need failing-on-main tests committed in-PR before merge.",
  "next_actions": [
    {"action": "Attach failing-on-main cross-tenant assign-404 regression test to draft PR #3188 (gh-issue-3187) — reuse `violations_mutation_cross_org_idor_tests` scaffolding from PR #3183; the fix must not merge without the test file it fixes", "priority": "high", "dependency": "none", "definition_of_done": "PR #3188 contains a test under `backend/servers/api-server/tests/suites/` that fails on dev HEAD (403) and passes on the PR branch (404); CI diff-guard verifies the test file is in the same PR."},
    {"action": "Replace the pure-Rust re-model test for announcement cross-tenant fanout (#2484) with a sqlx integration test that exercises the real RLS predicate — unblocks carried risk `risk-announcement-fanout-test-fidelity-2026-07-23` (99 days aged)", "priority": "high", "dependency": "none", "definition_of_done": "`backend/servers/api-server/tests/suites/announcements_fanout_rls_tests.rs` exists, runs the actual INSERT via sqlx with Org A + Org B seeds, asserts fanout rows obey the RLS predicate; pure-Rust model test deleted."},
    {"action": "Co-own the e2e-form-submit smoke gate with pm-frontend (risk `pm-frontend-mobile-rn-fake-submit-pattern-cluster-2026-10-28`) — ratify MSW on mobile + Vitest+MSW on reality-web as the DoD; land one worked example (ReportFaultScreen) so the pattern is copy-paste for the remaining ~12 forms", "priority": "high", "dependency": "pm-frontend", "definition_of_done": "A `tests/e2e-smoke/` folder exists in `frontend/apps/mobile/` with a ReportFaultScreen test asserting the collected form payload is POSTed; `.github/workflows/frontend.yml` runs it; one PR template line documents the gate."},
    {"action": "Expand add_evidence-pattern sub-route cross-tenant regression coverage before PR #3171 merges (#3164) — mirror `community_cross_tenant_idor_tests.rs` across announcements/attachments, documents/signatures, mediation/evidence (same pattern as violations)", "priority": "medium", "dependency": "pm-backend", "definition_of_done": "A single integration test file enumerates every `/<resource>/<id>/evidence` POST and asserts cross-tenant → 404 for each; three new resources covered in addition to violations."},
    {"action": "Add failing-on-main test files to draft PRs #3192 (file_appeal status guard) and #3193 (moderation report flood dedup) before they enter the merge queue — currently both are code-only fixes with no test file", "priority": "medium", "dependency": "pm-backend", "definition_of_done": "Both PRs add a `*_regression_test.rs` file in the same commit that fails on dev HEAD; CI diff-guard rule applied."},
    {"action": "Draft a shared proptest corpus crate under `backend/crates/test-corpus/` for sanitizers (CSV, XSS, HTML, JSON) — carries over the 66-day-old fuzz-corpus decision from the 2026-08-25 run", "priority": "medium", "dependency": "pm-tech-lead", "definition_of_done": "Crate exists with CR/LF/CRLF/NUL/U+2028/U+2029 × {=, +, -, @, tab} × column-type combinations; CSV sanitizer tests consume it."}
  ],
  "risks": [
    {"risk": "Test-shadow debt is compounding: 7 open `code-review-*` fix PRs touch user-visible defects (invoice NaN, appeal re-file, moderation flood, auth-token stale, iot ws state) but only #3184 ships with regression tests — the dispatcher's fix loop is now outpacing the test-coverage loop and future regressions will land undetected", "probability": "high", "impact": "high", "mitigation": "Enforce the DEC-001 decision (every dispatcher-spawned fix PR must contain a failing-on-main test file) via CI diff-guard, not just reviewer discretion; block the merge queue on it. 66 days of soft-adoption did not stick."},
    {"risk": "The chronic add_evidence pattern (dispute #2483 → violations #3164, now a year apart in the same shape) is a leading indicator of a sub-route authz test-matrix gap, not an isolated bug. Without a one-shot cross-tenant sweep across every `/<resource>/<id>/<subresource>` POST, a third occurrence is a matter of time", "probability": "high", "impact": "high", "mitigation": "Build a parametric cross-tenant integration test that enumerates sub-routes from the OpenAPI spec and asserts 404 for every POST under Org B token on Org A seed; add to the backend CI gate."},
    {"risk": "7 of the 31 open PRs are 60+ days old (#2555/#2559 accounting trio, #2584–#2866 Dependabot) — the reviewer-starvation pattern pm-scrum-master flagged on 2026-07-30 has worsened; stale accounting PRs are not getting rebased against the live schema, so merge-conflict risk compounds every week they sit", "probability": "high", "impact": "medium", "mitigation": "Explicit reviewer-slot rotation for stale PRs; auto-rebase bot or close + respawn on PRs that cross 90 days; separate Dependabot batching so dep-bumps do not drown feature PRs in the queue."},
    {"risk": "No end-to-end integration test harness exists for spinning up >1 api-server replica against shared Postgres+Redis — the atomic-claim test called out 2026-08-25 is still unwritten, and the next quiet-hours drain concurrency regression (plus any future distributed-work-coordination bug) has nowhere to land", "probability": "medium", "impact": "high", "mitigation": "Spike a two-replica docker-compose test harness (reuse the Testcontainers Postgres fixture); land it with the quiet-hours concurrency test as the first consumer."},
    {"risk": "Mobile-native (KMP) tests remain structurally un-runnable in the cloud sandbox (#2652) — 4 KMP code-review items (CreateListingScreen stub, InquiriesResponse page_size, HttpClient no-timeout, pagination cap) have no regression-test attachment path from the dispatcher. QA for KMP is manual-only until #2652 clears", "probability": "high", "impact": "medium", "mitigation": "Prioritize #2652 unblock jointly with pm-devops; interim: pin a human-reviewer slot for KMP PRs and track the KMP test gap as its own backlog metric."}
  ],
  "open_questions": [
    "Is the DEC-001 'fix PR must contain a failing-on-main test' decision formally enforced via CI diff-guard today, or still advisory 66 days after the 2026-08-25 pm-qa run proposed it? If advisory, what blocks promotion to blocking?",
    "Does the OpenAPI spec expose a stable enumeration of `/<resource>/<id>/<subresource>` POST routes that a parametric cross-tenant test could consume, or does the sweep need to be hand-maintained?",
    "Does `frontend/apps/mobile/` have a Jest + MSW fixture today, or does the e2e-smoke gate need new fixture infra?",
    "Should the shared proptest corpus crate (CSV/XSS/HTML/JSON) sit under `backend/crates/` or be split per-language (also a frontend/packages/@ppt/test-corpus for TS sanitizers)?"
  ],
  "decisions_needed": [
    "Promote DEC-001 (every dispatcher fix PR requires an in-PR failing-on-main test file) from advisory to blocking via CI diff-guard — owner: pm-tech-lead + pm-qa",
    "Mandate parametric cross-tenant sub-route authz test driven by the OpenAPI spec — owner: pm-tech-lead + pm-backend",
    "Stale-PR policy: auto-close + respawn PRs older than 90 days (currently #2866 at ~103 days, accounting trio at ~70 days) — owner: pm-scrum-master + pm-tech-lead",
    "e2e-form-submit smoke gate rollout — blocking from day one, or warn-only for one sprint? — owner: pm-frontend + pm-qa"
  ]
}
```

## Notes

- Rotation idx 3 of 8; next pm-qa run ~ 2026-11-28 (if the ~30d/role cadence holds; see risk `pm-frontend-rotation-stale-83-days-2026-10-28` on cadence drift).
- Six pm-qa next_actions appended to `action-list.json` with `source = "pm-qa 2026-10-30"`; five risks appended to `risks.json` (including recarry of #3188 test-shadow pattern).
- Coverage upkeep this run (coverage_cursor idx 0 → 1): epic-10a OAuth Provider re-check — all 3 stories still `done`; no merged PR in the 2026-10-28..2026-10-30 window touched OAuth surfaces; `last_checked = 2026-10-30` stamped.
- Open-PR observations that shaped this run:
  - **#3188** (gh-issue-3187 fix) is a draft and the test it would need is exactly the one PR #3183 added — but the fix PR has no test file of its own yet. Classic test-shadow.
  - **#3171** (violations add_evidence authz for #3164) is a draft but lacks a cross-tenant regression test; same shape as the dispute #2483 → PR #2490 pattern a year ago.
  - **#3192 / #3193** (file_appeal status guard, moderation report flood dedup) are code-only fixes — no regression tests committed in-PR.
  - **#3184** (role-authz regression coverage expansion) is the one open PR that *is* a test file — the dispatcher's own regression work is landing but the fix PRs are not following suit.
  - **#3186** (iot-ws hasGivenUp state wired into dashboard) and **#3185** (neighbors stale-token useMemo fix) ship no component-level regression tests.
- Dependabot queue health: **14 open `chore(deps)`** PRs, 5 of them 60+ days old (#2584–#2866). This is a reviewer-starvation signal, not a QA signal per se, but it does mean security advisories (RUSTSEC) and transitive upgrades are routinely sitting ungated — pm-security + pm-devops both already carry risks that touch this.
- Carried QA-flavor risks still open after 66+ days:
  - `risk-announcement-fanout-test-fidelity-2026-07-23` (SQL vs re-model) — surfaced today's top-2 next_action.
  - `pm-qa-mobile-rn-lint-gap-hooks-i18n-2026-08-25` — now owned by pm-frontend (10-28); the lint-gates action is in-queue (`pm-frontend-mobile-rn-lint-gates-adopt-2026-10-28`).
- `coverage_cursor` advances 0 → 1 (epic-10a → epic-10b next run).
- `pm_cursor` advances 3 → 4 (pm-qa → pm-devops next). `role_last_run["pm-qa"] = 2026-10-30`.
