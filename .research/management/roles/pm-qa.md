# pm-qa — QA / Test lens (2026-11-07)

_Rotation idx 3 of 8. Read-only static analysis of sprint-status + merged PRs + open action-list + planner buffer. Previous run: 2026-08-25 (74 days stale)._

## Role JSON

```json
{
  "role": "pm-qa",
  "summary": "2 PRs merged this window (#3350 reality-web share-cmp schema validation [has test], #3351 ppt-web registry i18n [no test]) — the 1-of-2 test-coverage ratio is the same test-shadow pattern pm-qa flagged on 2026-08-25, now 74 days unresolved with no CI diff-guard in place. Planner buffer at 6/72 claimable (all open non-dep-blocked work mobile-native-gated on #2652) is a release-visible QA risk: cannot regression-test mobile-native paths that cannot be built in CI.",
  "next_actions": [
    {"action": "Backfill Vitest i18n regression test for the PR #3351 ppt-web registry pet/vehicle components — assert each useTranslation key resolves across all 6 locales and no hardcoded English remains in the 4 touched files (closes the test-shadow debt on #3351)", "priority": "medium", "dependency": "pm-frontend", "definition_of_done": "Vitest test covers the 4 registry files; assertion: no bare English string literal in the rendered output when locale=cs/sk/de/en/pl/hu; all t() keys resolve (no [missing] markers)."},
    {"action": "Escalate planner buffer-low (6/72 claimable, mobile-native-gated) as a release-readiness risk — publish a QA-visible release-blocker view distinct from the planner's buffer-health view so merge-gate decisions see it", "priority": "high", "dependency": "pm-devops", "definition_of_done": "release-readiness.md section added under .research/management/ enumerating the mobile-native-gated items as QA-blocked (not merely planner-blocked); #2652 named as the single release-gate."},
    {"action": "Confirm #3187 cross-tenant assign-404 contract test status on dev CI — 10 days since surfacing (2026-10-28), still listed as carried blocker; verify whether backend assign_violation fix landed or test was adjusted, close/escalate", "priority": "high", "dependency": "pm-security", "definition_of_done": "gh pr list / gh issue view #3187 confirms status; if still red, open issue cross-linking the existing pm-scrum-master-violations-xtenant-404-contract-sweep action; if green, mark closed in action-list."},
    {"action": "Convert the 2026-08-25 decision proposal (CI diff-guard requiring failing-on-main test for dispatcher-spawned fix PRs) into a concrete action — draft the CI check rule, name the enforcement workflow, quantify false-positive rate on last-30-days merges", "priority": "medium", "dependency": "pm-tech-lead", "definition_of_done": "Proposal PR targets .github/workflows/ with the diff-guard check; a 30-day retrospective CSV lists PRs that would have been blocked; pm-tech-lead + pm-qa sign-off recorded in decisions.md."},
    {"action": "Replace pure-Rust announcement fan-out predicate re-model test (#2484) with a sqlx integration test exercising the real RLS policy — carried risk pm-qa-announcement-fanout-test-fidelity-2026-07-23, now 107 days unresolved while the announcements surface churns", "priority": "medium", "dependency": "pm-backend", "definition_of_done": "Integration test with multi-tenant seed, Org A announcement targeting building/units/roles, Org B tokens attempt read; asserts RLS predicate blocks visibility; replaces the pure-Rust test file."},
    {"action": "Audit the last 30 days of dispatcher-spawned PRs for the fake-submit UI pattern (handleSubmit → setTimeout → no POST) that pm-frontend flagged on 2026-10-28 (ReportFault, reality-web /sell, VoteDetail) — expand the search to all PRs touching *Screen.tsx / *Page.tsx submit handlers", "priority": "medium", "dependency": "pm-frontend", "definition_of_done": "Grep report + hit list posted to roadmap.md or a separate audit file; each hit converted to an action-list row; pattern added to pm-qa's recurring static-check list."}
  ],
  "risks": [
    {"risk": "Test-shadow pattern persists: PR #3351 landed WITHOUT a regression test (same class pm-qa flagged 2026-08-25). The 1-of-2 ratio this window (#3350 has a test, #3351 doesn't) mirrors the 2-of-13 ratio from the 2026-08-25 window — enforcement decision still unformalized 74 days later.", "probability": "high", "impact": "medium", "mitigation": "Formalize the 2026-08-25 decision into a CI diff-guard on frontend.yml / backend.yml — dispatcher-spawned fix PRs require a failing-on-main test file in the same PR."},
    {"risk": "Planner buffer at 6/72 claimable — all remaining open non-dep-blocked work is mobile-native-gated on #2652. The dispatcher's 3-implementer pool (12 runs/day ≈ 36 slots) will starve while the mobile-native queue is structurally unclaimable; QA cannot regression-test mobile-native paths that cannot be built in CI.", "probability": "high", "impact": "high", "mitigation": "Pm-scrum-master requested off-rotation Tier-1b backlog scan scoped to backend+frontend+reality vectors (parallel queue); pm-devops to break #2652 on the parallel track."},
    {"risk": "#3187 cross-tenant assign-404 contract still carried as a blocker 10 days after surfacing (2026-10-28). The pm-scrum-master-violations-xtenant-404-contract-sweep action is still `open` with depends_on=['UNRESOLVED:pm-security'] — poisoned sentinel means it is non-claimable. Dev CI remains at risk of being red on that test.", "probability": "medium", "impact": "high", "mitigation": "Resolve the UNRESOLVED:pm-security sentinel to a concrete action-list id or escalate to pm-security directly; confirm dev CI status on the next routine run before moving on."},
    {"risk": "Fake-submit UI pattern (handleSubmit → setTimeout → no POST) flagged by pm-frontend on 2026-10-28 for three screens (ReportFault, reality-web /sell, VoteDetail) has no cross-cutting audit — more instances likely lurk, and the lint-gates adoption action is stalled on UNRESOLVED:pm-qa.", "probability": "medium", "impact": "high", "mitigation": "Grep sweep across *Screen.tsx/*Page.tsx submit handlers; convert each hit to an action-list row; unblock the pm-frontend lint-gates action by resolving its UNRESOLVED:pm-qa sentinel (pm-qa accepts the gate rollout as warn-only for one sprint — formalized in decisions.md)."},
    {"risk": "Announcement fan-out RLS predicate is tested only by a pure-Rust re-model (#2484), not the real SQL — a schema migration touching announcement RLS could regress visibility without the test suite catching it; carried 107 days with announcements.rs touched by multiple recent PRs.", "probability": "medium", "impact": "high", "mitigation": "Replace the pure-Rust re-model with a sqlx integration test; make it a #[sqlx::test] with a multi-tenant seed."}
  ],
  "open_questions": [
    "Should the lint-gates adoption action (pm-frontend-mobile-rn-lint-gates-adopt) be unblocked by pm-qa accepting warn-only-for-one-sprint, or does pm-qa want to see the retrospective-30-day cost first?",
    "Is there an existing release-readiness view distinct from the planner buffer-health view, or do we need to create one?",
    "For the mobile-native gated items — does blocking on #2652 mean QA defers regression tests on those paths entirely, or should we stand up a local-only KMP test lane (non-CI) as a stopgap?"
  ],
  "decisions_needed": [
    "Formalize the 2026-08-25 test-shadow decision proposal (CI diff-guard on dispatcher-spawned fix PRs) — pm-tech-lead + pm-qa sign-off — owner: pm-tech-lead",
    "Unblock pm-frontend-mobile-rn-lint-gates-adopt by pm-qa accepting warn-only-for-one-sprint (clears the UNRESOLVED:pm-qa sentinel) — owner: pm-qa + pm-frontend",
    "Define release-readiness view separate from planner-buffer view — so mobile-native-gated items surface as release blockers not just planner blockers — owner: pm-qa + pm-scrum-master"
  ]
}
```

## Notes (operational, not in JSON)

- Rotation state advanced: pm_cursor.next_index 3 → 4 (pm-devops next). role_last_run["pm-qa"] = 2026-11-07.
- 2 PRs merged map cleanly to 2 in-progress action-list items (`code-review-reality-web-share-cmp-unchecked-json`, `code-review-ppt-web-ui-registry-no-i18n`) — both flipped to `done`.
- The pm-frontend `fake-submit` pattern cluster first noted on 2026-10-28 hasn't been swept since; this run queues a cross-cutting audit action.
- The two poisoned-sentinel action rows (UNRESOLVED:pm-qa, UNRESOLVED:pm-security) continue to block their dependents — resolving them is a high-leverage next step distinct from new work.
