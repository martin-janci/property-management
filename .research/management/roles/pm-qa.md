# pm-qa — QA / Test lens (2026-08-24)

_Rotation idx 3 of 8. Last run 2026-06-15 (70d stale). Read-only static analysis of sprint-status + the 13 PRs merged 2026-08-22→08-24 + churn files. No compile, no test execution._

## Headline

The pre-merge review gate **is** under-catching — but not in the way the raw numbers suggest. It is not
letting through *untested* changes; it is letting through changes whose tests sit at the **wrong level**
for the risk the change introduces. Both intra-window regression chains are level mismatches, and both
are mechanically preventable.

## Evidence

| PR | Change | Tests added | Defect that followed | What the fix PR added |
|---|---|---|---|---|
| #2826 | migration `00234_held_notification_delivery_tracking.sql` + `GranularNotificationRepository` + `quiet_hours_drain.rs` per-channel bookkeeping/bounded retry (5 files) | **8 × plain `#[test]`, 0 × `#[sqlx::test]`** — all pure in-process logic tests inside `quiet_hours_drain.rs` | **#2831** — drain double-delivers held notifications across >1 api-server replica | #2834 added migration `00235_held_notification_replica_claim.sql` + `crates/db/tests/suites/held_notification_replica_claim_tests.rs` with 2 `#[sqlx::test]` (`claim_hands_each_due_row_to_at_most_one_replica`, `recording_a_partial_attempt_clears_the_claim_for_prompt_retry`) |
| #2829 | AML dashboard `window.prompt`/`alert` → in-app dialogs + i18n (12 files) | 4 × `it()`, all single-assessment: no-prompt assertion, empty-notes reject, decision-union submit, empty-EDD-reason reject | **#2832** — EDD/Review dialogs retain stale reason/notes across assessments | #2833 added exactly the two missing lifecycle cases: `resets review notes and decision when re-opened for a different assessment`, `resets the EDD reason when re-opened for a different assessment` |

Two files were each patched twice inside 48h: `backend/servers/api-server/src/services/quiet_hours_drain.rs`
and `frontend/apps/ppt-web/src/features/compliance/pages/AmlDashboardPage.tsx`.

## Root cause

`.research/management/pr-reviewer-prompt.md` line 66 instructs the reviewer:

> `| Test files (**/tests/**, **/*_test.rs, **/*.test.ts) | **Skim**: read assertions but don't deeply audit fixtures |`

There is no notion of a **required test level per risk class** anywhere in the prompt. A PR that adds a
schema migration plus eight passing unit tests therefore reads as *well-tested* to the gate, even though
no test ever touched a database. Same for a dialog PR with four green render assertions and no remount case.

## Rework-rate trend (from `post-merge-review.json`)

| Window | `prs_scanned` | `with_issues` | rate |
|---|---|---|---|
| 2026-08-06 → 08-14 | 52 | 0 | 0 % |
| 2026-08-20 → 08-23 | 36 | 8 | **22 %** |
| this window (from-merged-review PRs ÷ merged PRs) | 13 | 5 | **38 %** |

Post-merge review has quietly become the real correctness gate. Nothing currently tracks this ratio.

## Role JSON

```json
{
  "role": "pm-qa",
  "summary": "The pre-merge gate is under-catching by test LEVEL, not test count: #2826 shipped a migration + repo change with 8 pure unit tests and 0 #[sqlx::test] and regressed as #2831 within 48h; #2829 shipped 4 happy-path dialog tests with no remount case and regressed as #2832. Post-merge rework rate went 0% (08-06..08-14) to 22-38% this window, so post-merge review is now the de-facto gate.",
  "next_actions": [
    {"action": "Add a mechanical pre-merge gate: any diff touching backend/crates/db/migrations/** must add >=1 #[sqlx::test] in the same diff", "priority": "high", "dependency": "none", "definition_of_done": "Gate lives in the verify script or reviewer prompt; re-running it against PR #2826's diff blocks it, against #2834's passes."},
    {"action": "Add a risk-class -> required-test-level table to .research/management/pr-reviewer-prompt.md and carve concurrency / cross-process / component-lifecycle changes out of the current 'Skim test files' rule", "priority": "high", "dependency": "none", "definition_of_done": "Reviewer prompt names at least: schema/migration -> #[sqlx::test]; multi-replica or scheduler/drain -> DB-backed concurrency test; dialog/modal state -> remount test; crypto -> round-trip + wrong-key + legacy-read."},
    {"action": "Make 're-open the dialog for a different subject' a required case for every ppt-web dialog holding useState; backfill ReviewAssessmentDialog and InitiateEddDialog which still have no test file", "priority": "medium", "dependency": "none", "definition_of_done": "Both dialogs have their own test file with a remount/reset case; convention documented alongside the reviewer prompt."},
    {"action": "Add encrypt/decrypt round-trip, wrong-key reject and legacy-plaintext-read regression tests for the voice OAuth token encryption centralized by PR #2838", "priority": "medium", "dependency": "none", "definition_of_done": "Three cases land against routes/voice_webhooks.rs; failing-on-main proof for the legacy-read path."},
    {"action": "Set a test floor for ppt-web features/compliance — a test file per dialog and page (ReviewAssessmentDialog, InitiateEddDialog, ContentModerationPage first)", "priority": "medium", "dependency": "none", "definition_of_done": "features/compliance has >1 test file; no regulated-flow PR merges into it without a co-located test."},
    {"action": "Instrument a post-merge rework rate metric in the routine digest (from-merged-review PRs / merged PRs, and post-merge-review with_issues / prs_scanned)", "priority": "medium", "dependency": "none", "definition_of_done": "Both ratios appear in the Phase 6 digest and in project-state.md each run."}
  ],
  "risks": [
    {"risk": "Pre-merge gate accepts level-mismatched tests: 5 of 13 merged PRs this window were from-merged-review follow-ups and two files were patched twice inside 48h; pr-reviewer-prompt.md explicitly says to 'Skim' test files, so a migration PR with 8 pure unit tests reads as well-tested.", "probability": "high", "impact": "high", "mitigation": "Risk-class -> required-test-level table plus a mechanical migration => #[sqlx::test] gate."},
    {"risk": "Post-merge review has become the de-facto correctness gate (with_issues/prs_scanned 0/52 across 08-06..08-14 vs 8/36 across 08-20..08-23); every regression now costs a full extra PR cycle and inflates apparent throughput.", "probability": "high", "impact": "medium", "mitigation": "Track the rework rate in the digest; move the two enumerable defect classes into pre-merge required checks."},
    {"risk": "PR #2838 centralized voice OAuth token encryption in a single-file change with one test marker, on a file already flagged as a repeat churn hotspot. Untested crypto centralization can fail silently in prod (tokens decrypting to garbage rather than erroring).", "probability": "medium", "impact": "high", "mitigation": "Add round-trip / wrong-key / legacy-plaintext cases before further voice_webhooks.rs churn."},
    {"risk": "ppt-web features/compliance ships 3 pages + 7 components behind a single test file, while being both the window's highest-churn UI area and its most regulated (AML / EDD / DSA decisions).", "probability": "medium", "impact": "high", "mitigation": "Per-dialog/per-page test floor enforced before further compliance changes land."},
    {"risk": "8 of 13 PRs merged this window map to no coverage.json story (AML/compliance, facilities booking, verification badge, voice assistant), so QA has no acceptance-criteria anchor for the majority of code actually changing.", "probability": "high", "impact": "medium", "mitigation": "Run the local deep /ppt-project-management scan; until then treat post-merge review findings as the only AC signal for those areas."}
  ],
  "open_questions": [
    "Is the migration => #[sqlx::test] rule better enforced in scripts/verify-impact.sh (hard CI gate) or in pr-reviewer-prompt.md (reviewer judgement)? A hard gate is unbypassable but will false-positive on pure data backfills.",
    "quiet_hours_drain.rs now carries two overlapping mechanisms from #2826 and #2834 (per-channel bookkeeping + bounded retry, and the atomic replica claim). Is the #2826 retry path still reachable, or is it dead code the claim path supersedes?",
    "Do the three UC-33.x dispute sub-use-cases have acceptance criteria anywhere, or only a UC code? They are the last screen-map gap and I found no AC source for them.",
    "Should from-merged-review follow-up PRs be exempt from the post-merge review pass (they are themselves review output), or does re-reviewing them explain part of the 22% rate?"
  ],
  "decisions_needed": [
    "Enforce the migration => #[sqlx::test] rule as a hard CI gate or as a reviewer-prompt check? — owner: pm-tech-lead + pm-devops",
    "Adopt a standing 'required test level per risk class' table as part of the definition-of-done for dispatcher-implemented PRs? — owner: pm-tech-lead + pm-qa",
    "Set an explicit rework-rate budget (e.g. from-merged-review PRs must stay under 15% of merged PRs) and treat a breach as a stop-the-line signal? — owner: pm-scrum-master + pm-qa"
  ]
}
```

## Notes

- Six pm-qa `next_actions` appended to `action-list.json` with `source = "pm-analysis 2026-08-24"`; all carry `dependency: "none"` / `depends_on: []` (no concrete blocking action-list ids).
- Five pm-qa risks appended to `risks.json`, dedup-checked against existing ids.
- Coverage epic-80 (rotation idx 5) refreshed: all 3 stories confirmed `done`; sprint-status header drift recorded in evidence.
- **Correction to a prior QA read:** the "PRs shipped without tests" hypothesis does not hold — every regressing PR this window shipped tests. Counting test *files* in a diff under-reports Rust inline `#[cfg(test)]` modules; count added `#[test]` / `#[sqlx::test]` / `it(` markers and, more importantly, their *kind*.
- The two stale `partial` stories (84-1, 84-2) cleared this run were both frontend slices that had in fact shipped — the same class of stale-status drift the 2026-07-15 `gap-tracking-stale-status-flips` batch fixed for 80-2. Worth a periodic verification sweep rather than trusting `partial` indefinitely.
