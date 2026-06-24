# PPT Project State

_Generated: 2026-06-24 — Scrum Master + pm-security rotation (Phase 1.6); coverage_cursor 12 → 0 (epic-9 → epic-10a); pm_cursor 5 → 6 (pm-data next)._

## Executive summary


**Shipping velocity is exceptional.** 95 PRs merged in the window, only 1 revert (#1713 backing out #1690 delegation re-add), and 5 closed-not-merged. Three large feature surfaces landed simultaneously: (1) **Story 11.5 Stripe Checkout** (#1726) closes the payments loop with payment-reminders (#1709) + event-bus retry (#1716) + notification analytics (#1722); (2) **Epic 6 messaging** finishes with N-party messaging (#1689/#1696) + message attachments (#1702/#1712); (3) **fault & IoT operations** delivered fault notifications (#1705), OFFLINE fault queue (#1715), ReportFaultScreen wiring (#1679), IoT real-time dashboard (#1685), and IoT alert error feedback (#1740). Resident-facing surfaces (My Unit #1701, leases nav #1694, unit management UI #1695, document-templates UI #1707, e-signature #1697, building geocoding #1691/#1711) are now coherent.

**New architectural axis: standalone accounting-server (Epic ACC).** Six PRs introduced it this window — TypeSpec (#1817), accounting-web Next.js (#1808), generated client (#1803), payment-match state machine (#1811), e2e tests (#1805), hardening (#1809). Umbrella PR #1821 is open as draft. This is a major decoupling decision and needs an explicit owner + go/no-go before more workstreams attach.

**Refactor program is paying off.** 14 churn-hotspot route-file splits (reserve_funds, iot, vendors, sensor, subscription, enhanced_tenant_screening, emergency, aml_dsa, document, form/forms, integrations/booking, plus tests) — directly addressing the monolith risk class that previously hid cross-tenant IDORs.

**Operational drag is concentrated in CI.** Seven CI fires this window (migration version collisions 00187/00192, backend test gate, frontend mobile typecheck, build-mobile exec bit, docker frontend e2e pkg, skip determinism). Each unblocked quickly but the pattern — migration-number races and platform-CI flake — repeats. Dispatcher defect cluster (#1747/#1739) closed by #1751 archive-terminal reconciler; #1680 cloud cron env still open. 21 PRs remain open (mostly drafts, all updated in last 24-48h), and 76 of 84 new issues are auto-issues from post-merge review — backlog is breathing but follow-up close rate is the next constraint.

**Sprint posture:** the active sprint (epics 6/7A/8A/10A) is functionally complete on epic-6 (announcements + messaging) and epic-7A (documents + templates + e-signature) modulo verification flips on the 6-x/7a-x story rows in sprint-status.yaml — file is materially stale and should be reconciled this run.

## Sprint progress ()

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · epics_done=1/6 · stories_done=37 · stories_partial=12

## Shipped since last run (cursor #1439 → #1822, 95 PRs)

- PR#1726 Story 11.5 Stripe Checkout — pm-backend
- PR#1716 Event-bus retry/DLQ — pm-backend
- PR#1709 Payment reminders — pm-backend
- PR#1717 Financial reports backend — pm-backend
- PR#1725 Financial reports frontend — pm-frontend
- PR#1705 Fault notifications — pm-backend
- PR#1689 N-party messaging backend — pm-backend
- PR#1696 N-party messaging UI — pm-frontend
- PR#1702 Message attachments backend — pm-backend
- PR#1712 Message attachments UI — pm-frontend
- PR#1715 OFFLINE fault queue — pm-frontend
- PR#1714 Person-months endpoint — pm-backend
- PR#1694 Leases navigation — pm-frontend
- PR#1695 Unit management UI — pm-frontend
- PR#1691 Building geocoding backend — pm-backend
- PR#1711 Building geocoding UI — pm-frontend
- PR#1701 Resident My Unit screen — pm-frontend
- PR#1707 Document-templates UI — pm-frontend
- PR#1697 Document e-signature — pm-backend
- PR#1685 Real-time IoT dashboard — pm-frontend
- PR#1736 AI dashboards via api-client — pm-frontend
- PR#1740 IoT alert error feedback — pm-frontend
- PR#1822 Reality-web SSO callback e2e — pm-qa
- PR#1750 OCR endpoint — pm-backend
- PR#1688 Idempotency-key middleware — pm-backend

## What's next (top 5 actions)

1. **[high] Reconcile sprint-status.yaml: flip 6-1/6-2/6-3/7a-2/7a-4/7a-5 to done where coverage.json + merged PRs agree; reset stale-review rows** — owner: pm-scrum-master
2. **[high] Designate Epic ACC technical owner + merge-or-park decision on #1821; freeze further accounting-* PRs until decision lands** — owner: pm-tech-lead
3. **[high] Add migration-number allocation gate (CI check or pre-commit) — block PR if migration filename collides with dev** — owner: pm-devops
4. **[medium] Triage cluster #1758–#1791 + #1826–#1828 post-merge follow-ups: assign owner or close as won't-fix; cap auto-issue backlog growth** — owner: pm-qa
5. **[high] Run pm-security pass on Stripe Checkout #1726 + idempotency middleware #1688 + Epic ACC payment-match #1811 before they get more callers** — owner: pm-security

## Blockers

- **sprint-status.yaml materially stale (epic-6 + 7A stories not flipped to done despite shipped evidence)** — owner: pm-scrum-master
- **Epic ACC umbrella PR #1821 draft — no go/no-go owner; 6 dependent PRs already merged into the new server line** — owner: pm-tech-lead
- **Dispatcher cloud cron env defect #1680 — still open, blocks autonomous nightly runs** — owner: pm-devops
- **23 open post-merge follow-up PRs (cluster #1758–#1791, #1826–#1828) — auto-issue close-rate < merge-rate** — owner: pm-qa
- **Migration numbering races (00187/00192 collisions) keep recurring — no allocation gate** — owner: pm-devops

## Role focus today: **pm-scrum-master + pm-security**

- **pm-scrum-master** (always-on): delivery synthesis — 95 PRs merged in 8d, 1 revert, 5 closed-not-merged. Headline: epic-6 messaging + epic-7a documents functionally done modulo sprint-status reconciliation; new accounting-server architectural axis introduced (Epic ACC).
- **pm-security** (rotation idx 5, last 2026-05-27, 28d stale): Three high-severity surfaces landed this window (Stripe Checkout, idempotency-key middleware, accounting payment-match) with no recorded security review. Cross-tenant IDOR class continues to surface in monolith routes; mitigation is the refactor program, which is now active. 7 findings, 5 new risks, 6 next_actions appended.

## Coverage upkeep (cursor 12 → 0)

- Re-checked epic at index 12 (epic-9); cursor advances to 0 (epic-10a) for next run.
-  last deep-scanned 2026-06-23; this run did upkeep only (no scan).
- 8 stories likely flippable to  via this run's merged PRs — see sprint-status reconcile action.
