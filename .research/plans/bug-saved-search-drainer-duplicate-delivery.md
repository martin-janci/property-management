# bug-saved-search-drainer-duplicate-delivery

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review 2026-06-27 (Phase 1.5, reality-server segment); commit b7b6ad9 (drainer shipped in PR #1849)
**Confidence:** high

## Hypothesis
The saved-search alert drainer can re-deliver the same alert on every 60-second poll when the post-send `mark_search_alert_notified` UPDATE fails. The send + mark are two independent DB round-trips, the row is selected on `notified_at IS NULL` each tick, and the bounded-retry path only fires when *no* channel succeeded. So on a transient DB hiccup after a successful email/push fan-out, the user receives the same alert repeatedly until something else updates the row. The smallest fix is to wrap the send + mark as a transactional pair, or pre-bump `notify_attempts` before the send so a duplicate run can short-circuit on max-attempts.

## Evidence
- `backend/servers/reality-server/src/services/search_alert_drainer.rs:317-325` — `if any_ok { mark_search_alert_notified(...).await } …` logs the mark-failure path and continues without `record_search_alert_notify_failure`; the bounded retry is the `else if !any_ok` branch only.
- `backend/servers/reality-server/src/services/search_alert_drainer.rs:88` (LogEmailTransport) — emits `to_email` at INFO; if duplicates fire to wrong recipients, PII compounds.
- PR #1847 (`feat(reality-server): story 16.3 — saved-search alert_frequency cadence`, BIT-140) and PR #1849 (`feat(epic-16): email/push transport drainer for saved-search alerts`, BIT-139) shipped the drainer + cadence in the 2026-06-16 → 2026-06-27 window.
- Phase 1.5 rotating expert review (reality-server segment, churn-aligned: 3 changes/11d) flagged this as HIGH severity / HIGH confidence.

## Files
- `backend/servers/reality-server/src/services/search_alert_drainer.rs`
- `backend/servers/reality-server/src/services/saved_search_alerts.rs`
- `backend/crates/db/src/repositories/reality_portal.rs`

## Dependencies
<!-- No prior task_ids block this fix. -->

## Required capabilities
- [x] C1 — Systematic debugging (bug vector; trace mark/send ordering)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Mode: cloud-ok**

## Repro steps
1. Seed one saved search with `alert_frequency = realtime` (or `1m`) and a fresh match in the matches table so the drainer picks it.
2. In `mark_search_alert_notified` (or its repository sibling), inject a one-shot DB failure on the UPDATE (e.g. close the pool, drop the connection, or force a unique-violation on a side index) such that the SELECT succeeds but the UPDATE returns an error.
3. Let the drainer tick run twice (60s × 2 with the default cadence; lower the poll interval in test config to compress the wait).
4. **Expected:** at most one email/push send for that match; the second tick sees `notify_attempts` bumped or `notified_at` set and skips.
5. **Actual:** two (or N) sends — every successful poll re-delivers the same alert because the row stays `notified_at IS NULL` with `notify_attempts` unchanged.

## Suggested approach
1. Pre-bump `notify_attempts` (and stamp a `last_attempt_at`) **before** the transport fan-out begins, so a duplicate tick can short-circuit on `attempts >= MAX_ATTEMPTS` even when the post-send mark never lands. Combine with the existing `MAX_ATTEMPTS` constant.
2. After a successful send, run `mark_search_alert_notified` in a small retry loop (3 tries × 100 ms backoff) before falling through; on persistent failure, call `record_search_alert_notify_failure` so the bounded-retry path engages and the row eventually exits the queue. This keeps the change additive — same column shape, no migration.
3. Alternatively (or in addition): widen the SELECT to `notified_at IS NULL AND notify_attempts < MAX_ATTEMPTS` so a row that pre-bumped its attempts but failed to mark is not re-picked on the very next tick. Symmetrical with step 1.
4. While in the file, fix the `let _ = mark_saved_search_matched(...)` swallow at `saved_search_alerts.rs:176-180` and `:237-240` — at minimum a `tracing::warn!` with the saved-search id + error; consider propagating so the watermark cadence is not silently broken (the comment at lines 234-236 explicitly warns this).
5. Hash or redact `to_email` in the LogEmailTransport tracing field at `search_alert_drainer.rs:88` so the PII does not surface in INFO-level logs.
6. Add a regression test in `backend/servers/reality-server/src/services/` (new `search_alert_drainer_tests.rs` or extend existing) that simulates a mark-failure-after-success path and asserts exactly one send.

## Alternatives considered
- **Wrap send-and-mark in a single transaction** — rejected because the SMTP / push transports are external side effects that cannot be rolled back; a DB rollback on a successful send would mask, not fix, the duplicate-delivery problem.
- **Skip the change and rely on transport-level idempotency keys** — rejected because the current `LogEmailTransport` (and the planned SMTP adapter) emit no per-row idempotency key; adding one would mean a coordinated client+transport change far larger than the in-drainer fix.

## Root-cause trace
1. Symptom: a saved-search match generates duplicate `Alert sent` emails / pushes to the same user every 60s for at least one cadence window.
2. ← `search_alert_drainer.rs:317-325` calls `mark_search_alert_notified` AFTER the transport fan-out; when it errors, `notified_at` stays NULL and `notify_attempts` is not bumped, but the warning log proceeds and the loop continues.
3. ← `search_alert_drainer.rs` poll loop re-selects rows on `notified_at IS NULL` each tick; the bounded-retry guard at the `else if !any_ok` branch never fires because at least one channel succeeded.
4. Origin: PR #1849 (`feat(epic-16): email/push transport drainer for saved-search alerts`, merged 2026-06-25 area) — initial drainer landed without the attempt-pre-bump invariant; the cadence cleanup later in `saved_search_alerts.rs` documents the watermark contract but does not enforce it on the alert row itself.

## Test plan
- [ ] Add `backend/servers/reality-server/src/services/search_alert_drainer.rs` (or sibling test file) unit test `mark_failure_after_successful_send_does_not_redeliver`. Use a test transport that always succeeds plus a mocked `mark_search_alert_notified` that fails on the first call and succeeds on the second.
- [ ] Add a `watermark_update_failure_logged` test in `saved_search_alerts.rs` test module that injects a `mark_saved_search_matched` failure on the first-sighting branch and asserts a `tracing::warn!` was emitted (use `tracing_test`).
- [ ] Run `cargo test -p reality-server search_alert_drainer` and `cargo test -p reality-server saved_search_alerts` locally. Must fail on `main`, pass after the patch.

## Out of scope
- Replacing `LogEmailTransport` with a real SMTP adapter (BIT-139 follow-up).
- Reworking the saved-search alert *scheduler* (Story 16.3 cadence model) — only the post-send mark path is in scope here.
- Adding per-alert idempotency keys to the transport layer (separate plan).

## After-merge
- Move this file to `plans/_archive/bug-saved-search-drainer-duplicate-delivery.md`
- Mark `code-review-reality-server-drainer-dup-send` in `backlog.json` as `status: "done"`
