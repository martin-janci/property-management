# pm-qa — QA / Test lens (2026-10-29)

_Rotation idx 3 of 8. Read-only static analysis of sprint-status + merged PRs + open findings. Previous run: 2026-08-25 (65 days stale)._

## Role JSON

```json
{
  "role": "pm-qa",
  "summary": "Zero PRs merged in the 16h since last run; 4 dispatcher-authored auto-impl drafts (#3190–#3193) are stacked behind the human-approval gate (self-approval blocked, see #3131). The real QA signal this run is a test-shadow pattern on the auto-impl pipeline: 11 approved PRs have been sitting in review-queue for 15+ days, long enough that their failing-on-dev regression evidence (per the IG3 contract) is drifting against base. The oldest open blocker, #3187 (cross-tenant assign returns 403 vs expected 404), is still red on dev and reds every backend PR's required `test` check; it has sat 2 days without a fix PR despite being queued as `gh-issue-3187`.",
  "next_actions": [
    {"action": "Treat #3187 (cross_org_assign_violation_is_not_found) as the backend CI's single-point-of-failure: either land PR #3188's fix (align assign_violation handler to 404 cross-tenant) OR flip the test's expected contract to 403-when-manager-role-wrong-tenant and document the deviation. Current state blocks every backend PR's merge path.", "priority": "high", "dependency": "rust-backend", "definition_of_done": "Backend `test` check passes on dev HEAD; the specific test `cross_org_assign_violation_is_not_found` is green; action-list item gh-issue-3187 closed."},
    {"action": "Add a dispatcher-queue aging alarm — if any `status:review` row in assignments.json has been in review >14 days, surface it in the daily brief under 'Watch'. Oldest sibling in the backlog is PR #3024 at ~24 days (last_updated 2026-10-05); the current cadence lets approved-but-unmergeable PRs rot without visibility.", "priority": "high", "dependency": "pm-devops", "definition_of_done": "Routine brief contains a 'Review-queue aging' section listing PRs in review >14 days with age; verified across one subsequent routine run."},
    {"action": "Add a failing-on-main regression test for the add_evidence authz gap (#3164) — Resident/Tenant role attempts POST /violations/{id}/evidence and expects 403, mirroring the existing resident_cannot_delete_evidence case. The gap sits 4 days without a test PR; without a failing test it will keep slipping past review.", "priority": "medium", "dependency": "rust-backend", "definition_of_done": "`violations_role_authz_tests::resident_cannot_add_evidence` exists, red on dev before the fix, green after. Add a manager positive control."},
    {"action": "Audit the 11 in-review auto-impl PRs for IG3-contract drift: each PR must still fail-on-main after rebasing against the current dev HEAD. 15+ days of base drift is long enough that the regression test may now pass against dev (if the issue was fixed via a sibling PR), invalidating the PR's IG3 evidence. Flag drifted ones for rebase+re-verify.", "priority": "medium", "dependency": "pm-tech-lead", "definition_of_done": "Table of 11 PR#, with 'test-still-fails-on-main' column; PRs that drift queued for rebase or dropped."},
    {"action": "Promote the 'review-queue-aging' pattern as a persistent risk — the merge pipeline has been structurally deadlocked for 24+ days per #3131, and all 11 approved PRs depend on the operator granting a second-identity approval. Without a procedural fix the queue grows faster than it drains and QA's regression tests silently rot.", "priority": "medium", "dependency": "pm-tech-lead", "definition_of_done": "Risk entry added to risks.json; one of {second-identity, admin-merge-lane, relax-protection-for-bot-path} chosen as the DEC-002-style decision owned by pm-tech-lead."},
    {"action": "Add a 'test-shadow debt' line to the Scrum Master synthesis — count of auto-impl PRs that merged without the IG3 regression test AND were later re-fixed by a sibling PR. Currently invisible; the ppt-review-merged skill surfaces re-work but doesn't tag the shadow-debt pattern.", "priority": "low", "dependency": "pm-scrum-master", "definition_of_done": "project-state.md includes a 'Test-shadow debt' bullet enumerating re-fix pairs for the last 14 days."}
  ],
  "risks": [
    {"risk": "The merge pipeline has been deadlocked on human review for 24+ days (#3131 still open-ish — closed 2026-10-26 but condition unchanged). 11 approved PRs are now stacked; the oldest (#3024) was approved 2026-10-05. Review-queue drift is an active regression carrier: 15-day-old PRs can silently pass IG3 at open-time but fail on current dev.", "probability": "high", "impact": "high", "mitigation": "Pick one of {second-identity for dispatcher, admin-merge lane for bot-authored PRs, relax `required_approving_review_count` for /auto-impl/* branches}. Any of them is better than the current silent-rot state."},
    {"risk": "#3187 is the first test-shadow case where the auto-impl pipeline's own regression test exposed a different base-branch bug (role-gate vs ownership-gate ordering) — but the fix for that bug has not landed in 2 days, so the test itself keeps every subsequent backend PR red. One red root bug now gates N PRs; the dispatcher cannot route around it.", "probability": "high", "impact": "high", "mitigation": "Treat #3187 as a merge-blocking P0: either fix the handler (preferred, matches cross-tenant 404 convention) or flip the test contract. PR #3188 claims the fix — needs human merge."},
    {"risk": "The add_evidence authz gap (#3164) sat 4 days before being queued and has no failing test on dev. Resident-role write to arbitrary violation evidence is a tamper-record primitive; without a regression test the fix can regress silently (same shape as the ViolationRepository::list_notifications latent IDOR we caught in #3069).", "probability": "medium", "impact": "high", "mitigation": "Land resident_cannot_add_evidence test case red on dev, THEN apply require_manager gate. IG3 ordering."},
    {"risk": "The dispatcher's Tier 1d dev-review generator has emitted 10+ findings in the last 3 days (ppt-web-ui, reality-web, api-handlers, mobile-native-kmp) that are now all queued but un-staffed because WIP is capped at 8 and the merge pipeline is stuck. The buffer-starved mode is masking a growing untouched-findings pile — the backlog is not drained, just invisible.", "probability": "medium", "impact": "medium", "mitigation": "Either (a) raise DISPATCHER_WIP_CAP temporarily to drain, (b) prioritize draining the human-review queue first so WIP falls below cap. Addresses symptom, not root cause (#3131)."},
    {"risk": "Buffer-health gate GC3-buffer-bounds has FAILED on every dispatcher run for 3+ days (claimable 7–24 vs floor 36). The gate is now mostly noise — operators tune it out. Risk is that a real buffer-starvation event (e.g. the generator itself breaks) hides inside the chronic-failure signal.", "probability": "medium", "impact": "medium", "mitigation": "Either temporarily lower the floor to a value the current state can satisfy (restore the gate's signal value), OR fix the root cause (human-review drain). Chronic red gates train operators to ignore them."}
  ],
  "open_questions": [
    "Is the dispatcher-dispatcher cadence (2h) still right when the operator-side (human review) is the structural bottleneck? Running dispatcher-N times per day while the queue cannot drain just inflates `.research/` noise.",
    "The IG3 contract requires a failing-on-main test. Should we add a dispatcher-side 'IG3 freshness' check at merge-time (not just open-time) — re-run the regression test against current dev and gate the merge on it, catching review-queue drift?",
    "Should the ppt-review-merged skill tag re-fix pairs (fix PR N, then fix-for-fix PR N') as a formal 'test-shadow' signal in the backlog, or keep it as a human-only pattern?"
  ],
  "decisions_needed": [
    "Pick the unblock path for the merge pipeline (second-identity, admin-merge-lane, relaxed-protection, or something else) — owner: pm-tech-lead + pm-devops",
    "Should #3187 fix be landed as its own operator-merged PR or rolled into one of the stacked auto-impl PRs that already carry it transitively? — owner: pm-security + pm-tech-lead",
    "Lower GC3-buffer-bounds floor to 24 (match current demand profile) or keep floor=36 as aspirational? — owner: pm-tech-lead"
  ]
}
```

## Notes

- Rotation idx 3 of 8; next pm-qa run ~ 2026-11-28 (assuming ~30-day-per-role cadence).
- 65 days stale (last run 2026-08-25). The sprint-level test-shadow pattern this run finds (approved-but-unmergeable PRs rotting on human review) is a different flavor than 2026-08-25's version (fix-PR-without-regression-test): now the test exists but drifts against moving base. Same root cause (lack of IG3-freshness enforcement at merge-time), different manifestation.
- Six pm-qa next_actions appended to `action-list.json` with `source = "pm-qa 2026-10-29"`; five risks and three decisions_needed appended to risks.md / decisions.md.
- Coverage epic-10a (rotation idx 0 → 1) re-checked — all 3 stories still `done`; `last_checked = 2026-10-29`. No PR in this window touched OAuth provider surfaces (none merged at all).
- Phase-1 observation data that shaped this run:
  - 0 merged PRs since 2026-10-28T10:54:38Z (16h quiet).
  - 4 open PRs touched since last run: all draft, all dispatcher-authored (#3190–#3193).
  - 0 new issues since last_issue_seen=3187. #3187 remains the acute blocker on backend CI.
  - 0 code churn (only `.research/` dispatcher commits).
