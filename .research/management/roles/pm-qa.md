# pm-qa — QA / Test lens (2026-08-08)

_Rotation idx 3 of 8. Read-only static analysis of sprint-status + merged PRs + open issues._

## Role JSON

```json
{
  "role": "pm-qa",
  "summary": "This window's 6 merged PRs are dominated by security/DoS hardening fixes (response-body cap, RAG fail-closed, non-finite condition compare) with no confirmed regression tests in the available evidence — that's the top QA gap. Separately, epic-80 (Dispute Resolution) has a completion-status disagreement between sprint-status.yaml and coverage.json that should be resolved before treating it as release-ready.",
  "next_actions": [
    {"action": "Add regression test for PR #2707 workflow api_call 8 MiB response-body cap — boundary (exactly 8 MiB) + over-cap rejection cases", "priority": "medium", "dependency": "rust-backend", "definition_of_done": "Test asserts a response body just under 8 MiB is accepted and just over is rejected with the expected error, not a silent truncation."},
    {"action": "Add regression test for PR #2706 RAG embedding partial-batch fail-closed behavior in /rag/index", "priority": "medium", "dependency": "rust-backend", "definition_of_done": "Test asserts a batch with one failing embedding aborts the whole batch rather than persisting a mixed-quality set."},
    {"action": "Add regression test for PR #2708 non-finite (NaN/Infinity) rejection in workflow condition compare", "priority": "medium", "dependency": "rust-backend", "definition_of_done": "Test asserts NaN/Infinity/-Infinity operands are rejected (not silently coerced) in evaluate_conditions()."},
    {"action": "Add regression coverage for PR #2712 dispute add_evidence access-audit event", "priority": "low", "dependency": "rust-backend", "definition_of_done": "Test asserts the audit_logs event is emitted with the correct actor/dispute_id payload on a successful add_evidence call, and NOT emitted on a rejected cross-org attempt."},
    {"action": "Release-readiness: run a full Dispute Resolution (epic-80) regression pass before treating the epic as fully shipped — sprint-status.yaml (1/3) and coverage.json (3/3 done) disagree", "priority": "medium", "dependency": "none", "definition_of_done": "Full dispute test suite (currently 152 tests per prior evidence) re-run and green; sprint-status.yaml and coverage.json reconciled to the same number."},
    {"action": "Build a risk-based regression suite for workflow_executor.rs condition evaluation — repeated-churn hotspot (runs_seen 2) with 2 distinct bug classes fixed this window", "priority": "medium", "dependency": "rust-backend", "definition_of_done": "Property-based/fuzz test suite covers condition-compare edge cases (non-finite numbers, malformed JSON, deeply nested groups) as a standing regression gate."}
  ],
  "risks": [
    {"risk": "workflow_executor.rs is a 2-run repeated-churn hotspot with 2 distinct bug classes fixed this window (fail-open condition parsing pending via PR #2684, non-finite numeric compare via #2708) but no consolidated regression suite for condition evaluation", "probability": "high", "impact": "medium", "mitigation": "Build a property-based/fuzz test suite for condition-compare edge cases (queued this run)"},
    {"risk": "Epic-80 completion status disagrees between sprint-status.yaml (1/3) and coverage.json (3/3 done) — if release sign-off trusts sprint-status, the epic could ship without a confirmed full regression pass", "probability": "medium", "impact": "medium", "mitigation": "Run the full dispute-flow regression suite and reconcile status before treating epic-80 as release-ready"},
    {"risk": "3 security/DoS fixes merged this window (#2707 response-body cap, #2706 RAG fail-closed, #2708 non-finite reject) have no confirmed regression test per available evidence — a later refactor could silently reintroduce any of the three", "probability": "medium", "impact": "medium", "mitigation": "Land the 3 queued regression tests before the next refactor of these files"}
  ],
  "open_questions": [
    "Do #2707/#2706/#2708 already carry in-PR test coverage that simply wasn't surfaced in the Phase-1 PR summary, or is regression coverage genuinely absent? (Static read-only pass could not confirm test-file diffs from the summary alone.)",
    "Is the 152-test dispute suite figure (from the 2026-06-25 80-3 promotion note) still current, or has it drifted since?"
  ],
  "decisions_needed": [
    "Epic-80 (Dispute Resolution): should QA sign-off gate on a fresh full-epic regression pass before either sprint-status.yaml or coverage.json is trusted as authoritative? — owner: pm-qa + pm-scrum-master"
  ]
}
```

## Notes

- Rotation idx 3 of 8; last pm-qa run was 2026-06-15 (54 days stale) — this run brings the rotation current.
- Six `next_actions` appended to `action-list.json` with `source = "pm-analysis 2026-08-08"`, all targeting confirmed regression-test gaps on this window's 6 merged PRs plus the workflow_executor.rs repeated-churn hotspot.
- Three risks dedup-checked against existing pm-qa risk IDs and appended to `risks.json`.
- Coverage epic-80 (rotation idx 5, this run's rotating epic) re-checked: all 3 stories remain `done`; evidence appended to 80-1 for PR #2712 (add_evidence access-audit event) and to 84-5 for PR #2706 (RAG fail-closed). No status flips — the gap here is documentation (sprint-status.yaml drift), not implementation, which is why it's raised as a decision rather than a coverage-status change.
- Static read-only pass: could not directly confirm whether #2707/#2706/#2708 shipped with their own regression tests (PR diff/test-file contents weren't in the Phase-1 summary) — treated the absence of evidence as a gap rather than assuming coverage exists, per the "no invention" operating contract; flagged as an `open_question` rather than a certainty.
