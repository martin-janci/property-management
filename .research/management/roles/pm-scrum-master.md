# pm-scrum-master — 2026-10-27

**Summary.** Review-throttled day: 9 approved PRs sit in `assignments.json status=review verdict=approve` awaiting human merge (dispatcher payload: `claimable=6/72`). One narrow PR merged (#3183 — cross-tenant IDOR regression suite + 404 fix for violation mutations). pm-frontend rotation surfaces no new frontend-side blockers beyond the same reviewer-merge jam that is choking everything.

## Shipped since last run

- **#3183** — code-review-api-handlers-violations-no-xtenant-idor-tests: cross-tenant IDOR regression suite + 404 fix for violation mutations. Does not map to a coverage story (archived code-review task_id).

## Sprint progress

Sprint: **Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth** · epics_done = 3/5 at sprint level · 49/49 stories done at coverage level.

## Next actions (top 5)

1. **[high · pm-tech-lead]** Clear the 9-PR approved-review jam (merge #3024, #3075, #3108, #3119, #3171, #3184, #3185, #3186, #3188 after a last-minute re-approval sanity check). DoD: all 9 PRs merged or triaged to needs-changes with a one-line reviewer note.
2. **[high · pm-backend]** Sweep violations.rs for role-gate / identity-check gaps — Phase 1.5 surfaced 3 fresh findings this run (add_evidence role-gate asymmetry at :375, list_violations/list_appeals bulk leak at :295/:620, create_appeal missing violator-identity check at :584-600). DoD: one-shot audit of all violations.rs handlers; sweep plan proposed before per-site patching.
3. **[medium · pm-tech-lead]** Decide the auto-merge policy for Claude-Approvals PRs after a 24-48h cool-down. DoD: decision logged in `decisions.md`; auto-merge policy enforced or explicitly deferred.
4. **[medium · pm-backend]** Diagnose + resolve the merge-pipeline deadlock (#3131). DoD: traced repro or an upgraded sqlx advisory-lock usage note.
5. **[low · pm-backend]** Un-quarantine the dispute KPIs test + add window_start<=window_end validation (carried item from 2026-07-30). DoD: test suite passes on dev again.

## Risks

1. **[high prob × high impact · pm-tech-lead]** Review-throughput jam — 9 PRs approved by reviewer bot, blocked on human merge for >24h. Dispatcher cannot drain WIP; buffer stays effectively `claimable=6/72` regardless of refill. _Mitigation:_ define an auto-merge policy for green + Claude-Approved rows after a cool-down, OR carve a daily human-merge window with the operator.
2. **[medium prob × high impact · pm-backend]** violations.rs continues to leak authz oversights — 3 new findings this run after #3164/#3171 just addressed one. Whole route needs a role-gate audit before further per-site patching. _Mitigation:_ one-shot sweep for the require_manager/role-gate + identity-check pattern.

## Blockers

- 9 PRs stuck in approved-review state awaiting human merge — reviewer bot approves, no human merge window in >24h. Owner: **pm-tech-lead**.
- **#2652** — mobile-native/KMP cloud builds blocked (carried). Owner: pm-devops.
- **#2951** — mobile RN jest-expo version rot (carried). Owner: pm-devops.
- **#3131** — merge-pipeline deadlock (carried, likely sqlx advisory-lock contention). Owner: pm-backend.

## Open questions

_None this run._

## Decisions needed

- **Auto-merge policy for Claude-Approvals PRs** — should the dispatcher auto-merge after a 24-48h cool-down, or does the human-merge step stay a hard gate? Owner: pm-tech-lead.

## Return JSON (as consumed by the orchestrator)

```json
{
  "role": "pm-scrum-master",
  "summary": "Review-throttled day: 9 approved PRs sit in review awaiting human merge (dispatcher payload: claimable=6/72). One narrow PR merged (#3183 — xtenant-IDOR regression suite + 404 fix). pm-frontend rotation surfaces no new frontend-side blockers.",
  "shipped_since_last_run": ["#3183 code-review-api-handlers-violations-no-xtenant-idor-tests"],
  "sprint_progress": {"sprint": "Epic 6, 7A, 8A & 10A", "epics_done": 3, "epics_total": 5},
  "next_actions": [
    {"action": "Clear the 9-PR approved-review jam", "priority": "high", "dependency": "pm-tech-lead", "definition_of_done": "all 9 PRs merged or triaged to needs-changes"},
    {"action": "Sweep violations.rs for role-gate / identity-check gaps", "priority": "high", "dependency": "none", "definition_of_done": "one-shot audit + sweep plan"},
    {"action": "Decide auto-merge policy for Claude-Approvals PRs after cool-down", "priority": "medium", "dependency": "none", "definition_of_done": "decision logged"},
    {"action": "Diagnose #3131 merge-pipeline deadlock", "priority": "medium", "dependency": "none", "definition_of_done": "traced repro"},
    {"action": "Un-quarantine dispute KPIs test + window_start<=window_end validation", "priority": "low", "dependency": "none", "definition_of_done": "test suite passes on dev"}
  ],
  "risks": [
    {"risk": "Review-throughput jam", "probability": "high", "impact": "high", "mitigation": "auto-merge after cool-down OR daily human-merge window"},
    {"risk": "violations.rs authz oversights accumulating", "probability": "medium", "impact": "high", "mitigation": "one-shot sweep before per-site fixes"}
  ],
  "open_questions": [],
  "decisions_needed": ["Should the dispatcher auto-merge Claude-Approvals PRs after a 24-48h cool-down? — owner: pm-tech-lead"],
  "blockers": [
    {"item": "9 PRs stuck in approved-review awaiting human merge", "reason": "No human merge window has drained them in >24h", "owner_role": "pm-tech-lead"},
    {"item": "#2652 KMP cloud builds blocked", "reason": "Carried", "owner_role": "pm-devops"},
    {"item": "#2951 RN jest-expo version rot", "reason": "Carried", "owner_role": "pm-devops"},
    {"item": "#3131 merge-pipeline deadlock", "reason": "Carried — likely sqlx advisory-lock contention", "owner_role": "pm-backend"}
  ]
}
```
