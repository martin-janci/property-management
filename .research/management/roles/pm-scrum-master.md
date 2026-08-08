# pm-scrum-master — Delivery lead / coordinator (2026-08-08)

_Always-on role. Read-only static analysis of sprint-status + merged PRs + open PRs + issues + research backlog._

## Role JSON

```json
{
  "role": "pm-scrum-master",
  "summary": "6 PRs merged this window — mostly security/DoS hardening (workflow api_call cap, RAG fail-closed, non-finite condition compare) plus a dispute audit-event backfill and a reality-web i18n fix. Delivery is otherwise stable at 47/49 coverage stories done; the two open blockers are process, not code: an 8-day-stalled accounting trio and a stale sprint-status.yaml rollup for epic-80.",
  "next_actions": [
    {"action": "Shepherd/unblock accounting MVP-loop trio (#2555 invoice lifecycle, #2558 invoice PDF, #2559 PAY-by-square QR) — stalled 8+ days with zero reviewer engagement", "priority": "high", "dependency": "none", "definition_of_done": "At least one of the three has an assigned reviewer and a merge/defer decision within 24h."},
    {"action": "Resolve CI-red on draft PR #2705 (dependabot rust-toolchain 1.94.1→1.100.0)", "priority": "medium", "dependency": "none", "definition_of_done": "PR is green and merged, or closed with a documented reason (unsafe-to-bump this cycle)."},
    {"action": "Reconcile sprint-status.yaml epic-80 summary (stories_completed: 1/3) against coverage.json and per-story development_status, all of which show 80-1/80-2/80-3 done", "priority": "medium", "dependency": "none", "definition_of_done": "epics.epic-80.stories_completed = 3 and status flipped to done (or a documented reason it stays partial)."},
    {"action": "Batch-triage the 12 pending dependabot chore(deps) PRs (#2588/#2589/#2590/#2591 stalled 8d expo-*; #2581/#2583/#2584/#2585/#2586/#2673/#2674/#2675 fresh 1d)", "priority": "low", "dependency": "none", "definition_of_done": "Each PR is merged, closed, or has an explicit defer note with a re-review date."},
    {"action": "Follow-up hardening audit on services/actions/api_call.rs beyond #2707 (response-body cap) and #2710 (SSRF DNS-rebinding) — repeated churn hotspot on the workflow outbound-HTTP surface", "priority": "medium", "dependency": "none", "definition_of_done": "A short audit note lists any remaining outbound-HTTP hardening gaps (redirect handling, header injection, timeout bounds) or confirms none remain."},
    {"action": "Audit sibling reality-web forms for the same missing-i18n pattern just fixed in ListingForm by PR #2709", "priority": "low", "dependency": "none", "definition_of_done": "Every reality-web form component is confirmed to route strings through next-intl catalogs, or a gap list is filed."}
  ],
  "risks": [
    {"risk": "Accounting MVP-loop trio (#2555/#2558/#2559) has been draft-ready 8+ days with zero reviewer engagement — the MVP revenue-loop feature set cannot progress", "probability": "high", "impact": "high", "mitigation": "Assign an explicit reviewer slot; escalate to pm-tech-lead for a reviewer-rotation policy"},
    {"risk": "sprint-status.yaml epic-80 rollup is stale (1/3) vs coverage.json (3/3 done) — risks mis-sequencing planning off the wrong source of truth", "probability": "medium", "impact": "medium", "mitigation": "Reconcile the epic-80 summary block this window"},
    {"risk": "12 dependabot chore(deps) PRs pending with no decision (4 stalled 8d) — security/dependency patches accumulate un-landed", "probability": "medium", "impact": "medium", "mitigation": "Batch-triage this window; consider a weekly dependency-triage cadence"},
    {"risk": "Reviewer-slot policy raised 2026-07-30 for the same accounting trio was never actioned — the gap has grown from 2 days to 8+ days unaddressed", "probability": "medium", "impact": "medium", "mitigation": "Treat this decision as blocking, not advisory — set a hard 24h SLA this time"}
  ],
  "open_questions": [
    "Is the accounting trio still product-prioritized for this MVP window, or has it slipped scope silently by starving on review?",
    "Does PR #2705's CI-red reflect a genuine incompatibility with the pinned rust-version = 1.75 toolchain floor, or a transient CI issue?"
  ],
  "decisions_needed": [
    "Accounting MVP-loop trio (#2555/#2558/#2559): assign an explicit reviewer now vs formally deprioritize — owner: pm-tech-lead",
    "Draft PR #2705 (rust-toolchain bump): fix-forward vs close as unsafe-to-bump this cycle — owner: pm-devops",
    "sprint-status.yaml epic-80 rollup correction — who owns editing the YAML directly vs re-flagging it each rotation — owner: pm-scrum-master"
  ],
  "shipped_since_last_run": [
    "#2712 feat(pm-data): dispute add_evidence access-audit event",
    "#2711 refactor: dedupe layout tenant-override handlers",
    "#2709 fix(reality-web): i18n ListingForm via next-intl catalogs",
    "#2707 fix(api-server): cap workflow api_call response-body read (8 MiB) — closes #2704 (memory-amplification DoS)",
    "#2706 fix(api-server): fail closed on partial RAG embedding batch",
    "#2708 fix(api-server): reject non-finite numbers in workflow condition compare"
  ],
  "sprint_progress": {"sprint": "Epic 6, 7A, 8A & 10A - Announcements, Documents, Notifications & OAuth", "epics_done": 2, "epics_total": 6},
  "blockers": [
    {"item": "Accounting MVP-loop trio (#2555/#2558/#2559)", "reason": "8+ days, zero reviewer engagement", "owner_role": "pm-tech-lead"},
    {"item": "Draft PR #2705 (rust-toolchain bump)", "reason": "CI red since opened", "owner_role": "pm-devops"},
    {"item": "sprint-status.yaml epic-80 rollup", "reason": "stale vs coverage.json (1/3 vs 3/3 stories done)", "owner_role": "pm-scrum-master"}
  ]
}
```

## Notes

- `sprint_progress.epics_done/epics_total` (2/6) is computed directly from `_bmad-output/implementation-artifacts/sprint-status.yaml`'s `epics:` block (epic-6, epic-7a, epic-8a, epic-10a, epic-10b, epic-80 — done: epic-8a, epic-10a). This differs from the broader `coverage.json` extended-scope view (47/49 stories done across 13 epics) — the two numbers answer different questions (core-sprint epics vs full delivered-story inventory) and both are reported to avoid conflating them.
- Cross-referenced the 6 merged PRs against `coverage.json`: only #2712 (epic-80 story 80-1) and #2706 (epic-84 story 84-5) map to a tracked story; #2711/#2709/#2707/#2708 are infra/security hardening outside the tracked story set and were not force-mapped.
- Six `next_actions` appended to `action-list.json` with `source = "pm-analysis 2026-08-08"`. Four risks dedup-checked against existing risk IDs and appended to `risks.json`.
- Flagged the epic-80 sprint-status.yaml drift as a recurring theme: coverage.json has shown 80-1/80-2/80-3 as done since 2026-06-25/07-04/07-15 respectively, but the epic-level `stories_completed: 1` rollup was never updated — this is a documentation-hygiene gap, not a delivery gap.
- The accounting-trio reviewer-starvation risk was already raised on 2026-07-30 (2 days stalled then); it has now compounded to 8+ days without resolution — escalating from "risk" to "decision needed" this run.
