# PPT Project State

_Generated: 2026-06-26 — daily PM rotation (Scrum Master + pm-security, 30 days overdue). Coverage `scan_kind=upkeep`; pm_cursor idx 5 → 6 (pm-data next), coverage_cursor idx 12 → 0 (epic-9 → wrap to epic-10a)._

## Executive summary

- **Reconciliation wave merged 2026-06-25** flipped 9 coverage stories to **done** via verify-and-finish PRs: 6-1, 6-2, 6-3, 6-4 (announcements), 6-5 (direct messaging — merged 2026-06-25), 10b-5 (support data + refresh_tokens column bug), 10b-7 (contextual help), 79-1 (API client), 79-2 (auth flow), 80-2 (dispute filing). Coverage rollup now **46/49 done, 3 partial, 0 not-started**.
- **epic-16 saved-search alerts shipped:** Story 16.3 cadence (BIT-140, PR #1847) + email/push transport drainer (BIT-139, PR #1849) + org-scoped favorite alert worker (PR #1850, BIT-242).
- **Messaging N-party group threads landed** end-to-end across backend (PR #1848, BIT-206) and web (PR #1853, BIT-244) — `participants[]` contract replaces 2-party `other_participant`.
- **Churn-hotspot refactor wave:** route/repo monoliths split into sub-modules — reserve_funds (#1816), iot (#1815), vendors (#1813), enhanced_tenant_screening (#1800), emergency (#1798), document (#1683), subscription (#1796), sensor (#1810), form (#1781). Reduces review surface for next churn pass.
- **Open security-hardening drafts (8):** #1797 (OCR auth), #1799 (attachment IDOR), #1801 (event-bus retry/Lagged), #1802 (N-party delivery/block/unhide), #1804 (payment-reminder dedup), #1806 (booking_channel DB-gate), #1807 (meter dedup), #1823 (guest PII). All are 2 days old — need pm-security decision this cycle.
- **Post-merge follow-up issues opened 2026-06-23..25:** #1851, #1852, #1828, #1827, #1826, #1793, #1792 — untriaged.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · epics_done=4/6 (epic-6, epic-7a, epic-8a, epic-10b)

| Epic | Real status (post-reconciliation) |
|---|---|
| 6 — Announcements & Communication | **6/6 stories done** (reconciled 2026-06-25 via PRs #1832/#1834/#1835/#1843/#1844) |
| 7A — Basic Document Management | **5/5 stories done** per coverage |
| 8A — Basic Notification Preferences | **done** — 8a-3 publish-leg tests landed; only #480/#484 stale issue follow-up |
| 10A — OAuth Provider Foundation | **0/3 stories complete** — blocked by #481, #482, #487 |
| 10B — Platform Administration | **7/7 stories done** (10b-5, 10b-7 reconciled this cycle) |
| 79 — UI integration | **done** (79-1, 79-2 reconciled this cycle) |
| 80 — Disputes | 1 partial (80-3 mediation party submissions, draft PR #1846 open) |

## Shipped since last run (29 PRs)

- **Reconciliation (story-status finish-to-done):** #1832 (6-1), #1834 (6-2), #1835 (6-3), #1843 (6-4), #1844 (6-5), #1845 (80-2), #1829 (10b-5 + refresh_tokens fix), #1831 (10b-7), #1830 (79-1), #1822 (79-2)
- **Feature work:** #1847 (16.3 cadence BIT-140), #1849 (alert drainer BIT-139), #1850 (org-scoped favorite worker BIT-242), #1848 (N-party participants BIT-206), #1853 (web group threads BIT-244)
- **Churn refactors (route/repo splits):** #1816, #1815, #1813, #1810, #1800, #1798, #1796, #1781, #1683
- **Infra / fixes:** #1809 (accounting 404+secrets), #1808 (accounting-web landing PAP-312), #1755 (migration 192 dup fix + CI guard), #1779 (report presigned-download hardening), #1752 (sqlx test-schema idiom docs)

## What's next (top 5 actions)

1. **[high] Land or close PR #1846 (80-3 mediation party submissions)** — pm-frontend. Last MVP partial story in epic-80.
2. **[high] Resolve test-hardening blockers #481 (OAuth refresh-token revocation) and #487 (MFA rate-limit)** — pm-backend. Ungates epic-10a stories 10a-1 + 10a-3.
3. **[high] Resolve issue #482 (ProtectedRoute multi-tenant role fallback + missing tests)** — pm-frontend. Ungates 10a-2.
4. **[high] Triage post-merge follow-up issues #1851/#1852/#1828/#1827/#1826/#1793/#1792** — pm-scrum-master. New batch needs owner assignment + severity label.
5. **[high] Review the 8 security-hardening draft PRs (#1797/#1799/#1801/#1802/#1804/#1806/#1807/#1823)** — pm-security. Prioritise IDOR (#1799) and PII (#1823) first.

## Blockers

- **epic-10a (OAuth) fully blocked.** All 3 stories gated by #481 (refresh-token revocation), #482 (ProtectedRoute), #487 (MFA rate-limit). Owners: pm-backend (#481/#487), pm-frontend (#482).
- **8a-3 notification preference sync** — #480 (WS JWT in logs) + #484 (notification dispatch serial + FCM stub) still open >30 days. Owner: pm-backend.
- **80-3 mediation party submissions** — PR #1846 open as draft. Owner: pm-frontend.
- **8 security-hardening drafts unmerged** (#1797/#1799/#1801/#1802/#1804/#1806/#1807/#1823) — IDOR + PII items highest severity. Owner: pm-security.

## Role focus today

- **pm-scrum-master** (always-on synthesis)
- **pm-security** (monthly rotation, 30 days overdue — first run since 2026-05-27)

### pm-security one-line summary

> Dense cluster of unmerged security-hardening drafts covers auth gaps (unauthenticated OCR endpoints, JWT role vs DB manager gate on booking_channel, 3rd unmigrated JWT verification copy), PII exposure (guest booking reads, guest ID-doc upload without content-sniff or audit), and broken integrity guarantees (attachment IDOR, OAuth refresh-token revocation bypass still open as #481, event-bus at-least-once defeated by Lagged drops). None of the 8 high-priority hardening PRs are merged.
