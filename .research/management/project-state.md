# Project State — 2026-08-09

**Sprint:** "Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"
**Sprint goal:** ship the announcement/document/notification/OAuth foundation MVP.
**Last routine run:** 2026-08-07 (44h ago). **Role focus today:** pm-scrum-master, pm-qa.

## Executive summary

19 PRs merged in the last 44h, dominated by (a) two P0-fixes closing recently-filed follow-up issues (SSRF `#2703`, memory-DoS `#2704`, scheduled-notification `#2612`), (b) a security gate on unauthenticated community reads, (c) two admin-web PATCH endpoints that unblock previously no-op Save flows, (d) five churn-hotspot refactors on auth/layout/reports, and (e) the announcement fan-out real-SQL metrics suite that closes a long-standing test-fidelity gap. **Zero release-blockers open.** Sprint is coverage-complete on all epics except 84 (2 partial frontend slices).

## Sprint progress

- **Epics done:** 47 stories / 49 across 13 epics — unchanged from 2026-08-06 upkeep. Two remaining `partial` slices (both epic-84, both frontend-only, backend shipped): `84-1` direct-to-S3 upload wiring, `84-2` signer-facing sign page.
- **Epic status:** 6 in-progress (3/6 stories done, remaining are done-in-code, sprint-yaml drift only), 7a in-progress (5/5 done — yaml drift), 8a done (3/3), 10a done (3/3), 10b in-progress (7/7 done), 80 done (3/3).
- **Buffer:** 36/36 open (target met).

## Shipped since last run (2026-08-07 → 2026-08-09)

- `#2710` **SSRF DNS-rebinding TOCTOU** fixed in workflow `api_call.rs` (closes `#2703`) — inline IG3 test.
- `#2707` **memory-DoS cap** on workflow response body (closes `#2704`) — inline over/under-limit tests.
- `#2714` **scheduled-notification decoupled** from publish/activate/close via watermark columns (closes `#2612`) — inline sqlx retries test.
- `#2722` **community reads gated** on principal+tenant — closes cross-tenant unauth read (retry 2/2 success).
- `#2723` **announcement fan-out metrics** — real-SQL suite (295 LOC) partially closes `risk-announcement-fanout-test-fidelity`.
- `#2718` layout webhook HMAC body-binding parity test (thin — replay-guard #2485 still open).
- `#2716` PATCH/GET `/api/v1/platform-admin/settings` — closes admin-web no-op save.
- `#2717` PATCH/GET `/api/v1/admin/mobile-config` — closes admin-web no-op save.
- `#2719` reality-server anonymous inquiry POSTs routed through `InquiriesHandler`.
- `#2712` dispute `add_evidence` access-audit event emitted (refs `#2483`).
- `#2709` reality-web `ListingForm` i18n via next-intl catalogs (5 locales).
- Churn refactors: `#2711` layout tenant dedupe, `#2713` layout admin dedupe, `#2715` auth-handler dedupe, `#2720` reports helpers extract, `#2721` `acquire_public_conn` extract.
- `#2696` reality-server inquiry-notifier seam (injectable) + DB-free unit tests.
- `#2705` rust-toolchain bump — **closed unmerged** (dependency noise).

## What's next (top 5 — from roadmap ranker)

1. [high, pm-frontend] **Wire ppt-web direct-to-S3 upload** via POST `/documents/upload-url` — 84-1 partial → done. Blocked-on `#2573` reference-check fix.
2. [high, pm-frontend] **Build signer-facing document-sign page** in ppt-web — 84-2 partial → done. Prior implementer attempt failed no-PR; fresh attempt scoped to the shipped API.
3. [medium, pm-security] **Add nonce+timestamp replay-guard** to layout webhook (`#2485`) — PR #2718 only pinned body-binding parity.
4. [medium, pm-backend] **Fix `#2573`** — DELETE-by-file-key can delete a still-referenced same-org object.
5. [medium, pm-mobile] **Fix `#2574`** — Android SSO `SsoStateStore.mint()` has no call site so every callback is rejected.

## Blockers

- **`#2573`** blocks 84-1 upload wiring — must land before frontend wires the direct-to-S3 path. Owner: pm-backend.
- **Reviewer capacity** on the accounting MVP-loop trio (`#2555/#2558/#2559`) — still sitting from 2026-07-30 with no reviewer engagement per the last SM note. Owner: pm-tech-lead.

## Decisions needed

- Adopt "inline `#[cfg(test)]` counts as coverage" for the routine's `hotfix-no-test` heuristic — owner: pm-tech-lead. (Raised 2026-08-09 by pm-qa; three fix PRs this window were false-positive-flagged.)
- Downgrade or close `risk-announcement-fanout-test-fidelity-2026-07-23` after spot-checking #2723 — owner: pm-qa.

## QA note (pm-qa 2026-08-09)

Coverage on the window is GREEN. The orchestrator's "three fix PRs had no tests" claim is a file-count heuristic false positive — inline `#[cfg(test)] mod tests { ... }` blocks in `api_call.rs` and `scheduler/mod.rs` carry the regressions. No systemic gap; refactor-without-new-test on 5 churn-hotspot PRs is acceptable (behaviour preserved) but should be flagged if the pattern accelerates. Full lens in `.research/management/roles/pm-qa.md`.
