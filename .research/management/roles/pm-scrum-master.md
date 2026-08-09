# Role: pm-scrum-master — 2026-08-09

> Delivery lead / coordinator. Always runs. Static read-only.

## Summary
Explosive merge window: 18 PRs merged since #2702 in a single 24h burst (2026-08-07T20 -> 08-08T20), including 4 security fixes (SSRF #2710, DoS cap #2707, NaN reject #2708, community-read IDOR #2722), 2 admin PATCH endpoints (#2716/#2717), and 3 issue closures (#2703 SSRF, #2704 community-reads, #2612 notification retry). Coverage still at 47/49 done; the two 84-x frontend partials are unchanged. Dispatcher buffer critically low (claimable=1/72) — refill is priority-one and #2684 (workflow_cond_parse_failopen) is now quarantined per dispatcher after fix_rounds=3 exhausted.

## Sprint progress
- Sprint: **Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth**
- epics_done: **3 / 5**

## Shipped since last run (18 PRs)
- PR #2706 fix RAG index partial-batch failing closed
- PR #2707 DoS cap on workflow api_call body
- PR #2708 workflow_executor NaN reject
- PR #2709 reality-web ListingForm i18n
- PR #2710 SSRF DNS-rebinding TOCTOU fix (closes #2703)
- PR #2711 layout tenant refactor
- PR #2712 dispute add_evidence audit event
- PR #2713 layout admin refactor
- PR #2714 scheduled notification retry (closes #2612)
- PR #2715 auth boilerplate refactor
- PR #2716 platform-admin settings PATCH endpoint
- PR #2717 admin mobile-config PATCH endpoint
- PR #2718 layout webhook HMAC parity verified (no-fix outcome)
- PR #2719 inquiry send routed through InquiriesHandler
- PR #2720 reports helpers extracted
- PR #2721 acquire_public_conn extracted
- PR #2722 community read handlers gated on principal+tenant (closes #2704, SECURITY IDOR)
- PR #2723 announcement fan-out delivered/read/ack metrics

## Next actions
1. **[high]** Finish 84-1 direct-to-S3 upload wiring in ppt-web — api-client binding + UploadDocument integration + regression test — dependency: none — DoD: ppt-web uploads via presigned URL end-to-end; regression test asserts 200 + object in S3
2. **[high]** Build signer-facing document-sign page (84-2) in ppt-web against shipped signing API; verify signature-request email end-to-end — dependency: none — DoD: screen-map ppt/document-sign flips buildStatus=shipped; email delivery verified
3. **[high]** Human triage of quarantined PR #2684 (workflow_cond_parse_failopen) — CI test-shards 1-4 red after 3 respawns; classify as flake vs real regression — dependency: none — DoD: root cause identified; fix landed or task re-scoped/closed
4. **[medium]** Shepherd 3 in-review PRs (#2724 db-error-leak, #2725 password-reset, #2726 sso-session-invalidate) — all draft-ready day-0 — dependency: none — DoD: reviewer verdict + merge or changes; assignments.json advanced
5. **[medium]** Investigate churn on routes/reports/mod.rs + services/scheduler/mod.rs + routes/integrations/webhook.rs — top-3 hotspots this window — dependency: none — DoD: each hotspot has a refactor RFC or is flagged as one-shot
6. **[high]** Refill dispatcher backlog buffer to ≥36 open — claimable dropped to 1/72 after this window's merge surge — dependency: none — DoD: action-list.json ≥36 open items post-run; roadmap.md refreshed

## Risks
- **[medium prob / medium impact]** PR #2684 (workflow_cond_parse_failopen) quarantined after 3 fix rounds — CI test-shards RED; the workflow_executor edge case may be hiding a real regression — mitigation: Human investigation before re-queueing; split scope if test-shard boundary is the issue
- **[high prob / medium impact]** Dispatcher buffer starved (claimable=1/72) — implementer queue idle within 2h without refill — mitigation: This refill run adds >=30 new candidates from coverage gaps + post-merge follow-ups
- **[medium prob / high impact]** Merge pace of 18 PRs in 24h with 4 security-adjacent (SSRF, IDOR, DoS, NaN) exceeds review depth — regressions could ship undetected — mitigation: pm-qa scheduled a security regression sweep next window; enforce co-committed regression test for security fixes
- **[high prob / medium impact]** 3 in-review reality-server retry PRs (#2724/#2725/#2726) all report 'verify gate unrunnable in cloud' (utoipa-swagger-ui egress blocked) — retry loop is running blind for CI-only signals — mitigation: Mirror utoipa-swagger-ui build deps into cloud cache OR make verify gate skip-with-report on cloud

## Open questions
- Should coverage.json add the accounting MVP-loop epic (currently outside the 13-epic set)?
- For 3 in-review reality-server PRs blocked from local verify, should reviewer wait on GitHub Actions or shift to merge-then-monitor?

## Decisions needed
- Quarantined-PR retirement policy — after fix_rounds exhausted, does the task auto-close, escalate to human, or re-queue at lower priority? — owner: pm-tech-lead
- Post-merge security review cadence given 4 security-adjacent PRs in one 24h window — owner: pm-security

## Blockers
- **PR #2684 code-review-api-core-workflow-cond-parse-failopen** — fix_rounds=3 exhausted; CI test-shard(1-4) FAILURE at 926afe8 after 3 respawns — owner: pm-tech-lead
- **84-1 direct-to-S3 wiring (long-standing partial)** — prior implementer failed no-PR; needs fresh attempt — owner: pm-frontend
- **84-2 signer page (long-standing partial)** — prior implementer failed no-PR; scope to shipped API — owner: pm-frontend
