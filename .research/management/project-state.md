# Project state — 2026-06-26

_Last updated: 2026-06-26T06:30:00Z_

## Executive summary

The 10-day window (2026-06-16 to 2026-06-26) saw 270 merged PRs across messaging (Epic 6 group threads, participant lists — #1768/#1844/#1848/#1853), accounting MVP (#1717/#1804/#1807/#1811), saved-search alerts (Epic 16 — #1847/#1849), disputes (Epic 80 — #1845/#1846), and booking-channel hardening (#1806/#1824/#1825); coverage backlog exhausted (0/72 claimable), 12 partial stories now in-flight or shipped, but sprint-status.yaml labels lag — five Epic 6 stories verified done on dev still carry stale labels, and three test-hardening-batch gate issues (#480/#481/#484) remain open.

## Sprint progress

- Sprint: Epic 6, 7A, 8A, 10A — Announcements, Documents, Notifications, OAuth (extended to include Epics 11/16/80/84 in-flight)
- Epics done: 2 / 6

## Shipped since last run

- Epic 6 — group thread messaging (PRs #1768/#1844/#1848/#1853)
- Epic 11 — accounting MVP financial-reports + scheduler + state-machine (#1717/#1804/#1807/#1811)
- Epic 16 — saved-search alert cadence + transport drainer (#1847/#1849)
- Epic 80 — dispute state reconciliation + party submissions (#1845/#1846 in review)
- Booking-channel hardening — DB-backed manager gate + Stripe multi-currency + Booking.com currency validation (#1806/#1824/#1825)

## What's next

- **[high]** Reconcile sprint-status.yaml Epic 6 stories 6-1..6-5 to done — owner: `pm-backend`
- **[high]** Close or defer test-hardening gate issues #480/#481/#484 (block 10a + 8a stories) — owner: `pm-security`
- **[high]** Shepherd PR #1846 (80-3 mediation wire) to merge and promote story to done — owner: `pm-frontend`

## Blockers

- 10a-1/10a-3 blocked by gate #481 (OAuth refresh-token revocation bypass)
- 8a-2/8a-3 blocked by gates #480 (WS JWT in logs) + #484 (FCM serial swallow)
- 84-5 pgvector-rag PR #1833 quarantined (broken v_rag_statistics view)
- 80-2 dispute filing AC-4 (useDraftStorage.ts) not implemented
- 7a-2 folder-organization stuck in CI red

## Role focus today

- pm-scrum-master (always)
- pm-security (rotation index 5 → 6)

## Recent role analyses

- `roles/pm-scrum-master.md` — refreshed 2026-06-26
- `roles/pm-security.md` — refreshed 2026-06-26 (last run 2026-05-27 — 30d stale before this)
