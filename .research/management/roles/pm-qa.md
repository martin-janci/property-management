# pm-qa — QA / Test lens (2026-08-13)

_Rotation idx 3 of 8. Read-only static analysis of sprint-status + merged PRs + open issues._

## Role JSON

```json
{
  "role": "pm-qa",
  "summary": "Quiet window (0 merged PRs) with no new AC risk to review; the standing release-blocking gap is PR #2684 (workflow-condition fail-open fix) quarantined with CI still red on test-shard 1-4 after 3 exhausted fix rounds, and 4 KMP quality/test-gap findings (incl. an untested SsoService auth surface) stuck behind the cloud runner's structural Gradle/AGP block. Sprint-status epics (6, 7a, 10b, 80) still show in-progress/partial at the epic level even though every underlying story is `done` in development_status — a tracking-hygiene gap that could mask un-reconciled ACs.",
  "next_actions": [
    {"action": "Triage PR #2684 (auto-impl/code-review-api-core-workflow-cond-parse-failopen): determine whether the test-shard 1-4 failure on head 926afe8 is a real regression from the round-3 clippy fix (std::slice::from_ref change to workflow_executor.rs:1312) or a shard-level flake, since fmt-clippy+check are green but tests are not — quarantine reason gives no shard-level failure detail to distinguish the two.", "priority": "high", "dependency": "pm-backend", "definition_of_done": "Failing test-shard 1-4 case(s) identified by name and classified flaky-vs-regression; if regression, a 4th fix round or manual pickup is scoped (fix_rounds=3 auto-cap already exhausted, so this needs explicit reviewer/human intervention, not another auto-respawn)."},
    {"action": "Add a direct commonTest suite for mobile-native-kmp SsoService.kt (validateAndLogin/loginWithPassword/register/requestPasswordReset/refreshSession/restoreSession/401-logout) — it is the security-critical core of the auth surface and currently has zero coverage while sibling pieces (SsoStateStore, SsoInitiation, AuthGuard) are tested.", "priority": "high", "dependency": "pm-mobile (blocked on cloud runner)", "definition_of_done": "SsoService test file exists exercising success/error/status-mapping branches for each public method; landed via macOS runner or ppt-bridge route since cloud Gradle/AGP fetch is 403-blocked."},
    {"action": "Reconcile sprint-status.yaml epic-level status (epic-6, epic-7a, epic-10b all `in-progress`, epic-80 `partial`) against development_status, where all their stories already show `done` — confirm this is a stale rollup, not a signal that some AC verification was skipped.", "priority": "medium", "dependency": "pm-scrum-master", "definition_of_done": "Epic status fields flipped to done/match story-level truth, or an explicit note added explaining why an epic stays open despite all listed stories done."}
  ],
  "risks": [
    {"risk": "A security fail-open bug (workflow condition parser) is confirmed but stuck in quarantine 5+ days with CI red and fix_rounds exhausted — it ships to no one, but the underlying vulnerability class also isn't confirmed fixed anywhere else on dev.", "probability": "medium", "impact": "high", "mitigation": "Escalate #2684 to human/reviewer triage outside the auto fix-round budget; do not let quarantine silently become permanent shelving."},
    {"risk": "mobile-native-kmp cancellation-swallowing bug (9 of 10 shared repos catch CancellationException via generic catch(e: Exception)) produces spurious error UI on normal screen-navigation cancellation and undermines structured concurrency, but is unlandable in the cloud runner — backlog of untestable mobile findings grows with no closure path.", "probability": "high", "impact": "medium", "mitigation": "Batch the 4 open KMP findings (cancellation-swallowed, SsoService untested, portfolio-analytics-caps-100, plus prior SSO CSRF-class issues) into one macOS/ppt-bridge-routed landing pass rather than leaving them to accumulate one-by-one."},
    {"risk": "getPortfolioAnalytics() truncates realtor dashboards at 100 listings (ignores MyListingsResponse.total, no offset paging) — silently under-reports views/inquiries/favorites/trends for any realtor portfolio over 100 listings, with no test asserting the >100 boundary.", "probability": "medium", "impact": "medium", "mitigation": "Add a >100-listing boundary regression test alongside the paging fix once KMP lands; until then, flag as a known reporting-accuracy limitation rather than a silent gap."}
  ],
  "open_questions": [
    "Is there a defined un-quarantine process/owner for PRs like #2684 once the 3-round auto-fix budget is exhausted, or does quarantine currently mean indefinite shelving with no re-trigger path?",
    "Is the test-shard 1-4 failure on #2684 a known-flaky shard class (per docs/runbooks/nextest-partition-and-test-consolidation.md) or a first observation — one data point isn't enough to classify a stable-vs-flaky shard pattern this run.",
    "Was the /disputes/kpis quarantined-test-only gap (risks.json, story 80-3 area) actually closed by PR #2687 (adds an executing test outside BIT-440 + window-ordering 422 validation), or does the quarantined BIT-440 case still need explicit un-quarantining?"
  ],
  "decisions_needed": [
    "Whether #2684's fail-open workflow-condition fix should be hand-carried past the exhausted fix_rounds cap given its security severity, vs waiting for the next auto-dispatch cycle — owner: pm-backend / pm-tech-lead"
  ]
}
```
