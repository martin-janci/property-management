# pm-qa — QA / Test lens (2026-11-07)

_Rotation idx 3 of 8. Read-only static analysis. Previous run: 2026-08-25 (74 days stale — mobile lint-gates decision still unresolved)._

## Role JSON

```json
{
  "role": "pm-qa",
  "summary": "Quiet merge window (2 small front-end code-review PRs, 0 new issues, 0 open), but #3350 is the first dispatcher PR in this window to ship the canonical two-commit TDD split \u2014 a template the other two follow-up queues (five open gh-issue-33xx and the mobile-rn fake-submit cluster) should adopt before merge. Test-shadow debt is NOT growing this run; the lint-gate and screen-map-drift carryovers remain the main QA debt.",
  "next_actions": [
    {
      "action": "Extend reality-web parseListingSummary coverage \u2014 add negative-case tests for the two surviving holes (summary with null/non-object `address`, array body, HTML error page 200) atop #3350's TDD seed; same pattern should land for parseListingDetail too",
      "priority": "medium",
      "dependency": "none",
      "definition_of_done": "reality-web vitest adds 3 negative cases (null address / array body / HTML 200) on parseListingSummary; parseListingDetail gets the same negative sweep; both tests pre-#3350 fail."
    },
    {
      "action": "Add a localeNamespaceParity-style assertion for the new registry.{status,actions,pagination,pet,vehicle} leaf trees across en/sk/cs (top-level parity locales) after PR #3351 \u2014 de/pl/hu may legitimately fall back to en but the tri-locale tree should not drift leaf-wise once landed",
      "priority": "low",
      "dependency": "none",
      "definition_of_done": "localeNamespaceParity suite extended with a registry sub-tree assertion over en/sk/cs; de/pl/hu excluded by design (documented); test fails if any leaf drifts."
    },
    {
      "action": "Formalize the IG3 failing-on-main regression-test gate for dispatcher-spawned PRs \u2014 PR #3350 modelled it (two-commit TDD split: `test:` commit fails on unfixed handler, `fix:` turns it green) but no CI diff-guard enforces it yet; adopt a CI job that rejects an `auto-impl/*` PR whose test commit does not fail on `base_ref`",
      "priority": "medium",
      "dependency": "none",
      "definition_of_done": "CI job rejects any auto-impl/* PR whose designated test commit passes on base_ref; green on current dev."
    },
    {
      "action": "Sweep the five open from-merged-review follow-ups (#3304-#3310) for screen-map drift \u2014 none reference docs/screens/ updates; add a dispatcher pre-merge gate that flags any PR touching frontend routes without a screen-map Agent Log entry (companion to carried screen-map-drift-pr-3293-reality)",
      "priority": "low",
      "dependency": "none",
      "definition_of_done": "Dispatcher pre-merge gate warns on frontend-route diff without a docs/screens/ Agent Log entry in the same PR; applied retroactively to #3304-#3310."
    }
  ],
  "risks": [
    {
      "risk": "Carried: dispatcher-spawned fix PRs still ship without the failing-on-main regression test enforced in CI \u2014 #3350's two-commit TDD split models the pattern but is author-discipline, not mechanized. Next drift window could reintroduce a fix-without-test even post-#3350.",
      "probability": "medium",
      "impact": "medium",
      "mitigation": "Adopt a CI diff-guard on auto-impl/* branches: the first commit's test file must fail on merge-base; the final commit's test files must all pass."
    },
    {
      "risk": "Screen-map drift is accumulating silently \u2014 #3293 (reality-web realtor/[id] i18n) landed without a screen-map update (carried); the five fresh gh-issue-3304..3310 follow-ups all touch frontend routes and none reference docs/screens/. The 'Rules for agents' section D cadence is not being observed.",
      "probability": "high",
      "impact": "low",
      "mitigation": "Dispatcher-side pre-merge gate on any frontend-route-touching PR; warn-only for one sprint, then block."
    },
    {
      "risk": "pm-qa rotation was 74 days stale (last run 2026-08-25 \u2192 2026-11-07) \u2014 the lint-gates decision (eslint-plugin-react-hooks + i18next/no-literal-string on frontend/apps/mobile) is now 74 days unresolved; the carried mobile-rn fake-submit cluster is still open. Rotation cadence itself is a delivery risk.",
      "probability": "medium",
      "impact": "medium",
      "mitigation": "Keep pm-qa on the 8-day rotation (every ~8 daily runs) \u2014 no missed slots. Consider bumping to twice-a-rotation on run weeks that merge >5 frontend PRs."
    }
  ],
  "open_questions": [
    "Should the IG3 test-first gate be a hard CI-block on auto-impl/* branches, or should implementer specialists keep it as DoD-only (the current state)?",
    "For a pure i18n refactor like PR #3351 (no behaviour change), is 'IG3 N/A' acceptable or should we require a before/after render-snapshot diff test per locale?",
    "Who owns the screen-map pre-merge gate \u2014 pm-qa (where it logically sits) or the dispatcher reviewer (where it would enforce)?"
  ],
  "decisions_needed": [
    "IG3 CI diff-guard \u2014 adopt as blocking on `auto-impl/*` branches? \u2014 owner: pm-tech-lead + pm-qa",
    "Screen-map pre-merge gate (frontend-route touching PR requires same-PR docs/screens Agent Log entry) \u2014 adopt? \u2014 owner: pm-qa"
  ]
}
```

## Notes

- Delta vs 2026-08-25 run: 2 merged PRs this window (both frontend), 0 new issues, 0 touched open PRs. **Zero test-shadow additions** (PR #3350 lands a proper two-commit TDD split; PR #3351 is pure i18n-refactor with existing parity suites).
- Carried-forward items from 2026-08-25: mobile-rn lint-gates adoption (now action `pm-frontend-mobile-rn-lint-gates-adopt-2026-10-28`, still open), fuzz-corpus discipline (not progressed), >1-replica concurrency harness (not progressed).
- New: #3350 demonstrates the canonical dispatcher TDD contract — codify it in CI before the pattern decays.
