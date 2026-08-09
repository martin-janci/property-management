# PPT Project State

_Generated: 2026-08-09 — routine Phase 1.6 lightweight upkeep (pm-qa rotation slot, coverage epic-80 upkeep). Sprint window 2026-08-07T20 -> 08-08T20 shipped **18 PRs** in a single 24h merge burst; dispatcher buffer critically low (claimable=1/72) triggered this refill. pm_cursor 3 -> 4 (pm-qa -> pm-devops next); coverage_cursor 5 -> 6 (epic-80 re-checked -> epic-81 next)._

## Executive summary

- **Delivery unchanged at 47/49 stories done, 2 partial** (84-1 direct-to-S3 wiring, 84-2 signer page). The 2026-08-07T20 -> 08-08T20 window shipped **18 PRs** — a broad mix of security (SSRF #2710, DoS cap #2707, NaN reject #2708, community IDOR #2722), refactors (auth #2715, layout tenant/admin #2711/#2713, reports helpers #2720, acquire_public_conn #2721), features (announcement fan-out metrics #2723, admin PATCH endpoints #2716/#2717), and 1 verification-only PR (#2718 layout webhook HMAC parity confirmed, no fix needed).
- **3 issues closed by this window:** #2703 (SSRF) via #2710, #2704 (community IDOR) via #2722, #2612 (notification retry) via #2714.
- **1 PR quarantined:** #2684 (workflow_cond_parse_failopen) — CI test-shard(1-4) RED at 926afe8 after 3 respawns; fix_rounds=3 exhausted per dispatcher.
- **3 draft PRs open, day-0:** #2724 (db-error-leak), #2725 (password-reset transport), #2726 (sso-session-invalidate) — all from the pm-tech-lead retry queue; all reported 'verify gate unrunnable in cloud' (utoipa-swagger-ui egress blocked).
- **Buffer refilled 2 -> 49 open** (60 items total: 3 in-progress, 7 done this run, 1 failed/quarantined).

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth** · **epics_done = 3/5** unchanged.

| Epic | Sprint status | Coverage status |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6/6 done (fan-out metrics #2723 added evidence for 6-1) |
| 7A — Basic Document Management | in-progress | 5/5 done |
| 8A — Basic Notification Preferences | done | 3/3 done |
| 10A — OAuth Provider Foundation | done | 3/3 done |
| 10B — Platform Administration | in-progress | 7/7 done (PATCH endpoints #2716/#2717 restore admin flow) |
| 80 — Dispute Resolution | partial | 3/3 done in coverage (last_checked bumped this run; PR #2712 add_evidence audit event added to 80-1 evidence) |
| 84 — Documents / e-signature | (extended) | 3/5 done, 2 partial (84-1, 84-2 — both frontend) |
| 79 / 81 / 82 / 83 / 85 / 8a / 9 | (extended) | all done in coverage |

## Shipped since last run (18 PRs > #2702)

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


## What's next (top 5 actions from ranked backlog)

1. **[high] Finish 84-1 direct-to-S3 upload wiring in ppt-web — api-client binding + UploadDocument integration + regression test** — owner: pm-frontend
2. **[high] Build signer-facing document-sign page (84-2) in ppt-web against shipped signing API; verify signature-request email end-to-end** — owner: pm-frontend
3. **[high] Human triage of quarantined PR #2684 (workflow_cond_parse_failopen) — CI test-shards 1-4 red after 3 respawns; classify as flake vs real regression** — owner: pm-tech-lead
4. **[high] Refill dispatcher backlog buffer to ≥36 open — claimable dropped to 1/72 after this window's merge surge** — owner: pm-tech-lead
5. **[high] Investigate #2684 CI test-shard(1-4) failure — is it a real regression from clippy fix at workflow_executor.rs:1312 (cloned_ref_to_slice_refs -> std::slice::from_ref) or shard-splitting flake?** — owner: pm-tech-lead

## Blockers

- **PR #2684 code-review-api-core-workflow-cond-parse-failopen** — fix_rounds=3 exhausted; CI test-shard(1-4) FAILURE at 926afe8 after 3 respawns — owner: pm-tech-lead
- **84-1 direct-to-S3 wiring (long-standing partial)** — prior implementer failed no-PR; needs fresh attempt — owner: pm-frontend
- **84-2 signer page (long-standing partial)** — prior implementer failed no-PR; scope to shipped API — owner: pm-frontend

## Role focus today: **pm-qa** (rotation idx 3; last 2026-06-15, 55d stale) + pm-scrum-master always-on

- **pm-scrum-master** (always-on): produced the delivery synthesis above. Headline = 18 PRs in 24h with 4 security-adjacent and 1 quarantined; dispatcher buffer starved; refill of 49 open items this run.
- **pm-qa** (rotation): flagged #2684 CI test-shard fragility as top QA lever; recommends security-fix regression-test co-commit policy; wants to codify the #2718 no-fix HMAC-parity verification pattern; concerned that 3 in-review PRs (#2724/#2725/#2726) can't run local verify due to cloud egress blocking utoipa-swagger-ui.

## Coverage (upkeep this run — 2026-08-09)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated` bumped.
- **Epic re-check: epic-80** — cursor idx 5. All 3 stories still `done`; evidence entry added to 80-1 for PR #2712 (dispute add_evidence audit event). `last_checked = 2026-08-09` stamped on 80-1/80-2/80-3.
- **Merged-PR evidence added:** 80-1 (PR #2712 audit event). No status flips.
- **`coverage_cursor` advances 5 -> 6** (epic-80 -> epic-81 next run).
- **`pm_cursor` advances 3 -> 4** (pm-qa -> pm-devops next run). role_last_run["pm-qa"] = 2026-08-09.
- **Composition unchanged: 47 done · 2 partial · 0 not-started** across 13 epics. Same 3 missing UC links (UC-33.x — 3 queued into action-list this run).
