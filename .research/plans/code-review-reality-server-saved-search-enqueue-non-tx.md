# code-review-reality-server-saved-search-enqueue-non-tx

**Vector:** bug
**Score:** 3
**Source:** Phase 1.5 reality-server code review (2026-06-27); churn cluster PRs #1847/#1849/#1850
**Confidence:** high

## Hypothesis
`SavedSearchAlertsService::run_once` issues `enqueue_search_alert` (`INSERT INTO search_alert_queue`) and `mark_saved_search_matched` (`UPDATE saved_searches SET last_matched_at`) as two separate statements against the same `PgConnection`, **not** inside a transaction. The watermark error is even discarded with `let _ = …`. If the process is killed between the two calls — or the `UPDATE` fails for any reason — the next poll re-finds the same listings (because `last_matched_at` was never advanced) and re-enqueues them. `search_alert_queue` has no `UNIQUE(saved_search_id, listing_id)`, so the duplicate row drains as a second in-app + email + push alert. The fix wraps both writes in `let mut tx = conn.begin().await?; … tx.commit().await?;` and propagates the watermark error.

## Evidence
- `backend/servers/reality-server/src/services/saved_search_alerts.rs:216-240` — two-call pattern: `enqueue_search_alert(conn, …).await?;` then `let _ = mark_saved_search_matched(conn, …).await;` — no `begin()`, no `commit()`, watermark error swallowed
- `backend/crates/db/src/repositories/reality_portal.rs` — `enqueue_search_alert` and `mark_saved_search_matched` are independent helpers, each takes `&mut PgConnection`, neither asserts a transaction is open
- `backend/crates/db/src/repositories/reality_portal.rs` — `search_alert_queue` schema has no `UNIQUE (saved_search_id, matching_listing_ids)` or per-pair dedup index; the table relies entirely on the watermark advancing to skip already-emitted rows
- Module doc at top of `saved_search_alerts.rs` claims "exactly-once per (saved_search_id, listing_set) pair" — the implementation does not back that claim

## Files
- `backend/servers/reality-server/src/services/saved_search_alerts.rs`
- `backend/crates/db/src/repositories/reality_portal.rs`

## Dependencies
<!-- The drainer-side row-reservation fix (sibling plan) is independent;
either can land first. They compose: enqueue dedup + drain reservation
together give the documented exactly-once guarantee. -->

## Required capabilities
- [x] C1 — Systematic debugging (transactional correctness bug)
- [x] C2 — Seed data (need saved_search + matching listings to repro)
- [x] C3 — Dev instance running (pg for the integration test; reality-server worker for the kill-mid-flight repro)
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. `stack up pm-local`; seed one tenant with one `saved_search` plus one new listing that matches it (so `find_listings_for_saved_search` returns exactly that listing).
2. Inject a panic in `mark_saved_search_matched` (temporary `panic!()` at top) or trap SIGKILL mid-statement.
3. Trigger `SavedSearchAlertsService::run_once`. Observe one row written to `search_alert_queue`; process dies before `last_matched_at` advances.
4. Restart the worker, run again. Expected: **zero new rows** (the prior enqueue already covered this listing). Actual: a *second* identical row in `search_alert_queue` — the user gets two alerts after the drainer runs.

## Suggested approach
1. In `saved_search_alerts.rs::run_once`, change the per-saved-search block to:
   ```rust
   let mut tx = pool.begin().await?;
   enqueue_search_alert(&mut *tx, …).await?;
   mark_saved_search_matched(&mut *tx, saved_search.id, last_match_at).await?;
   tx.commit().await?;
   ```
   Remove the `let _ = …` swallow on the watermark write — propagate the error so the run logs it and the next poll retries cleanly.
2. Add a defensive `UNIQUE (saved_search_id, matching_listing_ids_hash)` index on `search_alert_queue` via a new SQLx migration. Compute `matching_listing_ids_hash` as `md5(array_to_string(matching_listing_ids, ','))` (or `sha256` if md5 isn't available) — a generated column or trigger keeps it consistent with the `matching_listing_ids` array. With the index, even if a future code path re-enqueues, the second insert hits a constraint violation (caught + logged, not propagated to the user as a duplicate alert).
3. Update the module doc at the top of `saved_search_alerts.rs` to match the now-true invariant: "enqueue + watermark are atomic; dedup is enforced by `(saved_search_id, listing_ids_hash)` index".
4. Run `cargo sqlx prepare --workspace` to refresh offline data for the new migration + queries.
5. Add an integration test (`backend/servers/reality-server/tests/saved_search_alerts_transactional_test.rs`) that simulates the kill-between-writes path: spawn `run_once` in a task, abort the task while holding a savepoint, then run a fresh `run_once` and assert `search_alert_queue` has exactly one row.

## Alternatives considered
- **Make the dedup window time-based (`AND last_matched_at IS NULL OR last_matched_at < listing.created_at - INTERVAL '1 hour'`)** — rejected because it papers over the real bug (split write becoming non-atomic) and would still emit a duplicate if two listings match within the hour bucket; a transaction is the smaller, correct change.
- **Move the watermark advance to be the FIRST write (so a crash leaves us with a `last_matched_at` ahead of an unsent enqueue, i.e. lose-an-alert instead of duplicate-an-alert)** — rejected because losing user alerts is a worse failure mode than duplicating them; the team has explicitly chosen at-least-once + idempotent dedup over at-most-once.

## Root-cause trace
1. Symptom: a single matching listing produces two saved-search alerts after a worker crash/restart cycle.
2. ← `saved_search_alerts.rs:216-240` — enqueue + watermark are issued as two unrelated statements; a crash between them leaves the queue row but not the watermark advance.
3. ← `reality_portal.rs::mark_saved_search_matched` — independent helper takes `&mut PgConnection` with no transaction assertion, so calling code is free to mis-use it as we do here.
4. ← `reality_portal.rs` — `search_alert_queue` schema has no per-(saved_search, listing_ids) dedup index, so the second enqueue lands as a real duplicate row.
5. Origin: PR #1847 (story 16.3 alert_frequency cadence, merged 2026-06-25) — introduced the two-call sequence; PR #1849 (drainer wiring) made the duplicate visible to users.

## Test plan
- [ ] `backend/servers/reality-server/tests/saved_search_alerts_transactional_test.rs` — run_once kill-between-writes test; asserts `SELECT COUNT(*) FROM search_alert_queue WHERE saved_search_id = $1` is exactly 1 after the restart. **Fails on `main` today** (would observe count = 2).
- [ ] Existing `saved_search_alerts` unit tests stay green — the happy path (single run, no crash) is unchanged.
- [ ] Migration `up` + `down` apply cleanly against the local DB.
- [ ] Command: `cd backend && cargo test -p reality-server --test saved_search_alerts_transactional_test && cargo test -p db --test saved_search_alert_queue_uniqueness_test`

## Out of scope
- Fixing the *drain* side row-reservation gap (covered by sibling plan `code-review-reality-server-drainer-no-row-reservation`).
- Adding retry/backoff to the drainer outbound calls (covered by backlog `code-review-reality-server-drainer-no-backoff-rate-limit`, score 2).
- Auditing or porting the same fix to the favorite-alerts (`favorite_alerts.rs`) drainer — it likely has the same shape and deserves its own follow-up plan.

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-saved-search-enqueue-non-tx.md`
- Mark `backlog.json` row `code-review-reality-server-saved-search-enqueue-non-tx` as `status: "done"`
