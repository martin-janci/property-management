# PPT Project State

_Generated: 2026-07-02 — routine upkeep pass after 16d lag. Coverage `scan_kind=upkeep`; pm_cursor idx 5 → 6 (pm-security completed → pm-data next), coverage_cursor idx 12 → 0 (epic-9 re-checked → epic-10a next). This is a lightweight refresh: no deep role fan-out (routine cost); consult the local `/ppt-project-management scan` for the authoritative rebuild._

## Delta since last run (2026-06-16)

- **380 PRs merged** into `dev` over 16 days — heavy velocity across security IDOR closes (`fix(BIT-25)`, `fix(rentals)`, `fix(messaging)`), test-coverage backfill waves (BIT-267/268 batches 1–5), and mobile-native shipping (KMP inquiries + iOS SwiftUI parity).
- **96 new issues, 6 open PRs** (2 draft — #1812 reality_portal split awaiting human review; #1797 rental-guest PII gate; #2006 KMP inquiries verify-to-done). One PR (#1797) stalled 7d.
- **Routine ran with a 16d lag** — the cloud research routine cron appears to have been paused between 2026-06-16 and today. Recommend inspecting the cron schedule (see .research/routine-prompt.md § Stale-routine alert P4). The dispatcher (separate loop) continued running; see `.research/management/assignments.json` on `planning`.
- **Backlog:** decay applied to 14d-old opens; 3 new items this run (1 code-review + 1 risky-churn + 1 screen-map-drift). See `backlog.md` for the ranked view.

## PM-security snapshot (today's rotation focus)

`pm-security` last ran 2026-05-27 (36 days stale). Given the delta above touches auth (BIT-25 IDOR), rental guest PII (#1797 gate open), OCR endpoints (#1797 auth added), and Stripe hardening (#1824 closed unmerged), the security-relevant open work this run is:

- **Open — #1797 draft, 7d idle**: auth on OCR endpoints + manager-gate rental guest PII reads. Blocks #1772 and #1766.
- **Open — code-review-api-core-scheduler-metrics-lock-poison** (new this run): 13× `metrics.lock().unwrap()` in `services/scheduler.rs`; a task-panic cascade could kill the whole scheduler service.
- **Verify** — several IDOR closes shipped (`BIT-25 #1419`, `#1790` reality/dispute + org isolation); confirm regression coverage via the merged happy-path batches.

Deferring full 8-role deep analysis to next non-lag run or a manual `/ppt-project-management full` invocation.

## Sprint

Last recorded sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** (from 2026-06-16 project-state). Given 380 PRs merged since, sprint status is stale; a full re-scan is recommended before the next PRD/story cut.

## Delivery cadence

- Auto-fix count this run: 0 (allowlist not met — kill-switch not set; no allowlist signal cleared the certainty bar).
- Plans promoted this run: see brief.
