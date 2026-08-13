# pm-scrum-master — Delivery lead (2026-08-13)

_Always-on. Runs every research routine invocation._

## Role JSON

```json
{
  "role": "pm-scrum-master",
  "summary": "Quiet 4-hour window: zero PRs merged, one new issue (#2743) that reproduces two previously-fixed dispatcher defects and is choking the terminal-transition/claim-buffer loop. Story-level work for the active sprint is effectively complete (development_status shows nearly all epic-6/7a/8a/10a/10b/80 stories done) but epic-level status fields in sprint-status.yaml haven't been reconciled to match, and coverage.json shows only 2 remaining mvp gaps, both frontend (84-1, 84-2).",
  "next_actions": [
    {"action": "Fix dispatcher archive-oversize regression (issue #2743): shard or prune .research/management/*-archive.json (~638KiB/~660KiB) back under the 64KiB MCP push ceiling", "priority": "high", "dependency": "infra/pm-tech-lead", "definition_of_done": "both archive files push cleanly under ceiling; dispatcher terminal transitions unblock"},
    {"action": "Fix retry-remint.sh dedup so it doesn't re-mint tasks (e.g. code-review-api-handlers-voice-webhook-default-secret) already landed on dev under a different id (PR #2660)", "priority": "high", "dependency": "infra/pm-tech-lead", "definition_of_done": "retry-remint checks merged-PR history before re-minting; regression covers the #2460 case"},
    {"action": "Wire ppt-web direct-to-S3 upload flow to POST /documents/upload-url (story 84-1-s3-presigned-urls, mvp, partial)", "priority": "high", "dependency": "pm-frontend", "definition_of_done": "UploadDocument component uses the presigned-URL flow; ppt/upload-document screen-map apiStatus flips partial->complete"},
    {"action": "Build the signer-facing document-sign page in ppt-web (story 84-2-esignature-email, mvp, partial) — scope tightly, prior attempt (gap-84-2) failed with no PR", "priority": "high", "dependency": "pm-frontend", "definition_of_done": "ppt/document-sign screen-map buildStatus planned->shipped; e2e signature-request email verified"},
    {"action": "Find an alternate landing path for the 4 structurally-unlandable KMP items sitting in the claim buffer (cloud runner cannot build mobile-native)", "priority": "medium", "dependency": "pm-mobile-native", "definition_of_done": "KMP items routed to a runner that can build them, or explicitly deferred with a tracked follow-up"},
    {"action": "Reconcile epic-level status/story-count fields in sprint-status.yaml (epic-6, epic-7a, epic-10b, epic-80) against the already-done story-level detail", "priority": "medium", "dependency": "none", "definition_of_done": "epics block matches development_status truth; sprint reported as accurately near-complete"}
  ],
  "risks": [
    {"risk": "Dispatcher archive files recurring over the MCP push ceiling blocks all terminal transitions, stalling the entire research-to-implementation loop", "probability": "high", "impact": "high", "mitigation": "shard/prune archives now and add a size-guard check to prevent recurrence (this is the second time — was #1162)"},
    {"risk": "retry-remint ghost-retries waste cycles re-doing work already merged under a different task id, masking true backlog depth", "probability": "medium", "impact": "medium", "mitigation": "cross-check merged-PR history before re-minting; add regression for the #2460 pattern"},
    {"risk": "Claim buffer is structurally starved (4 KMP-unlandable + 1 stem-blocked by quarantined #2684 + 1 closed-not-merged) — throughput drops to near zero even once dispatcher infra is fixed", "probability": "high", "impact": "medium", "mitigation": "route KMP work to a capable runner; resolve or drop #2684 quarantine; re-triage the closed-not-merged retry"},
    {"risk": "Stalled acc-track PRs #2555/#2558/#2559 (15d open, 13d idle) risk growing merge conflicts / scope drift the longer they sit unreviewed", "probability": "medium", "impact": "medium", "mitigation": "assign a reviewer decision this cycle: merge, request changes, or close"},
    {"risk": "Epic-level sprint-status.yaml fields are stale relative to story-level done markers, risking inaccurate stakeholder-facing sprint reporting", "probability": "low", "impact": "medium", "mitigation": "batch-reconcile epic block against development_status"}
  ],
  "open_questions": [
    "Is the archive-oversize fix intended to be a durable structural fix (permanent sharding/rotation) this time, given it already recurred once (#1162 -> #2743)?",
    "Who owns retry-remint.sh going forward, and should it consult PR/merge history before re-minting?",
    "Is there a plan/owner for a non-cloud runner to land the 4 structurally-unlandable KMP items?",
    "What is the disposition of quarantined PR #2684 (workflow-cond-parse-failopen, in review >5d) — rework, reassign, or drop?",
    "Should the sprint be formally closed/renamed given story-level work for epics 6/7a/8a/10a/10b/80 is already done, leaving only two mvp frontend gaps (84-1/84-2) which are epic-84, not part of the named sprint?"
  ],
  "decisions_needed": [
    "Archive-oversize remediation strategy (shard vs raise ceiling vs prune) — owner: pm-tech-lead",
    "Routing plan for cloud-unlandable KMP backlog items — owner: pm-mobile-native",
    "Disposition of stalled PRs #2555/#2558/#2559 (merge/changes/close) — owner: pm-tech-lead",
    "Disposition of quarantined PR #2684 — owner: pm-tech-lead"
  ],
  "shipped_since_last_run": [],
  "sprint_progress": {"sprint": "Epic 6, 7A, 8A & 10A - Announcements, Documents, Notifications & OAuth", "epics_done": 2, "epics_total": 6},
  "blockers": [
    {"item": "Dispatcher infra (issue #2743)", "reason": "Both .research/management/*-archive.json files again exceed the 64KiB MCP push ceiling (recurrence of #1162), blocking dispatcher terminal transitions for the whole research-to-implementation loop", "owner_role": "pm-tech-lead"},
    {"item": "Claim buffer", "reason": "Starved of landable work: 4 KMP items structurally unlandable in the cloud runner, 1 stem-blocked by quarantined PR #2684, 1 investigative item closed-not-merged", "owner_role": "pm-mobile-native"},
    {"item": "PR #2684 (workflow-cond-parse-failopen)", "reason": "Quarantined, in review >5 days, stem-blocking a backlog item", "owner_role": "pm-tech-lead"}
  ]
}
```

## Digest highlights

- **Sprint:** Epic 6, 7A, 8A & 10A - Announcements, Documents, Notifications & OAuth — 2/6 epics done at the rolled-up level; story-level truth is much further ahead.
- **Shipped since last run:** nothing (4-hour quiet window).
- **Blockers:** dispatcher-archive oversize (#2743), starved claim buffer (KMP), quarantined PR #2684.
- **Role focus today:** pm-scrum-master + pm-qa.
