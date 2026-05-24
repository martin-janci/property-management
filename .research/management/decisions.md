# Decision log — PPT delivery

> Maintained by `pm-scrum-master`. Append decisions; never delete. Format below.

## Decisions made

### DEC-001 — Epic 2B build order: land infra first, formally defer dependent slices

**Date:** 2026-05-24
**Owner:** pm-tech-lead
**Status:** decided

**Decision:**
Land Epic 2B notification infrastructure (event bus, WebSocket server, push/FCM/APNs, email dispatch) **before** picking up:
- Epic 6 stories 6.2+ (announcement viewing/comments/pinning that trigger notifications on publish)
- Epic 8A.2 notification dispatch logic
- Epic 8A.3 WebSocket-based preference sync

The formally deferred slices are:
- `6-2-announcement-viewing-acknowledgment`
- `6-3-announcement-comments-discussion`
- `6-4-pinned-announcements`
- `6-5-direct-messaging` (requires WS realtime)
- `8a-2-critical-notification-override` (dispatch half)
- `8a-3-notification-preference-sync` (WS half)

**Rationale:**
1. **Zero WS infrastructure exists.** No WebSocket server (Axum upgrade handler, connection registry, pub/sub bridge) exists in `backend/`. Only type definitions and notification preference models are present. Building 8A.3 or 6-5 real-time features now means writing throw-away stubs that will be replaced when 2B lands — a confirmed high-probability, high-impact rework risk (risks.json `pm-tech-lead-notification-ws-foundation-rework`).
2. **Sprint notes already acknowledge the deferrals.** `sprint-status.yaml` explicitly records "Story 8A.2 notification dispatch logic deferred — requires Epic 2B" and "Story 8A.3 WebSocket sync deferred — requires WebSocket infrastructure". This decision formalises what the sprint already implies.
3. **Feature-flag alternative is not cost-free.** Freezing dependent slices behind feature flags requires flag infrastructure that doesn't exist either. Landing 2B directly is cleaner and lower net effort than building a flag system first.
4. **No functional regression.** The deferred stories have no shipped frontend yet (all `apiStatus: stub` or `ready-for-dev`). Deferring them does not break anything currently live.
5. **2B unblocks multiple epics at once.** Once 2B lands it unblocks 8A.2, 8A.3, 6-2 through 6-5, and the future direct-messaging realtime slice — making it the highest-leverage infrastructure investment in the current backlog.

**Scope of Epic 2B to land (in order):**
1. Story 2B.1 — Event Bus Foundation (Redis pub/sub adapter in `backend/crates/`)
2. Story 2B.2 — Push Notification Service (FCM/APNs via HTTP v1)
3. Story 2B.3 — Email Notification Service (dispatch, not preference toggle — `services/email.rs` skeleton exists)
4. Story 2B.4 — In-App Notification Center (DB-backed, REST endpoints for list/mark-read)
5. Story 2B-C.1 — WebSocket Real-Time Sync (Axum WS upgrade handler, ADR-008)

Stories 2B.5 (Privacy-Aware Design), 2B.6 (Offline Sync Queue), 2B.7 (Idempotency), and 2B-C.2/2B-C.3 can follow in a subsequent sprint as they do not block the immediate Epic 6/8A unblock.

**Formally deferred (not deleted from backlog):**
The stories listed in "deferred slices" above remain in the backlog at their current priority. They are unblocked the moment Epic 2B stories 1–5 (including WS) are merged to dev. pm-scrum-master should re-sequence them into the sprint immediately after 2B lands.

---

## Decisions needed
- Whether to pull Epic 2B notification infrastructure into the current sprint to unblock Epic 6 publish + 8A.2 dispatch — owner: pm-scrum-master _(raised 2026-05-23)_ **unblocked by DEC-001**
- Merge target/priority for security PR #435 vs. feature review backlog — owner: pm-tech-lead _(raised 2026-05-23)_
- Whether to delete the dead AuthHandler/BuildingHandler modules now vs. wire them as the canonical path — owner: pm-tech-lead _(raised 2026-05-23)_
