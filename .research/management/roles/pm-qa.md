# pm-qa — QA / Test lens (2026-08-08)

_Rotation idx 3 of 8. Last run 2026-06-15 (54 days stale — long overdue). Read-only static analysis of sprint-status + this window's 7 merged PRs + 4 open PRs + issues #2703/#2704/#2612._

## Role JSON

```json
{
  "role": "pm-qa",
  "summary": "This window's 7 merges cleared 3 code-review findings and the #2704 memory-DoS fast, but two hotfix-class PRs (#2707, #2712) shipped without a named regression test — the exact 'hotfix-no-test' pattern pm-backend asked for a merge-gate on 2026-07-30. And #2696 (inquiry-email seam) is ready-to-merge but functionally dead until its route-wiring follow-up lands.",
  "next_actions": [
    {"action": "Sequence-lock: hold #2696 (inquiry-email notifier seam) merge until code-review-reality-server-inquiry-notify-route-wiring lands together — merging #2696 alone ships a live 'success message, no notification' regression on send_contact_message", "priority": "high", "dependency": "pm-backend", "definition_of_done": "Both PRs merged in the same window; a route-level test asserts the notifier fires on send_contact_message."},
    {"action": "Backfill regression test for #2707 body-cap (memory-DoS): oversize Content-Length pre-check + mid-stream over-cap chunked-transfer reject; assert truncation-marker + tracing::warn emitted", "priority": "high", "dependency": "pm-backend", "definition_of_done": "New test in api_call tests: pre-check reject + streaming cap enforced + truncation marker present in stored payload."},
    {"action": "Prove SSRF DNS-rebinding TOCTOU fix (draft #2710, closes #2703) with a spoofed-resolver test — first lookup returns public, second returns private; assert workflow api_call refuses the resolved-late private IP", "priority": "high", "dependency": "pm-tech-lead", "definition_of_done": "Test drives a fake DNS resolver; asserts the workflow api_call refuses the resolved-late private IP."},
    {"action": "Backfill regression test for #2712 add_evidence audit event — assert audit_logs row is written on dispute evidence upload with correct actor/resource/dispute context", "priority": "medium", "dependency": "pm-data", "definition_of_done": "Failing-on-main test in dispute repo suite; passes with #2712."},
    {"action": "Vendor / cache utoipa-swagger-ui zip so cargo test / clippy for api-server can run locally in the sandbox (currently DEFERRED-TO-CI on every refactor PR — including #2711/#2713 this window)", "priority": "medium", "dependency": "pm-devops", "definition_of_done": "`cd backend && cargo test -p api-server` succeeds locally without github.com egress; documented in verify guide."},
    {"action": "Add sqlx integration test for the announcement fan-out RLS predicate (open risk risk-announcement-fanout-test-fidelity-2026-07-23 / gh-issue-2484) — replace pure-Rust re-model with a test hitting the actual policy", "priority": "medium", "dependency": "pm-backend", "definition_of_done": "New sqlx::test in announcement_targeting_visibility_tests.rs exercises the SQL predicate directly."}
  ],
  "risks": [
    {"risk": "Hotfix-no-test pattern recurring: PR #2707 (memory-DoS body cap, closes #2704) and PR #2712 (dispute add_evidence audit event) shipped this window without a named regression test — exactly the standard pm-backend proposed on 2026-07-30. Without a merge-gate, the next similar security/data-loss fix will slip.", "probability": "high", "impact": "medium", "mitigation": "Adopt the 2026-07-30 pm-backend standard as a PR-template checklist item + reviewer gate; retro-backfill this window's two gaps."},
    {"risk": "Ready-to-merge #2696 in isolation ships a 'silent success' regression — the live public endpoint send_contact_message bypasses the new notifier seam per already-known follow-up. Reviewer approving in isolation is a real path — the seam itself is correct code, only the wiring is missing.", "probability": "medium", "impact": "high", "mitigation": "Mark #2696 as DO NOT MERGE SOLO in the PR body; add a route-level assertion test that fires on the live endpoint before the pair merges."},
    {"risk": "Sandbox limitation (utoipa-swagger-ui GH egress blocked) means every api-server PR defers cargo test/clippy to CI — the biggest crate has no local pre-flight. Silent CI-only failures land as noise or blocked merges; sandboxed refactor loop can't self-verify.", "probability": "high", "impact": "medium", "mitigation": "Vendor the swagger-ui zip in the repo (or allowlist that specific download via the sandbox proxy); make `just verify` succeed offline for api-server."},
    {"risk": "SSRF DNS-rebinding TOCTOU (#2703, workflow api_call.rs) is a live high-severity vulnerability shipped in prod. Draft PR #2710 has been open >24h and is still not merged. Workflow engine is on the critical path.", "probability": "medium", "impact": "high", "mitigation": "Top-slot #2710 for implementer + reviewer this window; add tests before merge; do not close #2703 until PR merges."},
    {"risk": "sprint-status.yaml still lists epics 6/7a/10b/80 as in-progress despite coverage.json showing all their stories done for weeks — dashboards and any planner reading sprint-yaml will underreport progress and mis-prioritize.", "probability": "high", "impact": "low", "mitigation": "pm-scrum-master reconciles sprint-status.yaml against coverage.json in a housekeeping PR."}
  ],
  "open_questions": [
    "Should regression-test-required-on-security-fix become a hard merge-gate (blocking CI status), or stay a reviewer checklist?",
    "Can #2710 (SSRF TOCTOU) be tested end-to-end with the sandbox proxy, or does it require an integration harness that spins up a rebinding resolver?",
    "Is #2696 acceptable to ship in isolation with a follow-up commitment, or do we hold until the notify-wire PR lands too?",
    "How stale is the accounting MVP-loop trio (#2555/#2558/#2559) reviewer-starvation from 2026-07-30 — did any of them merge? Not in this window's touched-PR list."
  ],
  "decisions_needed": [
    "Merge-gate policy: mandatory named regression test on any PR closing a security or data-loss issue — owner: pm-tech-lead + pm-qa",
    "Sequencing convention for a seam-PR + wire-PR pair (e.g. add tag `blocks-alone` on both) — owner: pm-scrum-master",
    "Backend local pre-flight for api-server (vendor utoipa-swagger-ui zip vs. allowlist proxy) — owner: pm-devops"
  ]
}
```

## Notes

- Rotation idx 3 of 8. Last pm-qa run was 2026-06-15; 54 days stale. Next scheduled rotation ~ 8 days out (pm-devops runs next at idx 4).
- Six pm-qa next_actions appended to `action-list.json` with `source = "pm-analysis 2026-08-08"`.
- Five pm-qa risks appended to `risks.json`, dedup-checked against prior pm-qa risks (2026-06-15 risks are now stale — issues #1360-#1377 batch mostly closed).
- Coverage epic-80 (rotation idx 5) refreshed: PR #2712 add_evidence audit-event evidence added to 80-1; 80-2/80-3 re-check note only. No status flips.
- Post-merge review of the 2026-08-07 batch (PRs #2706-#2713) has NOT been executed as of this Phase 1.6 pass — dispatcher Phase 2.5 will run after Phase 1.6 and may surface additional findings.
