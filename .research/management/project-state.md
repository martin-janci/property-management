# PPT Project State

_Generated: 2026-06-26 — daily PM rotation (Scrum Master + pm-security; 10-day catch-up window 2026-06-16→2026-06-26)._ _Coverage `scan_kind=upkeep` (epic-9 re-checked, status unchanged: done/high)._ _pm_cursor idx 5 → 6 (pm-data next); coverage_cursor idx 12 → 0 (full wrap, next: epic-10a)._

## Executive summary

- **262 PRs merged in 10-day window** (#1440–#1853). Massive status-reconcile sweep landed (PRs #1832/#1834/#1835/#1843/#1844/#1845/#1830/#1829/#1831) flipping 6-1/6-2/6-3/6-4/6-5/80-2/79-1/10b-5/10b-7 to done. **Sprint Master headline: sprint-status is stale on these promotions** — next deep coverage scan will likely flip the `partial` count from 12 → ~3.
- **pm-security rotation (30-day stale, longest in rotation)** matches the window's security-heavy theme: 18 auth-hardening PRs landed (unified verification path #1744, sensor WS dedup #1737, Airbnb manager-gate #1741, OTA gate #1552, public-client secret rejection #1539, mfa RLS+IDOR #1467, cross-tenant OAuth upsert #1473, 10b-5 support-data-access done #1829). **But 10 open `follow-up,from-merged-review` security issues remain** — heavy IDOR/PII/JWT theme: message attachment file_key IDOR (#1791), 3rd JWT verify copy (#1782), OCR unauthenticated (#1772), booking guest PII manager-gate (#1766).
- **In-flight critical work**: 80-3 mediation party-submissions (#1846 in review), 84-5 pgvector-RAG (#1833 quarantined), accounting MVP N1-N5 (#1453/#1454).
- **THB story-gates still open 32+ days**: #480 (WS JWT in logs), #481 (OAuth refresh-token revocation bypass), #484 (serial FCM), #487 (MFA rate-limit). These continue to block 8a-3 and 10a-1/10a-3 formal promotion.
- **155 issues opened this window**; 36 currently open (most are `follow-up,from-merged-review` from the dispatcher post-merge reviewer).

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **Epic 6, 7A, 8A & 10A - Announcements, Documents, Notifications & OAuth** · epics_done=4/6.

| Epic | Tracked status | Real status (from coverage + window activity) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | **All 6 stories reconciled to done** via PRs #1832/#1834/#1835/#1843/#1844 — sprint-status flip pending |
| 7A — Basic Document Management | in-progress | Coverage marks 5/5 done with high confidence — sprint-status counter stale |
| 8A — Basic Notification Preferences | partial | 8a-3 mobile push (FCM/APNs) still open; THB #480/#484 gate full promotion |
| 10A — OAuth Provider Foundation | in-progress | Coverage marks 3/3 done; THB #481 (revoke bypass) + #487 (MFA rate-limit) gate |
| 10B — Platform Administration | in-progress | 10b-5 + 10b-7 reconciled to done via #1829/#1831; coverage marks 7/7 done |
| 80 — Disputes | in-progress | 80-2 reconciled (PR #1845); 80-3 mediation party-submissions in PR #1846 review |
| 82 — Mobile (Reality KMP) | in-progress | Multiple mobile/auth slices landed; 82-3 infinite scroll evidence still owed |
| 84 — pgvector / E-signature | partial | 84-5 RAG retrieval service not implemented (PR #1833 quarantined) |
| 85 — Mobile Build Pipeline | in-progress | EAS workflow files in repo; pipeline verification still owed |

## Shipped since last run (cursor #1439 → #1853, 262 PRs · 10-day catch-up)

- PR #1849 -- Saved-search alert drainer
- PR #1850 -- Epic 16 alert worker
- PR #1853 -- BIT-244 messaging participant list fix
- PR #1848 -- BIT-206 messaging participant list
- PR #1846 -- Epic 80-3 mediation (in review)
- PR #1833 -- Story 84-5 pgvector-RAG (quarantined)
- PR #1829 -- Story 10b-5 support-data-access done
- PR #1822 -- Story 79-2 SSO callback e2e
- PR #1768 + #1756 -- camelCase wire flip for messaging
- PR #1746 -- Portal-listings IDOR tests
- PR #1744 -- Auth verification path unified
- PR #1741 -- Airbnb reservations manager-role gate / guest PII
- PR #1737 -- Duplicate JWT-trusting sensor WS removed
- PR #1713 -- Revert broken delegations
- PR #1642 -- Epic 15 portal-user CRUD
- PR #1621 -- Payment-matching auth fix
- PR #1600 + #1563 + #1460 -- Developer OAuth RLS probes
- PR #1552 -- OTA token-exchange manager-gate
- PR #1539 -- Public-client secret rejection
- PR #1530 -- Security-test-gate as required CI check
- PR #1473 -- Cross-tenant OAuth upsert hazard fixed
- PR #1467 -- MFA recovery-code RLS + IDOR test
- PR #1454 + #1453 -- Accounting MVP N1-N5

## What's next (top 5 actions — from coverage rank + Scrum Master synthesis)

1. **[high] Remediate the security follow-up debt** — file fixes for #1791 (message attachment file_key IDOR), #1782 (3rd JWT verify copy), #1772 (OCR unauthenticated), #1766 (booking guest PII manager-gate). Owner: pm-security + rust-backend.
2. **[high] Reconcile sprint-status.yaml** to match coverage — flip Epic 7A (7a-2…7a-5), Epic 10A (10a-1/2/3), Epic 10B (10b-5, 10b-7) to done; update epic stories_completed counters. Owner: pm-scrum-master.
3. **[high] Close or formally defer THB batch #480/#481/#484/#487** — these have been open 32 days and are the only remaining gates blocking 8A and 10A formal closure. Owner: rust-backend + pm-scrum-master.
4. **[high] Fix OAuth refresh-token revocation bypass (#481)** — restore `revoked_at IS NULL` predicate; RFC 9700 compliance. Owner: rust-backend.
5. **[medium] Land mediation party-submissions (#1846)** and complete 80-3 to done. Owner: pm-frontend.

## Blockers

- **Epic 8A stories 8a-3 (notification-preference-sync)** — THB batch items #480 (WS JWT token in access logs, high severity) and #484 (serial FCM dispatch / false sent count) are story-gates that have been open 32+ days with no recorded fix PR — owner: rust-backend
- **Epic 10A stories 10a-1, 10a-3 (OAuth authorization server, token management)** — THB batch items #481 (OAuth refresh-token revocation bypass, high severity -- breaks RFC 9700) and #487 (MFA rate-limit test gap) are story-gates; #481 has been open 32+ days — owner: rust-backend
- **Epic 80 story 80-3 (mediation-resolution)** — Party submissions endpoints remain unwired in MediationPage.tsx; dispute-detail screen-map apiStatus stays partial; story cannot be promoted to done until submissions integration is complete — owner: pm-frontend

## Role focus today: **pm-security** (+ pm-scrum-master always-on)

- **pm-security** (rotation idx 5, last 2026-05-27, **30d stale — longest in rotation**): 6 new next_actions + 5 new risks + 4 new decisions_needed appended. Full role JSON in `.research/management/roles/pm-security.md`. Headline: 10 open follow-up security issues — IDOR/PII/JWT theme — accumulating faster than fix rate; pm-security recommends dedicated security-sprint slice before next dev→main release gate.
- **pm-scrum-master** (always-on): produced the delivery synthesis above; headline = 10-day catch-up window cleared the 6-1…6-5 + 79-1 + 10b-5 + 10b-7 + 80-2 status-reconcile backlog via 9 reconcile-PRs; sprint-status board lag is the immediate hygiene task.

## Coverage (upkeep — 2026-06-26)

- **`coverage.json` upkeep cursor advanced**: epic-9 (TOTP 2FA Setup) re-checked. Status unchanged: `done` / `high` confidence. Story `9-1-totp-2fa-setup` shipped (backend mfa router + tests + ppt-web TwoFactorAuthPage + mobile TwoFactorScreen, screen-map `ppt/settings-two-factor` buildStatus=shipped/apiStatus=complete). `last_checked` bumped to 2026-06-26.
- **Coverage cursor wraps**: idx 12 (epic-9) → 0 (epic-10a) for next run.
- **Note**: full coverage.json (49 stories across 13 epics) is now ~3 days old (deep scan 2026-06-23). The 9 status-reconcile PRs (#1832-#1845, #1829-#1831) merged in the window are NOT yet reflected — next deep scan will flip Epic 6 (6-1..6-5), 10b-5, 10b-7, 79-1 from `partial` to `done`, dropping the partial count from 12 to ~3.

