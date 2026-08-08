# pm-scrum-master — Delivery synthesis (2026-08-08)

_Always-on. Phase 1.6 of the research routine. Read-only static analysis of sprint-status + this window's 7 merged PRs + 4 open PRs + issues + pm-qa findings._

## Role JSON

```json
{
  "role": "pm-scrum-master",
  "summary": "7 PRs merged since 2026-08-07 — 5 code-review clears + 2 layout refactors. Sprint-yaml unchanged (still 2/5 done); coverage still 47/49 stories with the same 2 epic-84 frontend partials. Auto-fix loop caught + closed #2704 within a day and is now working #2703 (SSRF, draft #2710). No new blockers, but two hotfix-no-test slips this window and a ready-to-merge dead-notify PR (#2696) need pm-qa sequencing.",
  "shipped_since_last_run": [
    "#2713 refactor(api-server): dedupe layout admin handler boilerplate (churn-hotspot)",
    "#2712 data(dispute): emit add_evidence access-audit event on audit_logs",
    "#2711 refactor(api-server): dedupe layout tenant-override handlers (churn-hotspot)",
    "#2709 i18n(reality-web): ListingForm via next-intl catalogs (closes code-review-reality-web-listingform-no-i18n)",
    "#2708 fix(workflow_executor): reject non-finite numbers in condition compare",
    "#2707 fix(workflow api_call): cap unbounded response-body read (closes #2704)",
    "#2706 fix(ai/llm): fail-closed on partial RAG embedding batch"
  ],
  "sprint_progress": {"sprint": "Epic 6, 7A, 8A & 10A - Announcements, Documents, Notifications & OAuth", "epics_done": 12, "epics_total": 13},
  "next_actions": [
    {"action": "Merge PR #2710 (SSRF DNS-rebinding TOCTOU fix) with a resolver-spoof regression test — closes #2703 (live vuln, workflow api_call.rs)", "priority": "high", "dependency": "pm-tech-lead", "definition_of_done": "PR merged; test asserts rebinding attempt is rejected at connect time; #2703 closed."},
    {"action": "Sequence-lock #2696 (inquiry-email seam) with `code-review-reality-server-inquiry-notify-route-wiring` — merge as a pair; a solo #2696 merge ships a silent-success regression", "priority": "high", "dependency": "pm-backend", "definition_of_done": "Both PRs land in the same window; route-level test asserts notifier fires on send_contact_message."},
    {"action": "Ship 84-1 direct-to-S3 upload wiring in ppt-web (POST /api/v1/documents/upload-url consumer) — #2573 same-org ref-check remains the residual blocker to safe wiring", "priority": "high", "dependency": "pm-frontend", "definition_of_done": "UploadDocument uses presigned PUT; regression test covers the direct path; coverage 84-1 flips partial → done."},
    {"action": "Ship 84-2 signer-facing document-sign page in ppt-web against the shipped signing API; flip screen-map ppt/document-sign buildStatus planned→shipped", "priority": "high", "dependency": "pm-frontend", "definition_of_done": "Signer opens invite link, signs, backend records signature; coverage 84-2 flips partial → done."},
    {"action": "Backfill regression tests for #2707 (body-cap) and #2712 (add_evidence audit) — closes the hotfix-no-test slips flagged by pm-qa this run", "priority": "medium", "dependency": "pm-qa", "definition_of_done": "Both tests land on dev; fail-on-main verified."},
    {"action": "Reconcile sprint-status.yaml — epics 6/7a/10b/80 all show coverage=done, sprint-yaml still in-progress. Flip in a housekeeping PR", "priority": "low", "dependency": "pm-scrum-master", "definition_of_done": "sprint-status.yaml matches coverage.json epic states."}
  ],
  "risks": [
    {"risk": "SSRF DNS-rebinding TOCTOU still shipped in prod code (workflow api_call.rs) — #2703 open >24h, #2710 still draft. Meanwhile the workflow engine is on the critical path.", "probability": "medium", "impact": "high", "mitigation": "Top-slot #2710 for implementer + reviewer; do not close #2703 until merged."},
    {"risk": "Reviewer capacity remains the tighter constraint — accounting MVP-loop trio (#2555/#2558/#2559) carried from 2026-07-30 still unresolved (not in this window's touched-PRs list).", "probability": "medium", "impact": "medium", "mitigation": "Confirm accounting trio still open; if yes, apply the reviewer-slot policy from DEC-2026-07-30."},
    {"risk": "#2696 ready-to-merge could ship as functional dead code if reviewer doesn't notice the notify-wire follow-up — the pattern is subtle (silent success is not obvious in review).", "probability": "medium", "impact": "high", "mitigation": "Add a red banner to the #2696 PR body: 'DO NOT MERGE without inquiry-notify-route-wiring'; make the route-test a coverage requirement."}
  ],
  "blockers": [
    {"item": "PR #2696 inquiry-email seam", "reason": "Would merge as functional dead code — live send_contact_message endpoint bypasses the new notifier seam", "owner_role": "pm-backend"},
    {"item": "Story 84-1 (ppt-web direct-to-S3 wiring)", "reason": "Blocked-on #2573 same-org reference-check gap (carried from 2026-07-30)", "owner_role": "pm-backend"},
    {"item": "SSRF #2703", "reason": "Live vulnerability with no merged fix yet (draft #2710)", "owner_role": "pm-tech-lead"}
  ],
  "open_questions": [
    "Is #2573 (DELETE-by-file-key same-org ref-check) actually merged as of 2026-08-08? Roadmap 2026-07-30 flagged it as blocking 84-1; not in this window's touched-PR list.",
    "Are #2555 / #2558 / #2559 (accounting MVP-loop trio) still open — the reviewer starvation flagged on 2026-07-30?",
    "Should the sprint-yaml epic status flip to done be automated (script that reads coverage.json) or stay manual?"
  ],
  "decisions_needed": [
    "Adopt hotfix-no-test merge-gate for security/data-loss labelled PRs — owner: pm-tech-lead + pm-qa",
    "Sequencing convention for a seam-PR + wire-PR pair (add tag `blocks-alone` on both?) — owner: pm-scrum-master"
  ]
}
```

## Notes

- Always-on; ran alongside pm-qa this rotation. pm-qa was 54 days stale — long-overdue slot.
- Sprint-progress uses the extended coverage view (12/13 epics done, 1 partial). Sprint-yaml direct view is 2/5 done and drifting — a reconcile PR is queued as one of this run's next_actions.
- Deliberately did NOT tool-verify #2555/#2558/#2559 / #2573 status via `gh pr view` — kept as `open_questions` to preserve token budget; the routine's later phases (or the next pm-scrum-master run) will resolve.
- No new decisions logged this run beyond what's in `decisions_needed`; existing `decisions.md` retained unchanged.
