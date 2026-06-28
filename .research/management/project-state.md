# PPT Project State

_Generated: 2026-06-28 — Phase 1.6 (12-day catch-up sweep; routine was stale 12 days)._

## Executive summary

Heavy delivery window. **281 PRs merged since #1439** (range #1440 → #1918) spanning 2026-06-16 → 2026-06-28. The biggest headline is Epic 6 (announcements) effectively closing out — 6-1/6-2/6-3/6-4/6-5/6-6 all reconciled to `done` via the #1832/#1834/#1835/#1843/#1844 cluster, with supporting screen-maps flipped to `shipped`. Epic 8a notification preferences shipped its screen-map batch (#1907-#1918). On the platform side, story 11.5 (Stripe Checkout, #1726), story 11.7 (financial statements, #1717/#1725), story 16.3 (saved-search alerts), tenant-migration (#1859), and regional-compliance (#1861) all landed.

Two pain points this run: **(1) dispatcher thrash on OCR-meter-reading test fix today** — four PRs (#1899-#1902) closed-not-merged before the canonical `replace sqlx::query!() with runtime query` fix took; **(2) 50 open PRs** dominated by the BIT-263 → BIT-282 test-backfill batch plus stuck drafts (#1812, #1814, #1833, #1846, #1857). The DRAFT IDOR fix #1857 (LLM document context) is a security blocker — it must merge this week. The routine itself sleeping 12 days means many PRs landed without per-PR review pipeline coverage; pm-security flagged residual risk below.

## Sprint progress

Sprint: **Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth**

| Epic | Done | Total | Status |
|------|------|-------|--------|
| epic-6 Announcements | 6 | 6 | **done** (just closed) |
| epic-7a Documents | 1 | 5 | in-progress (7a-2 CI-red review; 7a-3/4/5 ready-for-dev) |
| epic-8a Notification Prefs | 3 | 3 | **done** (screen-maps just landed) |
| epic-10a OAuth Provider | 0 | 3 | blocked on #481 (revoked-token bypass test) and #480 (WS token in logs) |
| epic-10b Platform Admin | 7 | 7 | **done** |
| epic-80 Disputes | 1 | 3 | partial (80-2 wizard redesign in flight, 80-3 submissions unwired) |

## Shipped since last run (top 15)

1. **#1832** — 6-1 announcement creation & targeting → done
2. **#1834** — 6-2 announcement viewing & acknowledgment → done
3. **#1835** — 6-3 announcement comments/discussion → done
4. **#1843** — 6-4 pinned announcements → done
5. **#1844** — 6-5 direct messaging → done
6. **#1822** — 79-2 SSO callback e2e
7. **#1726** — Stripe Checkout (story 11.5)
8. **#1717 / #1725** — financial statements (story 11.7)
9. **#1859** — tenant-migration endpoints
10. **#1861** — regional-compliance endpoints
11. **#1907-#1918** — 8a notification screen-maps batch
12. **#1741** — manager-gate Airbnb PII (security)
13. **#1750** — guest ID-document upload (security)
14. **#1737** — sensor WebSocket authn (security)
15. **#1779** — 81-2 report presigned-download hardening

## What's next (top 5)

1. **Merge #1857 (LLM document IDOR fix)** — pm-security flags as release-blocker; owner: pm-security
2. **Resolve #481 (OAuth refresh-token revocation test)** — gates 10a-1 / 10a-3; owner: pm-backend
3. **Resolve #480 (WS JWT in access logs)** — owner: pm-backend
4. **Fix OCR auth gap (#1772) + add 401 regression test** — owner: pm-backend
5. **Fix message attachment IDOR (#1791)** — no PR yet; owner: pm-security/pm-backend

## Blockers

- **#1680** — dispatcher infra: cron env can't run the pipeline (active)
- **#1014** — MCP push size limit blocks archive write-back (active)
- **#481** — OAuth refresh-token revocation bypass test missing — gates Epic 10A
- **#480** — JWT token exposed in WS access logs — gates Epic 10A + 8a-3 WS leg
- **#1857** — IDOR fix sitting in DRAFT 12d after routine pause; latent security exposure
- **Dispatcher thrash today (#1899-#1902)** — four superseded retries on OCR test fix; investigate as a process smell
- **Routine staleness (12d gap)** — pm-security flagged latent risk from PRs landing without routine review

## Role focus today

**pm-security + pm-scrum-master** (rotation: pm-security, last run 32d ago on 2026-05-27).

### pm-security summary
The sprint's highest-severity opens are unresolved OAuth refresh-token revocation bypass (#481, gates 10a-1/3) and WS JWT exposure in access logs (#480). PR #1857 (LLM doc IDOR fix) merge-readiness could not be confirmed via the proxy and must be manually verified. OCR endpoints have AuthUser extractors per handler but middleware mount is unverified (#1772). Booking.com integration logs PII at info level without redaction. The oauth_integration_tests.rs file is the run's highest-churn (2718 LOC, REPEAT) — signals unstable auth test logic. See `.research/management/roles/pm-security.md` for the full role JSON.
