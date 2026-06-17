# PPT Project State

Generated: 2026-06-17T00:00:00Z

## Executive summary

Sprint 'Epic 6, 7A, 8A & 10A' saw a very high-velocity 38-hour window: 66 PRs merged (#1440–#1545) including the native accounting MVP (17,983 LOC), RLS-pool migration completions (BIT-74/76/78/85), Booking.com OTA security hardening (BIT-98/110), bank-statement/payment-matching (PAP-210), and notification preference-aware push (8A-3). Epic 8A remains the only fully closed epic; test-hardening batch #480-#487 still open.

## Sprint progress

- Sprint: **Epic 6, 7A, 8A & 10A - Announcements, Documents, Notifications & OAuth**
- Epics done: **1/6**

## Shipped since last run

- PR #1454 — Native Accounting MVP (17,983 LOC)
- PR #1453 — Bank-statement upload + payment-matching N5 (PAP-210)
- PR #1450 — Notification preference-aware push (8A-3)
- PRs #1471, #1460, #1467, #1473 — RLS-pool migration + force-RLS (BIT-74/76/78/85)
- PR #1472 / #1485 — Booking.com OTA credential security hardening (BIT-98/BIT-110)
- PRs #1441, #1452, #1455 — DevOps gates: pre-push fmt/clippy + dev-push compile gate + CI fmt/clippy mirror

## What's next (top 5)

- **[high]** Fix CI: make backend `test` job a required check on `dev` branch (issue #1538) — dep: `none`
- **[high]** Resolve test-hardening issues #480 (JWT in WS logs) + #481 (OAuth refresh-token revocation bypass) — dep: `none`
- **[high]** Land Epic 6 announcement web UI (draft PRs #474/#475/#479) to advance 6-2/6-3/6-4 — dep: `pm-frontend`
- **[medium]** Complete mobile slice for 7a-2-folder-organization; verify 7a-1, 7a-4 mobile previews — dep: `pm-frontend`
- **[medium]** Wire DisputeDetailRoute + /disputes/:id/mediation; implement useDraftStorage (80-2/80-3) — dep: `pm-frontend`

## Blockers

- **Issue #1538** (pm-tech-lead) — CI backend test job not required on dev — red-test PRs merge freely
- **Test-hardening batch #480-#487** (pm-backend) — Gate promotion of 8a-3, 10a-1/2/3, 7a-5, 6-2, 6-5 to done
- **Story 7a-2-folder-organization** (pm-backend) — Reverted; CI test red on document_folder_tests FK; mobile slice pending
- **Stories 80-2 and 80-3** (pm-frontend) — 80-2 missing useDraftStorage; 80-3 missing /disputes/:id/mediation route

## Role focus today

- **pm-security** (rotating slot)

## Per-role summaries

- **pm-security** — Three pre-existing high-severity open issues (#481 revoked refresh tokens reusable, #480 JWT in WS query-param logs, #487 MFA rate-limit untested) carry over from 2026-05-27 with no evidence of closure; new #1538 CI gate gap means red backend test runs can merge to dev, undermining RLS and OAuth reg…
