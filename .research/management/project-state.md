# PPT Project State

_Generated: 2026-07-02 — catch-up PM synthesis (Scrum Master + pm-security; routine hadn't run since 2026-06-16). Coverage `scan_kind=upkeep`; pm_cursor idx 5 → 6 (pm-security ran, pm-data next); coverage_cursor idx 12 → 0 (wrap to epic-6)._

## Executive summary

- **41 PRs landed on `dev` (#1946–#2008) plus #2011.** Dominant themes: fault-notification triage/confirm (#1974), messaging N-party read watermark (#1977), messaging unread soft-delete + set-based block (#1993), regional-compliance quorum (#1976), auth capability-token hardening (#1946), TOTP fix (#1946), reality-web SSO CSRF close (#1987), rentals PII manager-gate (#1973), migration template pagination (#1994), and a large BIT-4xx test-backfill wave (forms/faults/finance/governance).
- **`dev` is GREEN** — the #1437 backend compile break from 2026-06-16 is history; downstream CI signals are healthy again.
- **Sprint reconciliation:** Epic 8A **done** (3/3, WS + toggles + critical override). Epic 10B **done** (7/7). Epic 80: 80-3 mediation promoted to done, 80-2 still partial (5-step wizard + draft auto-save). Epic 6: 5/6 done (6-1..6-5 verified + 6-6 done earlier). 84-5 pgvector RAG promoted done (#1833). 81-1 report schedule promoted (#1917).
- **Follow-up backlog (from merged-PR review):** #2028 (u.name schema drift → 500 in `list_bookings`), #2029 (fault-triage tests don't exercise real handler), #2030 (Slovak accounting export renders misleading 0). Older-but-hot: #2001 (finance 100% overstated), #2000 (CountryCode `S_K` serialization — **HIGH severity**), #1999 (favorite-alert enqueue+advance race dropping alerts), #1998 (#1771 test never runs in CI), #1997 (migration pagination follow-ups). #2009 (cargo-deny ammonia) resolved by #2011 today.
- **Top churn:** `backend/servers/api-server/src/routes/messaging.rs` (3 touches) and `backend/crates/db/src/repositories/messaging.rs` (3 touches). N-party messaging is the hottest surface; two follow-ups still open on it (read-watermark #1977, unread-leak #1993).

## Security posture (pm-security lens)

Six merged PRs in this window materially tightened the attack surface:

| PR | Surface | Effect |
|---|---|---|
| #1946 | Auth capability-token + TOTP | Capability-token replay hardened; TOTP verification bug closed |
| #1973 | Rentals PII | PII fields now behind manager-only gate |
| #1976 | Regional-compliance | Quorum enforced; single-manager overrides removed |
| #1987 | reality-web SSO | CSRF token close on callback |
| #1989 | Accounting | 404 symmetry + `ToSchema` dropped from secret model (prevents OpenAPI leak) |
| #1993 | Messaging unread | Soft-delete + set-based block prevents unread-count leak across blocked users |

**Residual risks worth calling out this run:**

1. **HIGH — CountryCode `S_K` serialization (#2000).** Wire contract renders `SK` (Slovakia) as `S_K` under current serde config. Survived the multi-currency test un-quarantine, meaning tests either don't cover this axis or use a different Serialize path. Any external consumer (SDK, partner integration, mobile client) that parses `country_code` will silently break for Slovak orgs. Fix: switch the enum to `#[serde(rename_all = "UPPERCASE")]` or add explicit `#[serde(rename = "SK")]`, then add a golden-file regression at the OpenAPI boundary.
2. **HIGH — Favorite-alert enqueue+advance race (#1999).** The alert dispatcher advances the cursor before the enqueue commit is durable — under load, alerts are silently dropped. This is a data-loss bug on a user-visible surface. Fix: wrap the advance in the same transaction as the enqueue (or use SELECT … FOR UPDATE SKIP LOCKED with commit-then-advance).
3. **MEDIUM — #1771 test never runs in CI (#1998).** Test file exists but is excluded from the runner (path or feature-gate misconfig). Silent regression of whatever #1771 fixed. Fix: audit the CI test filter; add a CI-side "expected test count" guard.
4. **Latent — Multiple hardening PRs shipped without an over-arching negative-test suite** for capability-token replay, CSRF and PII gating. Add a lightweight tenant-boundary + auth-negative test file per surface so a future refactor can't silently reopen these holes.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A + Epic ACC + BIT-4xx test wave"** · epics_done=5 tracked (8A, 10B, plus 80-3/84-5/81-1 promotions).

| Epic | Tracked status | Real status |
|---|---|---|
| 6 — Announcements & Comms | in-progress | 5/6 stories done (6-1..6-5 + 6-6); web UI complete, mobile comments/pin UI still gap |
| 7A — Basic Document Mgmt | in-progress | 1/5 done (7a-1); 7a-2 in review (FK/isolation fix), 7a-3/4/5 ready-for-dev |
| 8A — Basic Notif Prefs | **done** | 3/3 |
| 10A — OAuth Provider | in-progress | 0/3; blocked on #481 (revoked-token bypass) + #487 (MFA rate-limit test) |
| 10B — Platform Admin | **done** | 7/7 |
| 80 — Dispute Resolution | partial | 2/3 (80-1, 80-3 done; 80-2 partial — 5-step wizard + draft auto-save) |
| 82 — Mobile (Reality KMP) | in-progress | Coverage churn ongoing; 4 gap-82 items open |
| 84 — AI/RAG | in-progress | 84-5 pgvector done (#1833) |
| 85 — Mobile Build | in-progress | 85-1/85-2 promoted done; EAS pipeline green-status still owed |
| ACC — Accounting | in-progress | Slovak export display bug (#2030), finance 100% overstated (#2001) |

## Shipped since last run (window #1946–#2008 + #2011, top 12)

- **#1946** — Auth capability-token replay + TOTP verification hardening [pm-security]
- **#1973** — Rentals PII manager-gate [pm-security]
- **#1974** — Fault-notification triage/confirm flow [pm-backend]
- **#1976** — Regional-compliance quorum enforcement [pm-security]
- **#1977** — Messaging N-party read watermark [pm-backend]
- **#1987** — reality-web SSO CSRF close [pm-security]
- **#1989** — Accounting 404 symmetry + `ToSchema` off secret model [pm-security]
- **#1993** — Messaging unread soft-delete + set-based block [pm-backend]
- **#1994** — Migration template pagination [pm-backend]
- **BIT-4xx wave** — forms/faults/finance/governance test backfills [pm-qa]
- **#2011** — Ammonia RUSTSEC bump (closes #2009) [pm-devops]

## What's next (top 5 actions)

1. **[high] Rename CountryCode enum variants so `SK` serializes as `SK`, not `S_K` (#2000).** Owner: pm-security + pm-backend. Add OpenAPI golden-file regression.
2. **[high] Wrap favorite-alert enqueue+advance in a single tx (#1999).** Owner: pm-backend. Add a synthetic load test that asserts no dropped alerts under enqueue contention.
3. **[high] Un-gate the #1771 test (#1998).** Owner: pm-devops. Confirm the test now runs in CI; add an "expected test count ≥ N" tripwire.
4. **[medium] Triage new follow-up issues #2028/#2029/#2030 (u.name schema drift, fault-triage stubby tests, Slovak accounting 0).** Owner: pm-scrum-master.
5. **[medium] Refresh coverage.json (`scan_kind=upkeep`, epic-6 next cursor) to reflect 6-2/6-3/6-4/6-5/80-3/81-1/84-5/85-1/85-2 promotions.** Owner: pm-scrum-master.

## Blockers

- **#2000 CountryCode wire contract (HIGH).** External-facing serialization bug; blocks any consumer that parses country_code for SK orgs.
- **#1999 Favorite-alert drop-race (HIGH).** Data-loss on user-visible surface.
- **#1998 #1771 test excluded from CI.** Silent-regression risk.
- **Follow-ups from #481/#487 still block 10A promotion.** OAuth revoked-token bypass + MFA rate-limit test gap.

## Role focus today: **pm-security** (+ pm-scrum-master always-on)

- **pm-security** (rotation idx 5, ran today): produced the Security-posture section above; three residual risks appended to `risks.json`; two new items appended to `action-list.json`.
- **pm-scrum-master** (always-on): sprint reconciliation above; note that this is a catch-up synthesis — next scheduled run will be pm-data.

## Coverage (upkeep — 2026-07-02)

- Last deep scan 2026-06-23 (`scan_kind=deep`, 13 epics · 49 stories · 37 done · 12 partial). Since that scan, at least 8 more stories flipped done via merged PRs (6-2/6-3/6-4/6-5, 80-3, 84-5, 81-1, 85-1/85-2, 8a-1/2/3, 10b-1/2, 79-1). Actual `done` count likely ~45/49. Refresh needed.
- Highest-leverage remaining coverage gap: **7A document-management story cluster** (only 7a-1 done; 7a-2 in review with CI-red history, 7a-3/4/5 ready-for-dev). All backend routes exist; the block is verification + minor UX (share panel UUID validation, #485).
- Screen-map `epics:` frontmatter backfill still an unresolved systemic gap (from 2026-06-16 note).
