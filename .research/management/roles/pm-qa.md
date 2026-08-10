# pm-qa — QA / Test lens (2026-08-10)

_Rotation idx 3 of 8. Read-only static analysis of sprint-status + merged PRs + open issues._

## Role JSON

```json
{
  "role": "pm-qa",
  "summary": "22-PR burn window shipped a heavy security wave (SSRF, DoS, IDOR, HMAC, session-invalidate, unauth reads, RAG fail-closed, workflow-executor NaN guard) — every one of them warrants a matching failing-on-main regression test. Coverage bar for the reality-server security batch (#2724/#2725/#2726/#2727) needs an explicit QA gate before we call the security backlog cleared.",
  "next_actions": [
    {"action": "Un-quarantine /disputes/kpis test and add window_start<=window_end validation (400 on inverted window) — follow-up #2575 outstanding 10+ days", "priority": "high", "dependency": "pm-backend", "definition_of_done": "Un-quarantined sqlx::test asserts 400 on inverted window + happy-path payload shape; failing-on-main check confirmed before fix lands."},
    {"action": "Replace pure-Rust announcement fan-out test with a sqlx integration test that exercises the real RLS predicate (#2484 unresolved)", "priority": "high", "dependency": "pm-backend", "definition_of_done": "Integration test uses actual DB + RLS; deleting the SQL predicate makes it fail."},
    {"action": "Add regression tests for the reality-server security batch (#2725 password-reset transport, #2726 SSO session-invalidate error swallow, #2727 agency-members unauth IDOR) — code fixes shipped but ship each with a failing-on-main negative test", "priority": "high", "dependency": "pm-backend", "definition_of_done": "One integration test per merged fix asserting the pre-fix vulnerability now returns 401/403/500 as appropriate."},
    {"action": "Convert workflow_executor.rs unparseable-condition FAIL-OPEN branch to fail-closed (#2708 landed the NaN guard but the parse branch is a separate gap surfaced by tier1d signal)", "priority": "high", "dependency": "pm-backend", "definition_of_done": "Unknown/unparseable condition JSON returns evaluate=false + audit warning + unit test covers it."},
    {"action": "Add integration test for reality://sso happy-path callback that exercises SsoStateStore.mint() call site (#2574 half-wired regression is the last unclosed follow-up from PR #2568)", "priority": "medium", "dependency": "pm-mobile", "definition_of_done": "Android UI test drives a real SSO deep-link roundtrip and asserts mint()→verify() pair succeeds."},
    {"action": "Add HMAC-parity regression to all four webhook handlers (booking/airbnb/esignature/layout) — PR #2718 landed the layout leg, extend to the other three so #2528 stays closed system-wide", "priority": "medium", "dependency": "pm-integration", "definition_of_done": "Per-webhook test asserts a forged/timestamp-skewed signature is rejected; failing-on-main on booking + airbnb + esignature until wired."}
  ],
  "risks": [
    {"risk": "Security PRs #2724/#2725/#2726/#2727 landed the code fix but no evidence in the merge digest of matching negative regression tests — QA policy (2026-06-15 decisions_needed: 'test file for every security-labelled fix') is not being enforced", "probability": "high", "impact": "high", "mitigation": "Enforce the 2026-06-15 test-with-security-fix policy at PR-review time; QA re-audits the 4 PRs this week and files gap-issues if tests are absent."},
    {"risk": "workflow_executor evaluate_conditions() FAILS OPEN on unparseable stored condition (tier1d signal, still open) — a corrupted or attacker-crafted JSON can silently satisfy any workflow gate", "probability": "medium", "impact": "high", "mitigation": "Land the fail-closed rewrite + parse-error audit log + negative test."},
    {"risk": "Dispute KPI endpoint (#2572→#2575) still test-quarantined 10+ days after the follow-up was filed — reporting consumers can regress silently", "probability": "medium", "impact": "medium", "mitigation": "Un-quarantine, add window validation, and add a second contract test the reporting consumer runs against."},
    {"risk": "Announcement fan-out real-SQL test gap (#2484) unresolved — the pure-Rust re-model can drift from the actual RLS predicate; a fan-out regression would ship undetected", "probability": "medium", "impact": "high", "mitigation": "Replace with a sqlx::test that uses the real query + real RLS policies; delete the pure-Rust duplicate."},
    {"risk": "Reality-server churn (state.rs 1201 lines, agencies.rs 624 lines) landed 3 code-review fixes this window but no matching test-scope expansion — hotspot without a coverage baseline invites the next regression", "probability": "medium", "impact": "medium", "mitigation": "Establish a per-hotspot coverage baseline via cargo-llvm-cov; require any new PR touching top-3 hotspots to hold or improve coverage."}
  ],
  "open_questions": [
    "Are the reality-server security-batch PRs (#2724-#2727) actually shipping matching regression tests, or is the QA policy silently drifting?",
    "Should the announcement fan-out metric added by #2723 come with a lightweight metrics-emitted assertion test, or is it observability-only for now?",
    "Does the #2708 workflow_executor NaN guard cover the same code path as the tier1d unparseable-condition FAIL-OPEN, or are these two orthogonal fixes we've been conflating?",
    "Do we need a dedicated 'security-fix has a failing-on-main test' CI gate, or is the current review policy sufficient?"
  ],
  "decisions_needed": [
    "Enforcement mechanism for the 'test with every security fix' policy — CI gate, review checklist, or scrum-master audit? — owner: pm-tech-lead + pm-qa",
    "Coverage baseline requirement for churn hotspots (top-3 files by recent churn) before further dedupe passes — owner: pm-tech-lead"
  ]
}
```

## Notes

- Rotation idx 3 of 8; next pm-qa run ~2026-08-18 (assuming 1-per-day cadence with 8 roles).
- Six pm-qa next_actions appended to `action-list.json` with `source = "pm-analysis 2026-08-10"`.
- Five risks appended to `risks.json` this run (dedup-checked against existing pm-qa risk ids).
- Coverage epic-80 (cursor idx 5) refreshed this run: PR #2712 evidence added to story 80-1 (add_evidence IDOR fix), all three 80-x stories re-stamped `last_checked=2026-08-10`; no status flips.
- Post-merge-review batch note: **the reality-server auth batch (#2724/#2725/#2726/#2727) shipped code fixes but the merge digest gives no evidence of matching failing-on-main tests** — flagged as a top risk this run and folded into the top pm-qa next_action.
