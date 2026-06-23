# PPT Project State

_Generated: 2026-06-23 — daily PM rotation (Scrum Master + pm-security inline). Coverage `scan_kind=upkeep`; pm_cursor idx 5 → 6 (pm-security → pm-data), coverage_cursor idx 12 → 0 (epic-9 → epic-10a)._

> **Routine catch-up run.** The cloud routine did not run from 2026-06-17 through 2026-06-22 (7d gap). This refresh consolidates 96 merged PRs / ~250+ commits / 37 issue updates across that window and the prior brief's incident is fully resolved (see *Resolved since last run*).

## Executive summary

- **`dev` backend RED incident is closed.** The voting PDF-report test failure (BIT-158, issue #1645) that wedged backend CI since 2026-06-15 was unblocked by **#1656** (isolate report tests behind `bare-#[sqlx::test] + db::run_migrations` idiom) and then **#1734** (repair the test setup regression). Test-infra documentation follow-up #1665 is in flight as #1752 (open).
- **Second migration-collision incident this sprint.** Two migrations independently claimed version `00187` (fixed by #1724, BIT-223) and then `00192` (fixed by #1757). #1755 (open) wires a `check-migration-versions.sh` guard into the required `check` job — until that lands, the duplicate-version class can reoccur on any backend PR.
- **Massive ship rate.** 96 merged PRs in 7 days (avg ~14/day) — heaviest of the sprint. Headlines:
  - **IoT epic 14** end-to-end: WS channel (#1640/#1644 then deduped via #1737), sensor registry/edit (#1646), per-sensor thresholds (#1653), standalone alerts page (#1652), real-time dashboard via WS (#1685), i18n parity (#1649/#1650/#1732). One genuine **security regression caught** (#1668 → #1737): a duplicate JWT-trusting sensor WS handler that bypassed DB membership checks was deleted in favor of the DB-checked handler.
  - **Financial epic 11** AR/AP closure: invoice PDF generation (#1648 / FE #1654 / loading-state fix #1733), Stripe Checkout (#1726), financial-statement reports + PDF/xlsx (#1717 / FE #1725), payment-reminder scheduler + auto-overdue transitions (#1709).
  - **Messaging UC-05.x catch-up:** N-party group conversations (#1689), attachments S3 presigned upload (#1702) + ppt-web wiring (#1712) + camelCase wire-contract fix #1756 (open follow-up #1768 for the rest of the messaging surface).
  - **Epic 3 buildings:** unit management UI (#1695), building geocoding (#1691), map view (#1711), resident "My Unit" (#1701), person-month tracking (#1714).
  - **Dispatcher infrastructure:** two systemic-blocker issues (#1739, #1747) fixed by #1751 (archive-terminal reconciler + unique branch names + collision guard). Issue #1680 (cron environment can't run core pipeline — fire-and-forget implementers die, no DB, git-push blocked) remains **open** — infra-side fix needed.
- **Security wins this rotation (pm-security inline):**
  - **#1741** added manager-role gate to live Airbnb `/reservations` proxy (guest PII protection, follow-up #1667 → closed). The **persisted** rental booking/guest PII reads remain unguarded; tracked as #1766 (open follow-up).
  - **#1746** validates portal-listing enums/status + adds ownership IDOR tests (closes #1671). Follow-up #1762 flags a silent downgrade-to-draft bug on edit.
  - **#1737** removed duplicate JWT-trusting sensor WS channel — converged on DB-checked handler. Follow-up #1763 tracks the missing non-member 403 integration test.
  - **#1684** propagates CSPRNG failure instead of `expect()` — no more panic on encrypt path.
  - **#1744** unified the access-token verification path (closes #1675); follow-up #1761 (carry-over of finding 3) is open.
  - **#1753** added fail-fast preflight for required production env vars (closes #951). **Gap:** preflight only checks presence, not length; follow-up #1758 requests length floors for `JWT_SECRET` / `ESIGN_TOKEN_SECRET`.

## Resolved since last run

- **Issue #1437** (dev backend RED, voting PDF tests) → closed by #1656 + #1734 (re-fix); #1645 (the umbrella tracking issue) closed.
- **dev RED from migration collision #00187** → resolved by #1724.
- **dev RED from migration collision #00192** → resolved by #1757; CI guard in #1755 (open).
- **Dispatcher fire-and-forget implementers dying / branch-collision** → resolved by #1751 (#1739, #1747 closed).
- **Mobile env-var single-source-of-truth** + RN icon assets → #1658, #1659, #1745 (icons), #1748 (typecheck-fix), #1738 (regression-guard test). Most follow-up issues #1662 through #1670 closed.
- **3 churn hotspot items** auto-resolved by split PRs (now `done` in backlog): `forms.rs` (#1700), `aml_dsa.rs` (#1708), `booking.rs` (#1693).

## Sprint progress

| Epic | Status | Notes |
|---|---|---|
| 3 — Buildings/Units | major progress | Unit mgmt UI, geocoding, resident view, map all shipped this window |
| 6 — Announcements & Communication | stable | No new movement this window |
| 7A — Documents | progress | E-signature surface on document detail (#1697); template generation UI (#1707) |
| 7B — Document Lifecycle | progress | #1707 (template gen UI), #1697 (e-sig surface) |
| 8A — Notifications | near-done | Fault lifecycle hooks (#1705), 24h-inactivity digest worker (#1699), delivery analytics (#1710 + ops dashboard #1722) |
| 10A — OAuth Provider | unchanged | #1752 (test-infra doc) only related work |
| 11 — Financial | major progress | Stripe Checkout, invoice PDFs, financial reports, payment-reminder scheduler all landed |
| 12 — AML/DSA | stable | Meter reminders + OCR stub (#1703) |
| 13 — AI dashboards | wired | Sentiment + Predictive Maintenance (#1641); routing fix via api-client (#1736) |
| 14 — IoT | near-done | WS channel, sensor registry, thresholds, alerts page, real-time dashboard, i18n parity all landed |
| 15 — Portal users | progress | #1642 owner/realtor edit |
| 18 — Guest ID OCR | start | #1750 (DB migration + storage) — but introduced the 00192 collision; #1760 follow-up open |
| 82 — Mobile (Reality KMP) | unchanged | No movement this window |

## What's next (top 5 actions)

1. **[high] Land #1755** — dup-version CI guard. Without it the 00187/00192-class incident can reoccur on any backend PR (we've had 2 in 14 days). pm-devops.
2. **[high] Triage #1680** — research dispatcher cron environment broken: implementers die fire-and-forget, no DB access, git-push and archive-push blocked. Infra-side fix; this is the daily-loss-of-throughput-multiplier issue. pm-devops + infra.
3. **[high] Address messaging API surface — #1768** (camelCase wire contract for thread/message/participant endpoints). #1756 fixed attachment; #1768 finishes the job. pm-backend.
4. **[medium] Wire up the 19 open follow-up issues #1758-#1773** from the latest post-merge review round — many are short, security-flavored, and high-value (manager-gate persisted booking reads, MIME validation, JWT secret length floors, etc.). pm-scrum-master to assign.
5. **[medium] Confirm catch-up state of cloud routine.** This 7-day lag indicates the daily cron is not running. The research dispatcher loop (separate state) appears alive per #1751. Cloud routine clock must be checked independently of dispatcher state. pm-devops.

## Stalled / risky

- **#1683** (auto-impl document.rs split) — open as DRAFT since 2026-06-22; needs a human decision on whether to merge before the file churns further.
- **#1720** (Leases nav link) — open ~24h, no review; tiny PR.
- **#1754** (admin-web stale-fallback cleanup) — open as DRAFT, no review.

## Open questions

- Why is the cloud routine cron not firing? (separate from dispatcher loop)
- Should the `evidence`-gone resolution check be expanded to consume PR file-list data for higher-fidelity automated resolution detection? Without it, this run had to mark hotspot items resolved by hand-checking `ls`.
