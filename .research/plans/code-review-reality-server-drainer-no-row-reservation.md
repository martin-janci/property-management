# code-review-reality-server-drainer-no-row-reservation

**Vector:** bug
**Score:** 3
**Source:** Phase 1.5 reality-server code review (2026-06-27); churn cluster PRs #1847/#1849/#1850
**Confidence:** high

## Hypothesis
The Epic 16 saved-search alert drainer reads pending rows with a plain `SELECT … WHERE notified_at IS NULL` and only writes `notify_started_at` (no such column) / `notified_at` *after* the email + push transports finish. Two concurrent drainer instances — HA pair, rolling deploy with overlap, or one slow drainer whose `run_once()` latency exceeds `poll_interval_secs=60` — both load the same row set and both dispatch, so the user receives the saved-search alert twice (once per channel × N drainers). The fix is to reserve rows atomically with `FOR UPDATE SKIP LOCKED` (or an advisory lock around the per-row send) so only one drainer claims each row at a time.

## Evidence
- `backend/servers/reality-server/src/services/search_alert_drainer.rs:244-326` — `list_undelivered_search_alerts` performs `SELECT … FROM search_alert_queue WHERE notified_at IS NULL ORDER BY created_at ASC LIMIT $1` with no `FOR UPDATE`, no `SKIP LOCKED`, no reservation column write
- `backend/servers/reality-server/src/services/search_alert_drainer.rs:122-174` — per-row dispatch loop runs the email/push transports first, then calls `mark_search_alert_notified` — between SELECT and that UPDATE the row is visible to every other concurrent reader
- `backend/servers/reality-server/src/services/search_alert_drainer.rs` module doc — claims at-least-once-via-status-column safety, but the status column is owned by the in-app channel, not the transport channels
- Default config `poll_interval_secs=60` with batch_size=100 → trivial to hit overlap on any backlog where dispatch latency exceeds the poll interval (network blip, FCM slow path)

## Files
- `backend/servers/reality-server/src/services/search_alert_drainer.rs`
- `backend/crates/db/src/repositories/reality_portal.rs`
- `backend/servers/reality-server/src/services/saved_search_alerts.rs`

## Dependencies
<!-- No prior plans; this is the first remediation pass on the drainer. -->

## Required capabilities
- [x] C1 — Systematic debugging (drainer concurrency bug)
- [x] C2 — Seed data (need saved_search + matching listings to populate search_alert_queue)
- [x] C3 — Dev instance running (pg + reality-server for the integration test)
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Bring up `stack up pm-local` (or the bridge MCP equivalent); seed one tenant with one saved search and one matching listing so `search_alert_queue` has exactly one row with `notified_at IS NULL`.
2. Start two reality-server drainer instances pointing at the same DB (e.g. `cargo run -p reality-server --features search-alert-drainer` in two terminals, or run two `tokio::spawn(run_drainer(...))` tasks inside an integration test).
3. Wait one `poll_interval_secs` cycle. Expected: exactly one outbound email + one outbound push for the user. Actual: two emails and two pushes (one per drainer instance) — `mark_search_alert_notified` only updates the row after both already started.

## Suggested approach
1. Add a `notify_started_at TIMESTAMPTZ NULL` column to `search_alert_queue` via a new SQLx migration (sibling pattern: the existing `notified_at` column). Index it for the new WHERE clause.
2. Replace the `SELECT … LIMIT $1` in `list_undelivered_search_alerts` (`backend/crates/db/src/repositories/reality_portal.rs`) with the reservation pattern:
   ```sql
   UPDATE search_alert_queue
   SET notify_started_at = NOW()
   WHERE id IN (
     SELECT id FROM search_alert_queue
     WHERE notified_at IS NULL
       AND (notify_started_at IS NULL OR notify_started_at < NOW() - INTERVAL '5 minutes')
     ORDER BY created_at ASC
     LIMIT $1
     FOR UPDATE SKIP LOCKED
   )
   RETURNING *;
   ```
   The 5-minute stale-claim window handles a drainer crashing mid-send: another drainer reclaims it after the timeout, the dedup contract still holds against `notified_at`.
3. Run the `sqlx-cli` offline data prep (`cargo sqlx prepare --workspace`) so CI's offline mode picks the new query up.
4. Update `mark_search_alert_notified` to clear `notify_started_at` only when also setting `notified_at` (sentinel pairing — no orphan reservations).
5. Update the drainer module doc (top of `search_alert_drainer.rs`) to describe the new at-most-once-per-poll-window guarantee.
6. Add an integration test (`backend/servers/reality-server/tests/search_alert_drainer_concurrency_test.rs`) that spawns two drainer tasks against a shared `PgPool` + one queued row and asserts exactly one send (mock SMTP + push counted via test-double).

## Alternatives considered
- **PostgreSQL advisory lock per row id (`pg_advisory_xact_lock(hashtext(id::text))`)** — rejected because it adds a per-row round-trip on every drain and the existing schema already has the `notified_at` column we can pair with a sibling `notify_started_at`; `FOR UPDATE SKIP LOCKED` is the idiomatic PG pattern and matches sibling queues in this codebase.
- **Singleton drainer via leader election (Redis NX + TTL)** — rejected because it converts a row-level concurrency bug into an HA gap (single point of failure on the leader-elect path) and the team has explicitly chosen horizontal scale-out for the reality-server worker tier.

## Root-cause trace
1. Symptom: user receives saved-search alert email + push twice when two drainer instances are running.
2. ← `search_alert_drainer.rs:122-174` dispatch loop sends to SMTP/FCM before any DB write that would hide the row from concurrent readers.
3. ← `reality_portal.rs::list_undelivered_search_alerts` issues plain `SELECT … LIMIT $1` with no `FOR UPDATE`; the row remains visible to every other drainer's next poll.
4. Origin: PR #1849 (epic-16 email/push drainer for saved-search alerts, merged 2026-06-26) — drainer wiring landed without the reservation column the at-least-once contract requires.

## Test plan
- [ ] `backend/servers/reality-server/tests/search_alert_drainer_concurrency_test.rs` — two-task drain against one queued row, mock transports asserted at `send_count == 1`. **Fails on `main` today** (would observe `send_count == 2`).
- [ ] Existing `search_alert_drainer` unit tests stay green — no functional change for the single-drainer single-row case.
- [ ] Manual: kill drainer mid-send (after `notify_started_at` set, before `notified_at`), wait 5 min, confirm second drainer picks the row up exactly once.
- [ ] Command: `cd backend && cargo test -p reality-server --test search_alert_drainer_concurrency_test`

## Out of scope
- Fixing the *enqueue* side non-transactional behaviour (covered by sibling plan `code-review-reality-server-saved-search-enqueue-non-tx`).
- Adding rate limits / backoff on retry storms (covered by sibling backlog item `code-review-reality-server-drainer-no-backoff-rate-limit`, score 2 — not yet promoted).
- Migrating the favorite-alerts drainer (`favorite_alerts.rs`) — same fix shape applies but is a separate atomic patch.

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-drainer-no-row-reservation.md`
- Mark `backlog.json` row `code-review-reality-server-drainer-no-row-reservation` as `status: "done"`
