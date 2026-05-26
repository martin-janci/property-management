# pm-scrum-master — Delivery synthesis (2026-05-25)

_Always-on synthesis. Read-only._

## Summary

High shipping velocity: PRs #474–#502 closed out announcement web UI wiring, two IDOR security fixes, e-signature HMAC (Epic 84.2), WebSocket ADR-008, the SwiftUI iOS audit, and execution-history wiring. Three structural blockers remain: **7a-1** is formally not promotable (POST `/api/v1/documents/upload` missing from backend, per PR #502), **80-3 mediation** is orphaned in `App.tsx` routing despite complete page/hook code, and the **#480–#487** test-hardening batch gates seven stories from `done`.

## Sprint progress

- Sprint: *Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth*
- Epics done: 2 / 6 (8A done; others in-progress)

## Shipped since last run (#474–#502)

- #474 announcement view-page → mark_read/acknowledge APIs (6-2)
- #475 announcement comments web UI (6-3)
- #476 dispatcher hardening (tooling/CI)
- #477/#478 dispute filing + mediation partial (80-2/80-3)
- #489 execution-history UI wired (81-2)
- #491 test-hardening batch slotting (#480–#487)
- #492 Epic 6 announcement PR sequencing
- #493 equipment cross-tenant IDOR fix **with** regression test (security)
- #494 module-split backlog + discarded-principal CI lint
- #495 Epic 84.2 e-signature email (HMAC provider)
- #496 WebSocket infra ownership confirmed (ADR-008)
- #497 inquiry IDOR `mark_as_read` scoping — **no test** (3 TODOs open)
- #501 SwiftUI iOS audit + reality-mobile screen maps
- #502 gap-7a-1 NOT promotable: backend upload endpoint missing

## What's next (top 5)

1. **[high]** Implement POST `/api/v1/documents/upload` backend handler — unblock 7a-1 (pm-backend)
2. **[high]** Wire App.tsx dispute routes: DisputeDetailPage + `/disputes/:id/mediation` (80-3, pm-frontend)
3. **[high]** Add the 3 regression tests to PR #497 inquiry IDOR fix (pm-security/rust-backend)
4. **[high]** Resolve high-severity #480 (JWT in logs) & #481 (OAuth revocation bypass) before OAuth/WS stories promote (pm-backend)
5. **[medium]** Land mobile push PRs #490/#498/#499 to unblock 8a-3 mobile leg + 85-2 (pm-frontend)

## Blockers

- **7a-1-document-upload-metadata** — backend POST `/documents/upload` absent (PR #502); mobile UI (PR #447) calls a missing route. Owner: pm-backend.
- **80-3-mediation-resolution** — DisputeDetailPage/MediationPage implemented but unreachable; App.tsx inline JSX stub. Owner: pm-frontend.
- **thb-2026-05-25 gate (8a-3, 10a-1/2/3, 7a-5, 6-2, 6-5)** — #480–#487 open. Owner: pm-backend.

## Decisions needed

- P0 vs formal deferral for #480/#481 (high-severity security) — owner: pm-security + tech lead.
- App.tsx route-change merge sequence (80-3 wiring vs #503) to prevent churn conflicts — owner: pm-frontend.
- Epic 81 backend endpoint work in-scope this sprint or punted — owner: pm-backend + PO.
