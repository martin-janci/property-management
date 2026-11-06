# pm-scrum-master — Delivery synthesis (2026-11-06)

_Always-on. Routine Phase 1.6 incremental upkeep. Role focus today: **pm-qa** (rotation idx 3 → 4). coverage_cursor idx 0 → 1 (epic-10a re-checked, 3/3 still done, no PRs touched OAuth foundation)._

## Role JSON

```json
{
  "role": "pm-scrum-master",
  "summary": "Massive RLS/authz hardening run — 21 PRs merged (#3318-#3340) consolidating tenant isolation + manager gates across accounting, buildings, certifications, registry, maintenance, platform, voting, faults, IoT, outages, meetings, templates, facilities. All 49 coverage stories remain done; delivery focus has shifted from new epics to defense-in-depth on security + follow-up remediation of post-merge-reviewer findings.",
  "shipped_since_last_run": [
    "#3338 — scope pet/vehicle registration reads to the caller's units",
    "#3332 — round 5 authz B (accounting/buildings/certifications/registry/maintenance/platform, 12 items, regression tests)",
    "#3331 — round 5 authz A (API keys/package/visitor settings/attachments/rentals/news media/Booking.com push)",
    "#3330 — round 5 functional fixes (energy schema, alert rules, seed --force, dashboard layout, outages types, automation i18n; migrations 00241-00243)",
    "#3339 — churn-hotspot-suspend_cascade: Enumerate + Display arms coverage on SuspendCascadeError",
    "#3318-#3328 — 10× gh-issue-* follow-ups (RLS-aware vote reads, require_manager_rls consolidation, stable key order for reality-web messages/*.json, RecentActivity reporter-scope, fault org isolation via SQL, IoT fold-insert, manager-gate automation stats, formatMoney Decimal precision, honest /leases/{leaseId}/payments route, resolution-term responsiblePartyId mapping, member-role escalation via TenantRole::from_role_type)",
    "#3334-#3337, #3340 — screen-map-drift reconciliations (ppt + reality) following prior i18n and regression-test PRs"
  ],
  "sprint_progress": {"sprint": "Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth", "epics_done": 4, "epics_total": 4},
  "next_actions": [
    {"action": "Retire the accounting-server RLS-only by-id methods (noted 'Still open' in #3332) — move accounting reads onto the tenant-id-predicated by-id variants to match #3332's shape across the rest of api-server", "priority": "high", "dependency": "none", "definition_of_done": "accounting-server by-id handlers all consult tenant_id in the predicate; cross-org id returns 404; regression test added."},
    {"action": "Harden EnhancedWebhookSubscription.headers so auth headers cannot be smuggled into the serialized schema (noted 'Still open' in #3332) — reject or strip known auth header names on write", "priority": "high", "dependency": "none", "definition_of_done": "write-path rejects Authorization/Cookie/X-API-Key etc.; read-path omits them from the response schema; test asserts both."},
    {"action": "Apply the ResolvedTenant rule to onboarding so role selection cannot be influenced by a foreign tenant header (noted 'Still open' in #3332)", "priority": "high", "dependency": "none", "definition_of_done": "onboarding role selection consults ResolvedTenant only; 403 on a foreign org header; regression test."},
    {"action": "Design-decide the audit_logs ON DELETE SET NULL FK conflict with the append-only trigger (noted in #3330) — either drop the SET NULL semantics, scope the trigger, or formalize the ACCESS EXCLUSIVE escape hatch for every delete path", "priority": "medium", "dependency": "pm-tech-lead", "definition_of_done": "decision recorded in decisions.md with an owner and a migration plan or a documented ADR."},
    {"action": "Automate the Round-5 regression suite at CI level — add check-rls-enforcement.sh --strict and the round5_sec_b_authz_tests + suite_7 property_cluster_org_scope to the required-checks list on dev", "priority": "medium", "dependency": "pm-devops", "definition_of_done": "CI config updated; a PR that breaks tenant isolation on any listed route fails required checks."}
  ],
  "blockers": [],
  "risks": [
    {"risk": "Three high-impact 'Still open' items from #3332 (accounting by-id, webhook auth-header smuggling, onboarding ResolvedTenant) are flagged in the PR body but have no issue/PR filed — they will rot off the merged-PR body if not captured in the backlog this run", "probability": "high", "impact": "high", "mitigation": "Capture all three as backlog vectors this run; promote the accounting one as the next security plan."},
    {"risk": "Coverage shows 49/49 stories done across 13 epics but the project is still mutating heavily (21 PRs in 24h). The coverage map has drifted from reality — it reflects story-level completion, not security/correctness debt. action-list.json buffer is 25/36 open, under target.", "probability": "medium", "impact": "medium", "mitigation": "Re-run `/ppt-project-management scan` locally to refresh coverage with the current remediation landscape; keep the action-list buffer ≥36 by surfacing post-merge-review follow-ups as queue items."},
    {"risk": "audit_logs ON DELETE SET NULL FK conflict with the append-only trigger (#3330 note) is a landmine — seed --force needs ACCESS EXCLUSIVE lock as an escape hatch; any normal-operation delete path hitting those FKs will 500", "probability": "medium", "impact": "high", "mitigation": "Pm-tech-lead ADR; audit every DELETE path that could touch audit_logs FK; add integration test for the normal-operation delete path."}
  ],
  "open_questions": [
    "Should the three 'Still open' items from #3332 be filed as GitHub issues (per the post-merge-review convention) or promoted as plans directly?",
    "Is dispatch/review cadence still at 2h × 3-concurrent? Buffer is at 25/36 open and shrinking — can the planner surface more gap tasks from coverage?",
    "Does the 'Still open' PR-body convention trigger an `unchecked-todo` signal under the routine's taxonomy? Today it does not (it uses prose, not `- [ ]`) — worth formalizing a `pr-body-still-open` signal type if we keep using this pattern."
  ],
  "decisions_needed": [
    "audit_logs FK semantics — keep ON DELETE SET NULL (and extend the ACCESS EXCLUSIVE escape hatch to every delete path) vs rewrite the FK to CASCADE or NO ACTION — owner: pm-tech-lead + pm-backend",
    "Auth-header smuggling via webhook headers field — which exact header names are rejected on write, and should we also hash + redact on read? — owner: pm-security"
  ]
}
```

## Notes

- `scan_kind=upkeep` this run; coverage_cursor idx 0 → 1 (epic-10a re-checked — all 3 stories still done).
- pm-qa rotation idx 3 → 4; pm-qa last ran 2026-08-25 (73 days stale — long overdue).
- action-list buffer: 25/36 open (not drained, but under target — the planner should add gap tasks on the next scan).
