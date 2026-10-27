# pm-frontend — 2026-10-27

**Summary.** Frontend had minimal activity this run (0 frontend PRs merged). The 2 ppt-web-core code-review PRs (#3185 useNeighbors stale-token useMemo, #3186 useIotWebSocket.hasGivenUp drop) are both approved but stuck in the same review-jam blocking everything today. KMP continues structurally blocked by #2652 cloud-build env; the CreateListingScreen onSubmit stub and MyListings pagination cap remain carried.

## Next actions (top 4)

1. **[high · pm-tech-lead]** Push the 2 approved ppt-web-core PRs (#3185, #3186) through the human-merge gate so the frontend review-slot clears. DoD: #3185 and #3186 merged; the two matching action-list rows (`code-review-ppt-web-core-neighbors-stale-token-usememo`, `code-review-ppt-web-core-iot-ws-hasgivenup-dropped`) flip to done.
2. **[low · pm-frontend]** Reconcile ppt-web screen-map drift from PR #2994 (route refactor without `docs/screens/ppt/` updates). PR #3024 is the queued resolution and is itself in the review-jam. DoD: `docs/screens/ppt/*` mirrors the current route tree; `/screens validate` passes.
3. **[low · pm-frontend]** Link UC-33.1 / UC-33.2 / UC-33.3 to the correct screen-map entries — `coverage.screen_gaps.missing_use_cases` carries them. DoD: `docs/screens/<product>/*` frontmatter `use-cases` arrays updated; `missing_use_cases` drops to 0.
4. **[medium · pm-frontend]** Fix KMP CreateListingScreen onSubmit NotImplementedError stub. Dep-blocked by #2652 (KMP cloud builds). DoD: `onSubmit` wires to shared SDK create endpoint; local KMP build succeeds.

## Risks

1. **[high prob × medium impact · pm-tech-lead]** Frontend review-jam compounds mobile stall — #3185 + #3186 both approved and stuck in the same human-merge gate that blocks every PR today. _Mitigation:_ same as pm-scrum-master's auto-merge-after-cool-down proposal.
2. **[medium prob × low impact · pm-frontend]** Screen-map drift is accumulating silently — PR #2994 drifted without a CI gate catching it; next refactor will land the same way. _Mitigation:_ add a workflow step that fails the PR if frontend route files change without a matching `docs/screens/<product>/*` diff.

## Open questions

_None this run._

## Decisions needed

_None from the frontend lens; see pm-scrum-master's auto-merge policy decision — it covers the frontend side of the jam._

## Return JSON (as consumed by the orchestrator)

```json
{
  "role": "pm-frontend",
  "summary": "Frontend had minimal activity this run (0 frontend PRs merged). The 2 ppt-web-core code-review PRs (#3185 neighbors stale token, #3186 iot-ws hasGivenUp) are both approved but stuck in the review-jam. KMP continues structurally blocked by #2652 cloud-build env.",
  "next_actions": [
    {"action": "Push #3185 and #3186 through the human-merge gate so the frontend review-slot clears", "priority": "high", "dependency": "pm-tech-lead", "definition_of_done": "both merged; matching action-list rows flip to done"},
    {"action": "Reconcile ppt-web screen-map drift from PR #2994", "priority": "low", "dependency": "none", "definition_of_done": "docs/screens/ppt/* mirrors route tree; /screens validate passes"},
    {"action": "Link UC-33.1 / UC-33.2 / UC-33.3 to their screen-map entries", "priority": "low", "dependency": "none", "definition_of_done": "missing_use_cases drops to 0"},
    {"action": "Fix KMP CreateListingScreen onSubmit stub (dep-blocked by #2652)", "priority": "medium", "dependency": "pm-devops", "definition_of_done": "onSubmit wires to shared SDK; local KMP build succeeds"}
  ],
  "risks": [
    {"risk": "Frontend review-jam compounds mobile stall", "probability": "high", "impact": "medium", "mitigation": "auto-merge after cool-down OR daily human-merge window (per pm-scrum-master)"},
    {"risk": "Screen-map drift accumulating silently without CI gate", "probability": "medium", "impact": "low", "mitigation": "CI step: fail PR if frontend route files change without matching docs/screens/<product>/* diff"}
  ],
  "open_questions": [],
  "decisions_needed": []
}
```
