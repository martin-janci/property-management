# pm-scrum-master

<sub>Last run: 2026-06-27</sub>

## Summary

Sprint 'Epic 6, 7A, 8A & 10A' is materially ahead of its yaml labels: coverage.json (2026-06-23) confirms Epic 6 all 6 stories done, Epic 7A all 5 stories done, Epic 8A done, Epic 10B done — sprint-status.yaml is stale and needs reconciliation. The 11-day catch-up window (PRs #1567-#1856, 96 merges) delivered major work across Epics 3, 4, 6, 11, 16, 18, and messaging, but three high-severity bugs discovered in the saved-search-alerts drainer (Epic 16) and six open test-hardening gates (#480-#485, #487) are blocking story promotion for 10A and Epic 80 partials.

## Next actions

- **[high]** Reconcile sprint-status.yaml: promote Epic 6 (all 6 stories), Epic 7A (all 5 stories), Epic 10B (all 7 stories) to done and flip epic statuses to done to match coverage.json 2026-06-23 ground truth (dep: none; DoD: sprint-status.yaml epic statuses and story development_status entries match coverage.json; sprint_progress reflects accurate epics_done count)
- **[high]** Fix Epic 16 drainer HIGH bugs (row reservation + transactional enqueue) before the next alert worker deployment; assign rust-backend to author the patch and open a PR against dev (dep: pm-backend; DoD: PR merged on dev: SELECT FOR UPDATE skip-locked row reservation present, enqueue+watermark wrapped in single transaction, exponential backoff on retry loop)
- **[high]** Close or defer test-hardening issues #481, #487 (OAuth backend) so 10a-1 and 10a-3 can be promoted; assign rust-backend to write refresh-token revocation regression test and MFA rate-limit test (dep: pm-backend; DoD: Issues #481 and #487 closed; 10a-1 and 10a-3 promoted to done in sprint-status.yaml)
- **[medium]** Close test-hardening issue #482 (ProtectedRoute multi-tenant role fallback) so 10a-2 can be promoted; assign react-web to add ProtectedRoute unit tests covering multi-tenant users (dep: pm-frontend; DoD: Issue #482 closed; 10a-2 promoted to done in sprint-status.yaml)
- **[medium]** Wire party submissions endpoints in ppt-web dispute-detail to unblock 80-3-mediation-resolution; update dispute-detail screen-map apiStatus from partial to complete after merge (dep: pm-frontend; DoD: dispute-detail screen apiStatus=complete; 80-3 promoted to done in sprint-status.yaml)
- **[low]** Update sprint-status.yaml sprint_name and sprint_goal to reflect the active delivery scope (Epics 3, 4, 11, 16, 18, messaging) — current label reflects December 2025 planning, not 2026-Q2 reality (dep: none; DoD: sprint_name and sprint_goal updated; started_at reflects current sprint start date)

## Risks

- **high/high**: Epic 16 saved-search-alerts drainer has two HIGH-severity concurrency bugs (duplicate emails/pushes under load, data loss on crash) that are already deployed to dev on merged PRs #1847-#1850 — mitigation: Expedite rust-backend patch with SELECT FOR UPDATE skip-locked + single-transaction enqueue; do not promote Epic 16 to done or cut a release including this code until patch merges
- **high/medium**: sprint-status.yaml is 5+ weeks stale (last updated 2026-05-25) — team may make planning decisions off incorrect 'in-progress' counts, masking 3 fully-done epics and inflating the backlog — mitigation: Orchestrator should write the reconciled sprint-status.yaml immediately as part of this run's artifact output
- **medium/high**: Six open test-hardening gates (#480-#485, #487) with no assigned owner or due date — OAuth stories (10a-1/2/3) stuck in ready-for-dev indefinitely — mitigation: Assign specific owner roles and target sprint for each gate; consider deferring low-severity ones (e.g. #483 voice-device IDOR tests) to reduce block count
- **medium/high**: 11-day cursor lag in the daily research routine means 75+ post-merge reviewer issues accumulated without triage — security-themed follow-ups (sqlx checks, idempotency keys, IDOR) may age into real vulnerabilities — mitigation: Restore daily routine cadence; pm-security to triage the 75 'follow-up + from-merged-review' issues and promote actionable ones to the backlog
- **medium/medium**: saved_search_alerts.rs and reality_portal.rs are churn hotspots (3 touches each in this run) — continued parallel edits risk merge conflicts and regression in a code path already known to have concurrency bugs — mitigation: Serialize drainer work through a single branch; add integration test coverage before the next feature touch

## Open questions

- Were the Epic 16 drainer bugs (duplicate alerts, crash-restart duplicates) introduced by PRs #1847-#1850 or were they pre-existing? Need commit-level blame to scope the fix.
- Issue #480 (WebSocket JWT token in query param logged) is marked open and severity=high — has any security review been done or is this still unmitigated in production?
- Is Epic 10A (OAuth Provider Foundation) still in-scope for the current sprint, or has it been deferred to a future sprint given the test-hardening blockers?
- What is the correct started_at date for the current active sprint — the yaml shows 2025-12-21 but delivery pace suggests a re-plan occurred in Q1/Q2 2026?
- PR #1821 (accounting epic) is non-draft and open — is this Epic 11 follow-on work, and who is the reviewer?

## Decisions needed

- Decide whether to defer test-hardening issues #483 (voice-device IDOR tests) and #484 (notification dispatch serial/FCM stub) to a hardening sprint rather than blocking OAuth story promotion — owner: pm-tech-lead
- Decide whether Epic 16 saved-search-alerts code should be feature-flagged off in staging until the drainer concurrency patch lands — owner: pm-backend
- Decide the scope and owner for the 5-step dispute-filing wizard redesign (80-2 redesignStatus: in-progress) — currently no PR in flight — owner: pm-frontend
