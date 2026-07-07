# Project delivery snapshot

_Generated: 2026-07-07 10:30 UTC (rotating role: pm-security)_

## Exec summary

The 27-PR follow-up sweep (#2094–#2146) cleared the prior merged-review debt and unblocked **RUSTSEC-2026-0204** (#2144), which had been jamming every backend PR; mobile RN wiring (#2118 + #2146) closed a real mobile gap. Sprint spine still shows Epic 10A OAuth as `ready-for-dev` on all 3 stories, but `coverage.json` flags it done via older PRs — needs reconciliation — while **80-2 dispute wizard** and **8a-3 mobile push** remain the top partial MVP gaps.

## Sprint progress

- **Sprint:** _Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth_
- **Epics done:** 1 / 4
- **Started:** 2025-12-21

## Shipped since last run (2026-07-05 → 2026-07-07)

- **#2144** — RUSTSEC-2026-0204 crossbeam-epoch bump (was blocking every backend PR)
- **#2118 + #2146** — mobile RN Meters/Leases/Forms/Threads wired to `api-server`, typed against `@ppt/api-client`
- **#2094–#2137** — 20+ PR follow-up sweep clearing prior merged-review debt (fixtures, RBAC guards, enum-sync guards, XXE ban, CI gates, accounting-export honesty)
- **#2115** — ListingDetail auth reset spinner + hoisted tests
- **#2100** — rental repository split refactor
- 24 of 25 touched issues (#2082–#2141) closed via the sweep; **#2125** remains open pending PR #2135

## What's next

1. **[high]** Merge #2150 (fix follow-up sweep integration) to close the sweep cleanly — owner: PM
2. **[high]** Land #2135 (ListingDetail `updateAuth()` test + favorite-toggle rollback after logout), closing #2125 — owner: react-web/mobile
3. **[high]** Fix red CI on `document_folder_tests` to unblock 7a-2-folder-organization from review to done — owner: pm-backend
4. **[high]** Re-verify #481 (OAuth refresh-token revocation) — code appears already fixed; close/defer the gate to unblock Epic 10A stories — owner: pm-security/rust-backend
5. **[high]** Land #1797 — auth on OCR endpoints + rental guest PII gate (long-standing #1772/#1766) — owner: pm-security

## Blockers

- **7a-2-folder-organization** — CI red on `document_folder_tests` (FK/isolation fix in PR #1316 round 1); reverted from done pending green CI. _Owner: pm-backend_
- **80-2-dispute-filing-flow** — AC-4 draft auto-save (`useDraftStorage.ts`) not implemented; 5-step wizard redesign still in-progress. _Owner: pm-frontend_
- **8a-3-notification-preference-sync / Epic 10A stories** — test_hardening_batch items #480/#481/#484/#487 still listed `open` in sprint-status, gating done-promotion — needs audit against new sweep. _Owner: rust-backend_

## Role focus today

- **Rotating role:** pm-security (see `roles/pm-security.md`)
- **Always-on:** pm-scrum-master
