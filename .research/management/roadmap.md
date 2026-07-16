# PPT Delivery Roadmap

_Generated: 2026-07-16T02:20:00Z · scan_kind: upkeep · 49 stories · 13 epics_

## State of the project

- Stories: done=49, partial=0, not-started=0
- Epics fully done: 13/13
- Per-platform status:
  - **backend**: done=34, partial=0, not-started=0
  - **frontend**: done=39, partial=0, not-started=0
  - **mobile**: done=10, partial=0, not-started=0
- Screen coverage: 25 stories without screen-map · 0 orphan epics · 0 orphan screens · 3 missing UC links

## Ranked plan

_Backlog is fully drained (49/49 stories done in coverage.json). Active planning surface has moved to `.research/management/action-list.json` (20 open items) — this roadmap now tracks structural / cross-cutting work only._

### Security hardening (rotating expert review + pm-security 2026-07-16)
- [high] Fix POST /api/v1/agencies/{id}/invitations cross-tenant IDOR (missing check_agency_membership) — owner: rust-backend — why: any authenticated portal user can mint 7-day invitation tokens for any agency; live cross-tenant vuln
- [high] Validate invitation.role against allow-list — owner: rust-backend — why: DB currently COALESCEs to 'realtor' but caller can request privileged roles; escalation path
- [medium] Introduce shared AgencyMember Axum extractor — owner: rust-backend — why: 4 IDOR fixes in 72h indicate structural gap; type-level guard beats per-handler discipline
- [medium] Audit list_members handler for auth — owner: rust-backend — why: no extractor at all; confirm intent

### Delivery discipline (pm-scrum-master 2026-07-16)
- [high] Stabilize portal_webhooks.rs — owner: pm-tech-lead — why: 3 edits in one run + 2 open follow-ups (#2358/#2360); freeze until hardened
- [medium] Re-scan coverage for epic-84 confirmation — owner: pm-scrum-master — why: upkeep just marked 84-1/84-2 done from PR evidence; deep scan verifies

Buffer: 20/36 open · candidates_remaining: 0 (backlog empty — refill from post-merge review issues #2318, #2320, #2369, #2370 next dispatcher tick)
