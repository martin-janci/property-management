# pm-qa — QA / Test lens (2026-08-09)

_Rotation idx 3 of 8. Read-only static analysis of sprint-status + merged PRs + open issues. This-run focus: 18-PR merge surge (2026-08-07T20 -> 08-08T20) + PR #2684 quarantine + 3 in-review reality-server PRs blocked from local verify._

## Role JSON

```json
{
  "role": "pm-qa",
  "summary": "18-PR merge surge in 24h exposes uneven test-coverage discipline: #2718 (HMAC parity \u2014 verification-only, no fix needed) and #2722 (IDOR added regression tests for cross-tenant reads) modeled the right pattern; #2707/#2708/#2710 landed security fixes with unclear regression-test hygiene; #2684 quarantined with CI test-shard(1-4) RED after 3 respawns. Test-shard reliability plus a security-fix regression-test policy are the top QA levers this run.",
  "next_actions": [
    {
      "action": "Investigate #2684 CI test-shard(1-4) failure \u2014 is it a real regression from clippy fix at workflow_executor.rs:1312 (cloned_ref_to_slice_refs -> std::slice::from_ref) or shard-splitting flake?",
      "priority": "high",
      "dependency": "none",
      "definition_of_done": "test-shard failure reproduced locally on the specific shard; classified real-vs-flake; fix landed or task re-queued at lower priority"
    },
    {
      "action": "Add end-to-end SSRF DNS-rebinding regression test for PR #2710 \u2014 resolve-then-connect race guarded by DNS pin",
      "priority": "high",
      "dependency": "none",
      "definition_of_done": "test asserts DNS resolution + connect use pinned IP; adversarial DNS harness in tests/"
    },
    {
      "action": "Add regression test for DoS body-cap on workflow api_call (PR #2707) \u2014 oversized body rejected with 413 without memory spike",
      "priority": "medium",
      "dependency": "none",
      "definition_of_done": "integration test posts >cap body, asserts 413; heap-usage assertion under cap"
    },
    {
      "action": "Add unit test for workflow_executor NaN condition reject (PR #2708) \u2014 evaluate_conditions with NaN numeric operand returns error, not silently false",
      "priority": "medium",
      "dependency": "none",
      "definition_of_done": "unit test in workflow_executor covers NaN in numeric compare + negative-zero + Infinity"
    },
    {
      "action": "Regression test for scheduled notification retry (PR #2714, closes #2612) \u2014 retry backs off and terminates at max attempts (no infinite loop)",
      "priority": "medium",
      "dependency": "none",
      "definition_of_done": "test asserts retry sequence + terminal-failure state; time-mocked to avoid flake"
    },
    {
      "action": "Codify the #2718 no-fix HMAC-parity outcome as a documented policy \u2014 any webhook fix must ship a parity assertion test + a replay-window test",
      "priority": "medium",
      "dependency": "none",
      "definition_of_done": "docs/testing/webhook-parity-policy.md landed; PR template updated"
    }
  ],
  "risks": [
    {
      "risk": "Merge pace of 4 security-adjacent PRs in one 24h window (SSRF, IDOR, DoS, NaN) exceeds pm-security's after-the-fact review capacity \u2014 regressions could ship undetected",
      "probability": "medium",
      "impact": "high",
      "mitigation": "Add explicit security regression checkpoint to daily post-merge review; require regression test co-committed with security fix"
    },
    {
      "risk": "PR #2684 CI test-shards RED after 3 fix rounds implies systemic test-shard fragility, not per-PR bug \u2014 may block unrelated future PRs unpredictably",
      "probability": "medium",
      "impact": "medium",
      "mitigation": "Sample 5 recent test-shard runs for shard-affinity failures; if consistent, extract to isolate test"
    },
    {
      "risk": "#2723 announcement fan-out metrics shipped but the counterpart #2484 real-SQL RLS integration test is still open \u2014 metrics may under/over-count in edge cases the pure-Rust re-model doesn't catch",
      "probability": "medium",
      "impact": "medium",
      "mitigation": "Land real-SQL integration test before promoting metrics to production dashboard"
    },
    {
      "risk": "3 in-review reality-server PRs (#2724/#2725/#2726) all report 'verify gate unrunnable in cloud' (utoipa-swagger-ui build blocked by egress) \u2014 retry loop is running blind on CI-only signals",
      "probability": "high",
      "impact": "medium",
      "mitigation": "Mirror utoipa-swagger-ui build deps into cloud egress allow-list OR make verify gate skip-with-report on cloud"
    },
    {
      "risk": "Community-read IDOR fix #2722 added regression tests, but pattern needs an audit sweep for any remaining unauthenticated read routes across other handlers",
      "probability": "low",
      "impact": "high",
      "mitigation": "pm-security grep for handlers missing principal extractor; queue remaining hits to action-list"
    }
  ],
  "open_questions": [
    "Is #2684's test-shard failure a real regression a human should see, or an artifact of shard boundary \u2014 needed before we un-quarantine?",
    "For #2724/#2725/#2726 that couldn't run CI locally, should reviewer wait on GitHub Actions or shift to a merge-then-monitor stance?",
    "Should the #2718 no-fix-needed verification outcome be tracked as a distinct outcome in coverage.json, separate from done/partial?",
    "Are the workflow_executor changes (#2707/#2708) covered end-to-end by a workflow-integration test, or only by unit tests?"
  ],
  "decisions_needed": [
    "Should security fixes co-commit a regression test at PR-open time (mandatory), or is post-merge follow-up acceptable? \u2014 owner: pm-tech-lead + pm-security",
    "Test-shard budget \u2014 how many shard-splits are we willing to maintain vs consolidating? \u2014 owner: pm-devops"
  ]
}
```

## Notes

- Rotation idx 3 of 8; last pm-qa run 2026-06-15 (55 days stale — longest stale slot).
- Six pm-qa next_actions appended to `action-list.json` with `source = "pm-analysis 2026-08-09"`.
- Five risks dedup-checked against existing pm-qa risk IDs and appended.
- Merged-PR context for this run:
  - **Positive test-hygiene models:** #2718 (HMAC parity verification, no fix needed — a valid "verify" outcome), #2722 (IDOR fix with co-committed cross-tenant regression tests per task input).
  - **Test-hygiene unclear:** #2707 (DoS body cap), #2708 (workflow NaN reject), #2710 (SSRF DNS-rebinding TOCTOU), #2714 (scheduled notification retry). Actions queued to close the gap.
  - **CI signal broken:** #2684 quarantined after all 4 test-shards RED post-clippy fix (workflow_executor.rs:1312); root cause investigation queued as top-priority action.
- Coverage epic-80 (upkeep this run): all 3 dispute stories still `done`; 80-1 evidence refreshed with PR #2712 dispute add_evidence audit event.
- Next pm-qa rotation expected ~ 8 runs from now.
