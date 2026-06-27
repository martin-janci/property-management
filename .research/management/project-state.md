# PPT Project State

_Generated: 2026-06-27 — daily PM rotation (Scrum Master + pm-security; routine refresh, **catch-up mode after 11-day lag**). Coverage `scan_kind=upkeep`; pm_cursor idx 5 → 6 (pm-security → pm-data next), coverage_cursor idx 12 → 0 (epic-9 re-checked, wrap to epic-10a)._

## Executive summary

- **Catch-up run: 191 PRs merged in the 11-day window** (2026-06-16 → 2026-06-27, PRs #1548 → #1864). The routine missed 5+ scheduled runs — this brief surfaces the highlights but does not enumerate every signal. The dispatcher continued through the window (4 active assignments, 1 reviewed today) and most signal-worthy issues were already filed as follow-up GitHub issues by the post-merge-review pass.
- **Massive churn-hotspot wave landed.** Eight `churn-hotspot-*` and one `repeated-churn-*` PRs split previously-monolithic route/repo files into cohesive submodules: documents (#1683), forms (#1700), aml_dsa (#1708), icon-test (#1718), form-rls-tests (#1719), subscription (#1796), emergency (#1798), enhanced-tenant-screening (#1800), sensor (#1810), reserve-funds (#1816). The vendors (#1813) and iot (#1815) modules also split via refactor PRs. **All 12 hotspot-driven splits previously surfaced by this routine are now done.** Two still-open drafts: form (#1814), reality_portal (#1812).
- **Feature delivery, this window:** Epic 3 (delegations wiring + revert, person-month tracking, unit "My Unit" view, building geocoding, document templates, e-signature surface), Epic 4 (fault analytics + lifecycle notifications + mobile offline-queue), Epic 5 (N-party group conversations, message attachments S3, per-participant archive), Epic 6 (announcement viewing/ack/comments/pin), Epic 11 (financial statements + Stripe Checkout), Epic 16 (saved-search alert cadence + transport drainer), and Epic 18 (guest ID-document OCR seam). Of those, **22 stories were reconciled to "done"** via the dispatcher's reconcile PRs (#1829-#1845).
- **Security work, this window:** PR #1741 (Airbnb manager-gate for guest PII), PR #1744 (auth unification), PR #1746 (portal-listings IDOR), open draft #1857 (security-llm-doc-idor tests). Several **NEW** open security risks filed by post-merge review: #1772 (OCR endpoints unauthenticated), #1782 (a third access-token verification copy unmigrated), #1786 (sensor WS authz regression tests), #1791 (message-attachments IDOR via client-supplied file_key), #1797 (OCR/rental PII manager gate), #1799 (message attachment thread-binding + MIME validation), #1806 (booking_channel manager gate JWT vs DB), #1823 (guest ID-document PII hardening).
- **Reverted (process, not code):** PR #1713 reverted PR #1690 because #1690 re-wired the retired delegation frontend ~16min before CEO ruling that delegations stay retired. Process gap, not a code bug.
- **CI gate health, this window:** Multiple `fix(db)` PRs for duplicate migration version collisions (#1724 → #1755 → #1757 for v00187 + v00192) — the migration-version uniqueness check is fragile; consider adding pre-commit verification. PR #1735 fixed backend-skip mixed-PR decision determinism.
- **Stale draft-PR queue (open >3 days):** #1754 (admin-web stale 404/501 fallbacks), #1795/#1797/#1799/#1801/#1804/#1806/#1807/#1812/#1814/#1819/#1823/#1824/#1825/#1833/#1846 — most are dispatcher-managed follow-ups awaiting human-gated CI; not stalled-review.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"**. This sprint has been overtaken by parallel epic delivery — Epic 6 stories 6-1/6-2/6-3/6-4/6-5 reconciled to "done" this window; Epic 8A already done; OAuth (10A) still in-progress per sprint-status.yaml; 7A stories in flight via document e-signature work.

## Shipped since last run (cursor #1439, 191 PRs)

Too many to enumerate; see `briefs/2026-06-27.md` Shipped section for the top deltas. The dispatcher merged 12 churn-split refactors, the post-merge review filed ~30 follow-up issues (#1772-#1854), and feature PRs for Epics 3-18 landed in parallel.

## What's next (top 5 actions)

1. **[high] Investigate the open security IDOR/PII follow-ups (#1772, #1791, #1797, #1799, #1806, #1823)** — pm-security + pm-backend. These are the post-merge reviewer's highest-confidence findings from this window; left open they age and decay.
2. **[high] Confirm migration-version collision pattern is fully resolved** — pm-devops. PRs #1724/#1755/#1757 all patched duplicate-version races within the same window; consider adding a CI gate that fails on duplicate `NNNNN_` prefix before merge.
3. **[medium] Reconcile the 51 open action-list items against the 191 merged PRs** — pm-scrum-master. Many of the 2026-06-12/15 actions are likely shipped but not marked done. Defer to dispatcher's reconcile pass next cycle.
4. **[medium] Close out the two remaining churn-hotspot drafts** — pm-backend. #1814 (form repo split) and #1812 (reality_portal split) are the only structural-refactor PRs still open.
5. **[medium] Re-check Epic 16 saved-search alert work end-to-end** — pm-data. Stories 16.3 (alert_frequency cadence #1847) + 16.4 (transport drainer #1849) shipped this window; the worker is the system's first persistent background-job pattern — needs ops verification before more workers cargo-cult it.

## pm-security focus (rotating role this run)

The 2026-05-27 pm-security report tracked three open risks:
- **`pm-security-update-schedule-cross-tenant-idor`** (#614/#624) — still open in `risks.json`; reports.rs / report_schedule.rs unchanged in this window. **Status: still open.**
- **`pm-security-audit-hash-debug-format-p1-04`** (PR #435 residual) — no observable activity. **Status: still open.**
- **`pm-security-oauth-10a-untested-security-contract`** — OAuth 10a stories still in-progress per sprint-status. **Status: still open.**

NEW security risks introduced this window (from post-merge follow-ups, awaiting triage as risks-list rows):
- `#1772` OCR endpoints unauthenticated + meter-reminder doc drift
- `#1782` third access-token verification path (JwtService) left unmigrated by PR #1744 auth unification
- `#1786` sensor WS handler — authz regression tests for the surviving DB-checked handler
- `#1791` message attachments — link trusts client-supplied file_key (IDOR)
- `#1797` OCR endpoints auth + manager-gate rental guest PII reads
- `#1799` message attachment file_key bind to thread + MIME validation (IDOR)
- `#1806` booking_channel — DB-backed manager gate vs JWT role claim
- `#1823` guest ID-document upload PII hardening — audit logs, content sniff, manager gate

Recommendation: pm-security and pm-backend pair on the message-attachment + OCR + guest-PII triple in one sweep — they share the same defense-in-depth pattern (thread-bind + MIME-sniff + audit log + manager gate).
