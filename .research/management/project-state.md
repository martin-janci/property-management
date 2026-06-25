# PPT Project State

_Generated: 2026-06-25 — daily PM rotation (Scrum Master + pm-security; coverage upkeep). pm_cursor idx 5 → 6 (pm-data next); coverage_cursor idx 12 → 0 (epic-9 → epic-10a)._

## Executive summary

- **High-volume delivery window (95 PRs merged since 2026-06-16, range #1567–#1829)** spanning Epic 6 messaging (#1689 N-party, #1696 archive, #1702 attachments, #1768/#1756 camelCase), Epic 11 financial (#1726 Stripe Checkout, #1709 reminders+auto-overdue, #1725/#1717 statement reports + PDF/xlsx, #1733/#1654 invoice PDF), Epic 3 residents (#1701 My Unit, #1714 person-month, #1690 delegations, #1695 unit-mgmt, #1711 building map, #1691 geocoding), Epic 4 faults (#1705 notifications, #1715 mobile offline queue, #1704 reports), Epic 7B (#1707 templates, #1697 e-sig), Epic 79 (#1822 SSO callback e2e), and **10b-5 promoted to done via #1829** (refresh_tokens column bug fix + sprint-status reconcile).
- **New ACC (Accounting) workstream bootstrapped** — spec #1817, accounting-web public/signup #1808, PAP-321 #1809, plus six open drafts (#1821/#1820/#1805/#1803/#1818/#1811). No sprint plan, story estimates, or owner assignments yet — workstream risk flagged.
- **Architecture cleanup flood** — many churn-hotspot route/repo splits landed (emergency #1798, vendors #1813, iot #1815, enhanced_tenant_screening #1800, subscription #1796, sensor #1810, forms #1700, reserve_funds #1816, aml_dsa #1708, document #1683, financial via split + #1781 SQL collapse).
- **DB migration discipline restored** — duplicate version collisions 00187 (#1724) and 00192 (#1755/#1757) resolved; CI guard added to block future dup-version regressions.
- **Heavy follow-up backlog from post-merge reviews** — 30+ open `follow-up,from-merged-review` issues #1663–#1828 include security-class items: attachment IDOR (#1791), OCR unauthenticated (#1772), guest PII reads (#1766), payment reminder dedup (#1769/#1790), N+1 in start_thread (#1776), unread soft-delete (#1771), third JwtService copy (#1782), camelCase mobile break (#1780).
- **pm-security rotation today (29 days stale)** surfaced three live release-blocking gaps not previously tracked: unauthenticated `/api/v1/ai/ocr/*` endpoints (confirmed in source), third unmigrated `JwtService` token-verifier, and OAuth refresh-token revocation bypass (#481 — RFC 9700 break).

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · epics_done=2/6 — sprint-status materially stale vs coverage.json reality (epic-10a all 3 stories done per evidence; 7a-2…7a-5 done; 10b-7 done; flagged as a tracked action this run).

| Epic | Tracked status | Real status (from coverage + activity) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6-5 evidence strong via #1689/#1696/#1702 (N-party, archive, attachments); 6-1..6-4 promotion-lag only |
| 7A — Basic Document Management | in-progress | Coverage suggests 7a-2..7a-5 done; sprint-status lags |
| 8A — Basic Notification Preferences | near-done | Per previous run; 8a-3 still gated by #480 (JWT in WS logs) |
| 10A — OAuth Provider Foundation | in-progress | Coverage marks 10a-1..10a-3 done; **gated by #481 OAuth revoked-token bypass** |
| 10B — Platform Administration | in-progress | **10b-5 now done via #1829**; 10b-7 promotion-lag |
| ACC — Invoicing & Accounting (new) | not-yet-in-sprint | Spec + first PRs landed; needs sprint-0 scoping |

## Shipped since last run (cursor #1439, 95 PRs)

- **#1829** — 10b-5 Support Data Access: refresh_tokens column bug fix + sprint-status reconcile to done [pm-security/pm-backend]
- **#1822** — 79-2 Authentication Flow: reality-web SSO callback e2e coverage [pm-frontend]
- **#1817** — ACC spec (17 epics, ~140 UC-ACC-XX.Y) merged
- **#1809** — accounting PAP-321 F1/F2 — 404-not-500 + skip-serializing provider secrets [pm-backend]
- **#1808** — accounting-web public marketing/landing + signup (PAP-312) [pm-frontend]
- **#1798/#1813/#1815/#1800/#1796/#1810/#1700/#1816/#1708/#1683/#1781** — backend churn-hotspot route/repo splits and SQL collapses [pm-backend]
- **#1755/#1757/#1724** — DB duplicate migration-version collision fixes + dup-version CI guard [pm-backend/pm-devops]
- **#1752** — sqlx test-schema idiom documented + migrator no-op gotcha pinned
- **#1689 / #1696 / #1702 / #1768 / #1756** — Epic 6 messaging (N-party, archive, attachments, camelCase wire) [pm-backend/pm-frontend]
- **#1726 / #1709 / #1725 / #1717 / #1733 / #1654** — Epic 11 financial (Stripe Checkout, reminders+auto-overdue, statement reports, PDF/xlsx export, invoice PDF) [pm-backend/pm-frontend]
- **#1701 / #1714 / #1690 / #1695 / #1711 / #1691** — Epic 3 residents (My Unit, person-month, delegations, unit-mgmt, building map, geocoding) [pm-frontend/pm-backend]
- **#1705 / #1715 / #1704** — Epic 4 faults (lifecycle notifications, mobile offline queue, reports) [pm-backend/pm-frontend]
- **#1707 / #1697** — Epic 7B (document-template generation, e-signature surface) [pm-frontend]
- **#1779** — Epic 81 report presigned-download hardening [pm-backend]

## What's next (top 5 actions)

1. **[high] Reconcile sprint-status.yaml to match coverage.json reality** — flip 10a-1/10a-2/10a-3, 7a-2..7a-5, 10b-7 to `done`; epic-10a status `done`. Owner: pm-scrum-master. Why: produces false blockers in automated gates and misleads capacity planning.
2. **[high] Fix OAuth refresh-token revocation bypass (issue #481, RFC 9700 break)** — restore `revoked_at IS NULL` predicate in `oauth.rs` refresh; add regression test. Owner: pm-security. Why: gates 10a-1/10a-3 promotion; live session-hijack vector.
3. **[high] Add auth extractor to `/api/v1/ai/ocr/*` endpoints** — `process_meter_reading` and `submit_correction` callable without any token (confirmed in `routes/ai/ocr.rs`). Owner: pm-security. Why: unauthenticated surface in production-facing API.
4. **[high] Migrate third `JwtService` token-verifier copy (issue #1782)** to the unified path from #1744. Owner: pm-security. Why: divergent verifier may pass forged/mistyped tokens that the canonical path would reject.
5. **[high] Close test-hardening batch #480 (JWT in WS access logs)** — mask token or move to `Authorization` header. Owner: pm-security. Why: 31 days open; token harvestable from log aggregation; gates 8a-3.

## Blockers

- **OAuth refresh-token revocation bypass (#481, 31d open, sev=high).** Owner: pm-security. Gates 10a-1/10a-3 to staging.
- **Unauthenticated `/api/v1/ai/ocr/*` endpoints.** Owner: pm-security. Live in API surface; only the 501-stub status limits exploit value.
- **JWT exposed in WebSocket query-param access logs (#480, 31d open, sev=high).** Owner: pm-security. Gates 8a-3.
- **Sprint-status.yaml out of sync with coverage.json reality** (multiple done stories still labeled `ready-for-dev`). Owner: pm-scrum-master.
- **ACC workstream lacks sprint plan / owners** — 6+ draft PRs in flight without scoping. Owner: pm-scrum-master + pm-backend.

## Role focus today: **pm-security** (+ pm-scrum-master always-on)

- **pm-security** (rotation idx 5, last 2026-05-27, 29d stale): 6 next_actions appended to `action-list.json`; 5 new risks appended to `risks.json`; 2 new decisions in `decisions.md`. Full role JSON in `.research/management/roles/pm-security.md`. Headline: three live release-blocking gaps (unauthenticated OCR, third JwtService unmigrated, OAuth #481 revoked-token bypass) + PII logging audit due on guest ID-OCR (#1783).
- **pm-scrum-master** (always-on): delivery synthesis above; headline = 95 PRs in 9-day window, Epic 11 financial surge nearly complete, 10b-5 closed via #1829, but sprint-status reconciliation overdue and 30+ follow-up issues need triage.

## Coverage (upkeep — 2026-06-25)

- **`coverage.json` upkeep** (scan_kind=upkeep, generated 2026-06-25): 10b-5 promoted partial → done via #1829; 79-2 evidence appended for #1822 (SSO callback e2e); 6-5 evidence appended for messaging PRs (#1689/#1696/#1702/#1768/#1756). **49 stories: 38 done · 11 partial · 0 not-started** across 13 epics.
- **Buffer health**: `action-list.json` open=36/36 after refill from gap-scan candidates (5 churn-hotspot items closed by merged splits; 5 inflight closed by merged splits; 6 pm-security + 6 pm-scrum-master next_actions added; gap-scan refill brought buffer to target).
- **Next deep scan recommended** — coverage_cursor advanced 12 → 0 (wrap), so we've completed one full rotation; consider a `pm-scan` invocation locally if drift suspected before next rotation pass.
