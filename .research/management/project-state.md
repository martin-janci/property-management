# PPT Project State

_Generated: 2026-07-06 — daily PM rotation (Scrum Master + pm-security; routine refresh). Coverage `scan_kind=upkeep`; pm_cursor idx 5 → 6 (pm-data next), coverage_cursor idx 12 → 0 (epic-9 → epic-10a)._

## Executive summary

- **All 18 merged PRs (#2094–#2120) were backlog/hardening work.** Zero sprint-story movement this window: 7a-3/7a-4 stay done, 7a-2 remains stuck in review on red CI, 7a-5 and the three OAuth Provider Foundation stories (10a-1/10a-2/10a-3) remain gated by the 2026-05-25 test-hardening batch (#481, #482, #487).
- **Security hardening momentum is strong.** PR #2120 closed a real production authz gap (outage mutations resolving to `Guest` under the JWT-vs-DB-role mismatch). PR #2096 replaced the advisory `pinned-dependency-guard.yml` with a hard cargo-deny ban on quick-xml ≠ 0.41.0. PR #2111 gated `backend/deny.toml` with code-owner review. All three ship regression tests.
- **The 2026-07-04 post-merge review batch is fully closed** — all 15 follow-up issues #2082–#2110 are CLOSED and their remedial PRs merged. No residual technical debt from this batch.
- **Follow-up drafts still stuck.** #1797 (auth on OCR endpoints + rental guest PII manager-gate) has sat open in draft for 12+ days; #1812 (churn-hotspot reality_portal split, labeled `needs-human-review`) 11+ days.
- **New backlog signals (5 rows, +7 score total).** Phase 1.5 mobile-rn review surfaced 3 code-review findings on ThreadDetailScreen (no-onError, meter-type default, untested handleSend); Phase 1 emitted 3 churn-hotspot rows for single-PR-effect files (multi_currency.rs, regional_compliance.rs, LeaseDetailScreen.tsx). No promotable candidate hit score ≥ 3.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · epics_done=1/6 (Scrum Master hint; sprint-status.yaml counter-vs-detail reconciliation flagged as a `medium` action this run).

## What's next (top 5 delivery actions)

1. **[high]** Fix CI (`document_folder_tests` FK/isolation) blocking 7a-2-folder-organization — owner: rust-backend.
2. **[high]** Un-draft and land #1797 (OCR auth + rental guest PII manager-gate) — owner: rust-backend / security.
3. **[high]** Close or explicitly defer test-hardening batch thb-2026-05-25 items #481/#482/#487 gating 10a-1/2/3 — owner: rust-backend / react-web.
4. **[high]** Verify oauth.rs `revoked_at` filtering end-to-end vs issue #481; reconcile and close or re-open with root cause — owner: rust-backend (pm-security).
5. **[high]** Redirect WS auth off the query string (#480): move token to `Sec-WebSocket-Protocol` header or first-frame auth — owner: rust-backend (pm-security).

## Blockers

- **7a-2-folder-organization** — CI red (`document_folder_tests` FK/isolation), reverted from done.
- **7a-5-document-sharing** — gated by open issue #485 (window.confirm + no UUID validation).
- **10a-1/10a-2/10a-3 (OAuth Provider Foundation)** — gated by open test-hardening items #481, #482, #487.
- **#1797 (auth on OCR endpoints + PII guard)** — draft PR open 12+ days, no movement.

## Role focus today

- **pm-scrum-master** — delivery synthesis; flagged draft-PR stall, sprint-status counter drift, and stale coverage.json.
- **pm-security** — OAuth/WS/OCR authz surface; #481 revocation reconcile, #480 WS token-in-URL, #1797 draft, cross-endpoint back-port of the #2120 DB-role pattern.

_Delivery snapshot end._
