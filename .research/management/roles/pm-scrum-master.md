# pm-scrum-master — Delivery synthesis (2026-08-24)

_Always-on. Window 2026-08-22T03:03:20Z → 2026-08-24T03:12:08Z. Read-only._

## Headline

The coverage map closed at **49/49 done** this run — but the milestone is hollow, because 8 of the 13 PRs
merged this window touch code that map has never described. Delivery is no longer constrained by unfinished
stories; it is constrained by **planning inputs** (an exhausted coverage map) and **review capacity**
(a 26-day-stale accounting trio, and a pre-merge gate that let two same-window regressions through).

## Role JSON

```json
{
  "role": "pm-scrum-master",
  "summary": "Coverage closed at 49/49 done after clearing 84-1 and 84-2 as stale-partial, so the gap-driven ranker has zero story candidates and the action-list buffer sits at 20/36; meanwhile 5 of 13 merged PRs were from-merged-review follow-ups and the UC-ACC-05 accounting trio has been open 26 days with no reviewer engagement.",
  "shipped_since_last_run": [
    "#2821 — gate direct-connect OTA credential writes on manager role (booking connect non-manager hijack)",
    "#2825 — gh-issue-2824: i18n VerificationBadge expiry copy + de-duplicate expiry logic",
    "#2826 — gh-issue-2823: per-channel bookkeeping + bounded retry for held-notification drain",
    "#2827 — gh-issue-2822: neutralize CR/LF in CSV export sanitizer",
    "#2828 — surface ppt-web facilities booking fetch/approve/reject/cancel errors",
    "#2829 — replace AML dashboard prompt/alert decision flow with in-app dialogs + i18n",
    "#2830 — i18n facilities booking UI strings",
    "#2833 — gh-issue-2832: reset AML EDD/Review dialog state per assessment",
    "#2834 — gh-issue-2831: atomic claim so quiet-hours drain delivers held notifications at-most-once across replicas",
    "#2835 — run VoteDetailScreen hooks before the voteId early return (mobile RN)",
    "#2836 — localize ThreadDetailScreen UI strings (mobile RN)",
    "#2837 — localize voice-assistant confirmation/error strings (mobile RN)",
    "#2838 — centralize voice OAuth token encryption (churn hotspot voice_webhooks.rs)",
    "coverage: 84-1 (S3 presigned URLs) and 84-2 (e-signature page) confirmed shipped and flipped partial -> done"
  ],
  "sprint_progress": {"sprint": "Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth", "epics_done": 3, "epics_total": 5},
  "next_actions": [
    {"action": "Run the LOCAL /ppt-project-management scan to rebuild coverage.json — the 49-story map is fully done and 8 of 13 merged PRs map to no story at all", "priority": "high", "dependency": "none", "definition_of_done": "coverage.json regenerated with scan_kind=deep against the current epics catalog; ranker produces >0 story candidates."},
    {"action": "Resolve the UC-ACC-05 accounting trio #2555 / #2558 / #2559 — assign a reviewer and merge, or close and re-plan", "priority": "high", "dependency": "none", "definition_of_done": "All three PRs merged or closed; if closed, the accounting slice is re-queued as dispatcher tasks."},
    {"action": "Adopt the pm-qa pre-merge test-level gate (migration => #[sqlx::test]; dialog state => remount test) as definition-of-done for dispatcher-implemented PRs", "priority": "high", "dependency": "none", "definition_of_done": "pr-reviewer-prompt.md carries the risk-class -> test-level table; next window's from-merged-review rate is measured."},
    {"action": "Batch-triage the 13 open dependabot PRs against dev so open-PR count reflects real human work", "priority": "medium", "dependency": "none", "definition_of_done": "Green dependabot PRs merged; digest reports human-authored open-PR count separately."},
    {"action": "Decide on draft PR #2744 (dispatcher un-wedge, 10 days, still DRAFT) — the issue it tracked, #2743, is now closed", "priority": "medium", "dependency": "none", "definition_of_done": "PR #2744 merged or closed."},
    {"action": "Reconcile sprint-status.yaml epics.epic-80 header (status=partial / stories_completed=1) with its own development_status (80-1/80-2/80-3 all done)", "priority": "low", "dependency": "none", "definition_of_done": "Header and development_status agree; epic-80 reads done."}
  ],
  "risks": [
    {"risk": "coverage.json reached 49/49 done, so gap-driven planning yields zero story candidates and the action-list buffer sits at 20/36 open with the shortfall unfillable from coverage. The dispatcher's claim buffer will starve within a few runs.", "probability": "high", "impact": "medium", "mitigation": "Run the local deep scan; let the Tier-1d dev-review generator carry refill in the interim."},
    {"risk": "The UC-ACC-05 accounting trio (#2555/#2558/#2559) is 26 days open with zero reviewer engagement and no commits since the last run; the whole accounting/invoice MVP loop is frozen behind it and it has been the named top blocker since 2026-07-30 without movement.", "probability": "high", "impact": "high", "mitigation": "Assign a named reviewer this run and merge, or close all three and re-plan."},
    {"risk": "13 of 17 open PRs are dependabot and untouched; real human-authored work is 4 stale PRs, invisible in the raw open-PR count used for delivery signalling.", "probability": "medium", "impact": "low", "mitigation": "Batch-merge green dependabot PRs and report human-authored open-PR count separately."}
  ],
  "blockers": [
    {"item": "UC-ACC-05 accounting trio (#2555, #2558, #2559)", "reason": "26 days open, zero reviewer engagement, untouched since last run — the accounting/invoice MVP loop cannot advance", "owner_role": "pm-tech-lead"},
    {"item": "coverage.json planning inputs", "reason": "49/49 done leaves the gap ranker with no story candidates while 8 of 13 merged PRs fall outside the map; buffer at 20/36 and unfillable from coverage", "owner_role": "pm-tech-lead"},
    {"item": "Pre-merge review gate", "reason": "5 of 13 merged PRs were from-merged-review follow-ups; #2826 and #2829 each regressed inside 48h, so two files were patched twice in one window", "owner_role": "pm-qa"}
  ],
  "open_questions": [
    "Is there a human reviewer assigned to the accounting trio at all, or is it waiting on a queue nobody owns?",
    "Should the coverage map be re-derived from the epics catalog, or extended by hand to cover the AML/compliance, facilities-booking and voice-assistant surfaces that are absorbing most of the churn?",
    "Draft PR #2744 predates the closure of #2743 — does it still contain unique fixes, or was it superseded?"
  ],
  "decisions_needed": [
    "Run the deep coverage scan locally this week, or accept dispatcher-generated dev-review findings as the sole buffer source? — owner: pm-tech-lead",
    "Close-and-replan vs review-and-merge for the 26-day accounting trio? — owner: pm-tech-lead + pm-scrum-master",
    "Adopt the pm-qa required-test-level table as definition-of-done for dispatcher PRs? — owner: pm-tech-lead + pm-qa"
  ]
}
```

## Notes

- Five scrum-master `next_actions` appended to `action-list.json` (`source = "pm-analysis 2026-08-24"`); the sixth (test-level gate) is carried by the pm-qa rows rather than duplicated.
- Two new scrum-master risks appended; the carried accounting-trio risk was **refreshed in place** (probability/impact raised to high/high, trigger set) rather than duplicated by slug.
- Eight action-list rows closed this window from merged PRs and closed issues (see `resolution` field on each).
