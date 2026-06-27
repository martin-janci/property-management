# PPT Project State

_Generated: 2026-06-27 — daily PM rotation (Scrum Master + pm-security; routine refresh). Coverage `scan_kind=upkeep`; pm_cursor idx 5 → 6 (pm-data next), coverage_cursor idx 12 → 0 (epic-9 → epic-10a)._

## Executive summary

- **95 PRs merged 2026-06-22..2026-06-27 (#1567–#1864)** in an 11-day catch-up window; mostly martin-janci + 2 dependabot. Epic 6 (Announcements) **done**, Epic 11 (Financial / Stripe) shipping, Epic 16 (Saved-search alerts) live, Epic 18 (Guest OCR seam) landed, Messaging extended to N-party + group + attachments, Epic 7B e-signature integration shipped.
- **Sprint reconciliation:** Stories 6-1..6-5 + 79-1 + 10b-5 + 10b-7 promoted from sprint-status `ready-for-dev`/`review` → `done` (2026-06-24..2026-06-25 verify-reports). Action-list closes 7 corresponding coverage-gap items.
- **Security pressure-front (47 `from-merged-review` issues #1758-#1854):** IDOR cluster on the new messaging-attachment surface (#1791), OCR endpoints unauth + PII leak (#1772, #1823), Stripe hardening (#1764/#1824), payment-reminder double-fire (#1790), Booking JWT manager gate (#1787). Multiple draft PRs in flight: #1797 (OCR), #1799 (msg attachment IDOR), #1806 (booking_channel DB authz), #1825 (Booking.com currency), #1857 (LLM-doc IDOR regression).
- **Persistent infra blockers:** #1014 (archive-push MCP size limit) and #1680 (dispatcher cron env cannot run core pipeline) still open. Migration-version collision pattern reappeared (PRs #1724/#1755/#1757) — needs pre-commit monotonic-version guard.
- **CI / churn signals:** Multiple CI unblock PRs landed (#1721/#1723/#1730/#1742/#1748, #1730 mobile build, #1735 mixed-PR decision). One revert: PR #1713 reconciled BIT-213 delegations re-removal with retirement.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · epics_done=2/6 sprint-tracked (8A done; 6 effectively done with 6-1..6-6 all done).

| Epic | Tracked status | Real status |
|---|---|---|
| 6 — Announcements & Communication | in-progress | **6/6 stories done** (sprint-status reconcile pending — counter shows 3/6 but 6-1..6-5 all promoted to done 2026-06-25) |
| 7A — Basic Document Management | in-progress | 7a-1 done; 7a-2 in review; 7a-3/7a-4/7a-5 effectively done in coverage |
| 8A — Basic Notification Preferences | done | 3/3 stories done; mobile push (FCM/APNs) leg landed via #1556/#1450 |
| 10A — OAuth Provider Foundation | in-progress | 3/3 stories code-complete in coverage; sprint-status stale |
| 10B — Platform Administration | in-progress | 7/7 done (10b-5/10b-7 reconciled 2026-06-24/25) |
| 80 — Dispute Resolution | partial | 80-1 done; 80-2 partial (5-step wizard + i18n); 80-3 partial (party-submissions unwired) |

## Shipped since last run (95 PRs, #1567–#1864 — top headlines)

- **Epic 6 done** — verify-6-1..6-5 reconciled to done 2026-06-25 (announcements creation/targeting, viewing/ack, comments, pinning, direct messaging)
- **Epic 11 Financial / Stripe** — PRs #1717/#1725/#1726/#1729 (Stripe Checkout BIT-181, financial reports, payment hardening)
- **Epic 16 Saved-search alerts live** — PRs #1847/#1849/#1850 (BIT-139/140 alerts)
- **Messaging N-party + group + attachments** — PRs #1689/#1696/#1702/#1712/#1853
- **Epic 7B E-signature** — PR #1697 (story 7B.3 e-signature integration milestone)
- **Epic 18 Guest OCR seam** — PR #1750 (story 18.2)
- **Epic 3 Units/Buildings/Maps polish** — PRs #1691/#1711 building geocoding/map; #1701 resident My Unit
- **Fault notifications** — PR #1705 (story 4.x)
- **Payment reminders** — PR #1709 (story 11.6) [follow-up #1790 double-fire pending]
- **Event bus** — PR #1716 (story 2B.1)

## What's next (top 5 actions)

1. **[high] Land IDOR / PII cluster** — #1799 (msg attachment IDOR) + #1797 (OCR auth + rental PII) + #1823 (guest ID-doc PII). Owner: pm-security + pm-backend.
2. **[high] Close Stripe / Booking hardening** — #1764/#1824 (Stripe webhook signature + idempotency) + #1825 (Booking.com currency) + #1787 (Booking JWT manager gate). Owner: pm-security + pm-backend.
3. **[high] Fix payment-reminder double-fire (#1790)** — scheduler dedupe / advisory lock. Owner: pm-backend.
4. **[medium] Resolve infra blockers** — issue #1014 (archive-push MCP size limit) + issue #1680 (dispatcher cron env). Owner: pm-devops.
5. **[medium] Triage the 47 open `from-merged-review` follow-ups #1758-#1854** — assign owners, slot in by severity. Owner: pm-scrum-master.

## Blockers

- **#1014 archive-push MCP size limit** — Owner: pm-devops. Recurring; blocks large-archive PR pushes.
- **#1680 dispatcher cron env** — Owner: pm-devops. Research dispatcher cron environment cannot run the core pipeline.
- **47 open security follow-ups #1758-#1854** — Owner: pm-security + pm-backend. Not yet triaged into owners/priorities.
- **Migration-version collision pattern** — Owner: pm-devops. PRs #1724/#1755/#1757 all fix `duplicate migration version` — needs structural guard.

## Role focus today: **pm-security** (+ pm-scrum-master always-on)

- **pm-security** (rotation idx 5, last 2026-05-27, ~30d stale): 5 new next_actions appended to `action-list.json`; 2 new risks appended to `risks.json`. Full role JSON in `.research/management/roles/pm-security.md`. Headline: IDOR + PII surface on the just-shipped messaging-N-party / OCR / Stripe surfaces is the dominant risk this window — 47 `from-merged-review` follow-ups #1758-#1854 need triage.
- **pm-scrum-master** (always-on): produced the delivery synthesis above; headline = 95 PRs merged in 11d, Epic 6/8A/16 effectively done, security follow-up cluster + infra blockers are the gate to GA on the new financial/messaging surfaces.

## Coverage (upkeep — 2026-06-27)

- **Upkeep refresh** — supersedes the 2026-06-23 deep regen with the sprint-status reconciliations for stories 6-1..6-5, 79-1, 10b-5, 10b-7 (all → done). epic-9 (MFA) re-checked: no regression in window.
- **coverage_cursor advances 12 → 0** (epic-10a is next).
- **Action-list state** — 7 coverage-gap items closed (sprint-status reconciliation); 5 pm-security next-actions + 8 buffer-refill items appended. Buffer 20/36 — under target but no further sensible refill candidates remain (most partial stories already have action items).

