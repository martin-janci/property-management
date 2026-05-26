# PPT Project State

_Generated: 2026-05-25 — daily PM rotation (Scrum Master + pm-qa). Coverage map last rebuilt by `/ppt-project-management scan` on 2026-05-23; upkeep-refreshed 2026-05-25._

## Executive summary

- **15 PRs merged since the last run (#474–#502)** — announcement web UI wiring (#474 view-page → mark_read/acknowledge, #475 comments), execution-history UI (#489), e-signature email HMAC for Epic 84.2 (#495), the SwiftUI iOS project audit + reality-mobile screen maps (#501), plus delivery tooling (#476 dispatcher hardening, #492/#494 sequencing & module-split backlog).
- **Two IDOR security fixes landed — one with tests, one without.** #493 (equipment cross-tenant IDOR) shipped *with* `equipment_cross_tenant_idor_tests.rs`. #497 (inquiry `mark_as_read` scoping) shipped *without any test* and left 3 acceptance-criteria TODOs unchecked — a regression-test gap on a live security fix (now tracked as backlog `test-gap-inquiry-idor-regression`).
- **WebSocket infra ownership confirmed (ADR-008, #496)** — clears the architectural ambiguity gating 8a-3 sync; the mobile-push leg (#490, open) remains.
- **Two structural blockers persist.** 7a-1 is formally not promotable — POST `/api/v1/documents/upload` is missing from the backend (confirmed by #502) while mobile upload UI calls it. 80-3 mediation pages/hooks are complete but unreachable (App.tsx uses an inline JSX stub, no `/disputes/:id/mediation` route).
- **High-severity hardening still open:** #480 (JWT in WebSocket access logs) and #481 (OAuth refresh-token revocation bypass) gate 8a-3 / 10a-1 / 10a-3 from `done`.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

| Epic | Tracked status | Real status (from coverage upkeep) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6-1/6-5/6-6 done; 6-2 view + 6-3 comments web UI now wired (#474/#475), still `partial` pending gates; 6-4 partial |
| 7A — Basic Document Management | in-progress | 7a-3/7a-5 done; **7a-1 blocked** — backend upload endpoint missing (#502) |
| 8A — Basic Notification Preferences | done | 8a-1/8a-2 done; 8a-3 WS half confirmed (ADR-008 #496), mobile-push leg open (#490) |
| 10A — OAuth Provider Foundation | in-progress | backend done + admin UI; gated by #481 revocation bypass |
| 10B — Platform Administration | in-progress | 3 done; 10b-4 system announcements admin dashboard in draft (#503) |
| 81 — Reports | in-progress | execution-history UI wired (#489); backend execution/download endpoints still absent |

## What's next (top 5)

1. **[high · pm-backend]** Implement POST `/api/v1/documents/upload` backend handler — unblock 7a-1 (not promotable per #502; mobile UI calls a missing route).
2. **[high · pm-frontend]** Wire App.tsx dispute routes — replace the inline JSX stub with DisputeDetailPage and add `/disputes/:id/mediation` (80-3 pages/hooks are dead code until wired).
3. **[high · rust-backend]** Add the 3 inquiry-IDOR regression tests for PR #497 (B→404 & read_at NULL; A→204 set; idempotent re-mark→204).
4. **[high · pm-backend]** Resolve #480 (JWT in WS logs) & #481 (OAuth revocation bypass) before OAuth/WS stories promote.
5. **[medium · pm-frontend]** Land mobile push PRs #490/#498/#499 to unblock the 8a-3 mobile leg + 85-2 build config.

See `roadmap.md` for the full ranked plan and `action-list.json`/`action-list.md` for the tracker view.

## Blockers

- **7a-1-document-upload-metadata** — backend POST `/documents/upload` absent (#502); mobile upload UI calls a missing route. Owner: pm-backend.
- **80-3-mediation-resolution** — DisputeDetailPage/MediationPage implemented but unreachable from App.tsx. Owner: pm-frontend.
- **thb-2026-05-25 gate** — #480–#487 open; gates 8a-3, 10a-1/2/3, 7a-5, 6-2, 6-5. Owner: pm-backend.

## Role focus today

Role focus today: pm-scrum-master, pm-qa.

- **pm-scrum-master:** 15 PRs shipped; 3 structural blockers (7a-1 backend upload, 80-3 routing orphan, #480–#487 batch). Sequence App.tsx PRs to avoid churn conflicts.
- **pm-qa:** PR #497 inquiry IDOR fix shipped with zero regression tests (vs #493 which had tests); 8 open hardening issues, 2 high-severity (#480/#481), gate six stories. Recommends a test-file-required policy for security-fix PRs.
