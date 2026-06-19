# PPT Project State

_Generated: 2026-06-19 04:45 UTC — coverage-refresh upkeep run, driven by routine fire payload "buffer-low + coverage-stale". `scan_kind=upkeep`. Phase 1.6 role rotation deferred (Phase 1.5/1.6 skipped in this catch-up routine); coverage_cursor unchanged._

## Executive summary

- **Cloud routine missed 3 daily runs** (state.last_run_iso=2026-06-16T03:25Z → 2026-06-19T04:30Z). Coverage.json was stale; the dispatcher emitted 14 quarantine rows in that window because every refill claim mapped to already-shipped work. **This run refreshed coverage.json against the 113 PRs merged in the gap window — 16 stories advanced to `done`; 4 remain partial.**
- **Action-list reconciliation:** 8 of 9 action items at HEAD have been silently shipped on `dev` and are now flipped to `status: done` with PR or dev-codebase verification evidence. 3 new action items added for the genuinely remaining work (6-2 web viewing/ack, 82-3 debounced-search/CoreLocation, 85-1 env-var setup). Net open: **4** (down from 9, but with real work — not phantom).
- **Backend security hardening sweep dominated the window:** RLS / IDOR / FORCE-RLS fixes across appeals, MFA recovery, developer OAuth tables, dispute add_evidence/add_comment, PortalPrincipal admit gate, OTA manager-gate (BIT-73/75/76/78/85/98). 12+ merged PRs in this theme.
- **Booking integration matured to done:** OTA HotelRateAmountNotif RQ/RS models (#1577), rate/availability push flow (#1560), XXE/DTD non-vacuous guards (#1599), encrypted credentials at rest (#1474). Story 83.2 closed.
- **Airbnb integration verified done** against dev codebase (not only PR titles): OAuth + webhook + /integrations/airbnb/* routes all present.
- **Mobile:** RN folder rename/move (#1557), iOS deep-linking + URL schemes confirmed (#1516), reality-mobile FavoritesService session-gating (#1541), AnnouncementsScreen TZ pin (#1543). FaultsPage delete + statistics wired (gap-79-1 / #1555).
- **CI gap surfaced (issue #1538, OPEN):** backend `test` job is **not** a required check on `dev`; red-test PRs can merge freely and re-red the merge pipeline. New `dx` backlog vector tracking it.

## Sprint progress (against `coverage.json` refreshed 2026-06-19)

Current sprint: **"Epic 6, 7A, 8A, 79, 80, 81, 82, 83, 84, 85"** · stories_done=45/49 (92%) · stories_partial=4/49.

| Epic | Stories done / total | Notes |
|---|---|---|
| 6 — Announcements & Communication | 4 / 5 | 6-3 comments (#1559) + 6-4 pinned (#1446) shipped; 6-2 viewing/ack web UI still partial |
| 7A — Basic Document Management | 4 / 5 | 7a-1 upload-metadata (#1550 verify) + 7a-2 folder-org mobile (#1464/#1557/#1582) + 7a-4 download/preview (#1463/#1480/#1551) all done |
| 8A — Basic Notification Preferences | 3 / 3 | 8a-3 mobile push (FCM/APNs) shipped via #1556. Epic now fully done. |
| 79 — Web Feature Completion | 1 / 1 | gap-79-1 closed (#1555 FaultsPage + #1446 AnnouncementsPage) |
| 80 — Disputes & Mediation | 2 / 2 | dispute filing autosave + i18n (#1468/#1544) |
| 81 — Reports & Schedules | 2 / 2 | cron_expression column (migration 00166) + execution-history retry (#1548) |
| 82 — Mobile (Reality KMP) | 3 / 4 | 82-1 setup + 82-2 navigation + 82-4 favorites done; 82-3 home/search debounce + CoreLocation still partial |
| 83 — Integrations (Airbnb + Booking) | 2 / 2 | Both stories verified done (dev-codebase + PR evidence) |
| 84 — E-Signature | story 84-2 done | Webhook idempotency terminal-state guard #1558 |
| 85 — Mobile Build Pipeline | 0 / 2 | 85-1 env-vars + 85-2 build-config (app icons) both still partial — PR #1554 open draft pending binary PNG commits |

## Shipped since last run (113 merged PRs, range #1440–#1603)

High-impact themes (full PR list in brief `.research/briefs/2026-06-19.md`):

- **Security / RLS / IDOR hardening (18 PRs):** #1457 (BIT-75 appeals org-scope), #1467 (MFA recovery RLS + IDOR test BIT-78), #1460 (FORCE RLS developer OAuth tables BIT-76), #1473 (BIT-85 cross-tenant OAuth upsert), #1500 (BIT-73 dispute/violation add_evidence), #1561 (PortalPrincipal admit gate), #1601 (OTA manager-gate TenantRole), #1602 (MFA recovery limiter audit), #1603 (PortalPrincipal regression pin), #1472/#1474 (Booking.com creds at-rest), #1463/#1480 (document scoped download/preview gate).
- **Booking integration / OTA (9 PRs):** #1577 (typed OTA RateAmountNotif), #1560 (combined OTA rate/availability push flow), #1485 (timeout + RateLimited), #1486/#1492/#1505/#1537 (rental enum text-cast sweep PAP-158), #1562/#1599 (XXE/DTD non-vacuous guards).
- **Schema-drift cleanup / enum decode (12 PRs):** #1469 (schema-drift SQL), #1481/#1487 (budget reserve-fund retire), #1496/#1498 (form_status cast), #1565 (document u.name).
- **Mobile RN + KMP (10 PRs):** #1447 (RN infinite scroll), #1465 (iOS stale-response guard + entitlements), #1542 (iOS SearchView pagination), #1541/#1543 (mobile-native favorites + AnnouncementsScreen TZ pin), #1555 (FaultsPage delete/stats), #1582 (folder tree wiring), #1557 (mobile folder rename/move UI).
- **Process / CI (5 PRs):** #1441 (pre-push fmt+clippy gh-1375), #1452 (dev-push compile gate), #1455 (CI fmt+clippy mirror), #1490 (RLS baseline --strict).
- **Dependency noise (7 PRs):** #1449, #1566, #1570-#1573, #1575-#1576.

## What's next (top 5 actions — derived from refreshed action-list.json + open backlog)

1. **[high] Fix the cloud routine scheduler.** 3 daily runs missed (2026-06-17, 2026-06-18, 2026-06-19) before this catch-up. Issue #1604 (dispatcher backlog exhausted) is downstream of the same root cause. Owner: operator. Action: verify claude-code-on-the-web cron / routine trigger is healthy; re-arm if paused.
2. **[high] Make backend `test` job a required check on `dev`** (issue #1538). Owner: pm-devops. Red-test PRs are merging freely and re-redding the merge pipeline.
3. **[medium] Implement Coverage 6-2 web viewing/acknowledgment UI on ppt-web** (new action-list item). Owner: pm-frontend. Mobile shipped long ago; web side still references draft PRs #474/#475/#479 that never merged.
4. **[medium] Close Coverage 82-3 residual gaps** (debounced search AC-2 + CoreLocation + FilterSheet) on reality-mobile. Owner: pm-frontend. PR #1447 evidenced infinite scroll only; coordinate with existing backlog row `code-review-mobile-native-kmp-search-stale-response-race`.
5. **[medium] Drive 6 already-`ready` plans through the implementer.** `security-llm-doc-idor`, `bug-ios-searchview-uncompilable`, `code-review-reality-web-share-comparison-404`, `code-review-reality-web-listing-page-ssr-crash`, `code-review-reality-web-realtor-mgmt-untranslated`, `code-review-mobile-rn-report-fault-fake-submit`. They've been sitting at `status: ready` for 3–18 days.

## Blockers

- **Cloud routine scheduler paused / drifted.** 3-day gap. Root cause unknown until operator inspects. Until restored, dispatcher will keep emitting quarantine-ready claims because coverage.json refreshes only via this routine path.
- **PR #1554 (mobile app icons) needs binary PNG commits.** The auto-impl wrote all the scaffolding (ImageMagick script, SVG source, Contents.json asset manifests) but `mcp__github__push_files` cannot commit PNGs — maintainer must run the script and commit. Story 85-2 stays partial until then.

## Role focus today: **none — Phase 1.6 skipped this run** (catch-up routine focused on coverage refresh)

- `pm_cursor.next_index` unchanged (still 5, pm-security next). Will resume normal rotation on the next routine run.
- `coverage_cursor.next_index` unchanged (still 12). The mass refresh in this run touched 16/13 epics anyway, so the rotating-recheck cycle restarts effectively fresh.

## Coverage upkeep (this run)

Cross-matched all 20 partial stories against the 113 merged PRs (window 2026-06-16 → 2026-06-19) via the patterns: explicit story-id, `Story X.Y`, `Coverage X-Y`, `gap-X-Y`, `gap-X.Y`. Verified airbnb integration + cron_expression column directly against dev codebase. Results:

- **16 stories advanced from `partial` → `done`:** 6-3, 6-4, 7a-1, 7a-2, 7a-4, 8a-3, 79-1, 80-2, 81-1, 81-2, 82-1, 82-2, 82-4, 83-1, 83-2, 84-2.
- **4 stories remain `partial`** with refreshed `last_checked=2026-06-19` and live gap lists: 6-2 (web viewing/ack UI), 82-3 (debounced-search/CoreLocation residuals), 85-1 (env-vars), 85-2 (build-config app icons — PR #1554 open).
- **0 stories regressed**; all status changes were partial → done.

Next routine run: resume rotating-epic coverage_cursor at idx 12 (epic-9 if present in the sorted distinct epic set; otherwise wrap).
