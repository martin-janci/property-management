# pm-scrum-master — 2026-05-23

**Summary:** Sprint "Epic 6, 7A, 8A & 10A" is mid-flight: 10B leads (3/7 stories done), 8A is fully in review (3 stories), and 6.1 is in review — but most stories across 6/7A/10A are still `ready-for-dev`, and Epic 6/8A dispatch is blocked on the unbuilt Epic 2B notification infrastructure. Quiet run: nothing merged to `dev` since the last cursor; PR #435 (security 10-pass auth/security fixes) is open and awaiting a review decision.

## Shipped since last run
- _(none — no new PRs merged to `dev` since #433)_

## Sprint progress
- **Sprint:** Epic 6, 7A, 8A & 10A - Announcements, Documents, Notifications & OAuth
- **Epics done:** 0 / 5 (10B is 3/7 stories; 8A 3/3 in review; 6 has 1 story in review)

## Next actions
| Priority | Action | Dependency | Definition of done |
|---|---|---|---|
| high | Get a review decision on PR #435 (security auth/security fixes) and merge to `dev` | none | #435 reviewed + merged or change-requested |
| high | Complete code review of Epic 8A (8a-1/8a-2/8a-3, all in `review`) and move to done | none | 3 stories merged or kicked back |
| medium | Review and merge story 6.1 (announcement creation/targeting) | none | 6-1 status → done |
| medium | Sequence Epic 2B notification infrastructure ahead of Epic 6 publish + 8A.2 dispatch | tech-lead | 2B scoped/ordered before 6.2+ pickup |
| low | Land security-voice-device-idor plan (ready in `.research/plans/`) | none | IDOR fix PR opened against `dev` |

## Risks
| Risk | Prob | Impact | Mitigation |
|---|---|---|---|
| Epic 6/8A blocked on un-built Epic 2B notification infra; deferred dispatch + WS sync (8A.2/8A.3) | high | high | Promote Epic 2B into the sprint or explicitly defer 6.2+/8A.2 until 2B lands |
| 8A fully in review with 0 merged — review backlog stalling the sprint | medium | medium | Prioritize 8A review this run; one reviewer-owner |
| PR #435 (security fixes) lingering open un-reviewed | medium | high | Assign review owner; treat as P0 given auth scope |

## Open questions
- Is Epic 2B notification infrastructure scheduled for this sprint, or are Epics 6/8A intentionally partial until a later sprint?
- Who owns the review queue for the 4 stories now sitting in `review` (8A x3, 6.1)?

## Decisions needed
- Whether to pull Epic 2B notification infrastructure into the current sprint to unblock Epic 6 publish + 8A.2 dispatch — owner: pm-scrum-master
- Merge target/priority for security PR #435 vs. feature review backlog — owner: pm-tech-lead

## Blockers
| Item | Reason | Owner role |
|---|---|---|
| Epic 6 (6.2–6.6) | Depends on Epic 2B notification infrastructure for publish notifications | pm-tech-lead |
| Story 8A.2 / 8A.3 | Dispatch logic + WebSocket sync deferred pending Epic 2B + WS infra | pm-tech-lead |
