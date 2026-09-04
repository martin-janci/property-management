# code-review-api-core-data-export-download-token-reuse

**Vector:** security
**Score:** 3
**Source:** Phase 1.5 rotating expert review 2026-09-04 (api-core segment)
**Confidence:** high

## Hypothesis
`download_export` in `backend/servers/api-server/src/routes/gdpr.rs` drops the `Result` of `mark_downloaded` with `let _ = ...await;` (lines 198-201) and *then* streams the export body. `data_export.repositories::get_by_token` (`backend/crates/db/src/repositories/data_export.rs:271-281`) filters WHERE `status = 'ready'`, so if `mark_downloaded` fails transiently (deadlock, network blip, PG restart mid-request) the row's status stays `ready` and the same one-shot GDPR download token stays valid until `expires_at`. That silently degrades a one-shot token to unlimited replay of a personal-data export on any DB blip. The fix is a two-part change: (a) run `mark_downloaded` *before* materializing the body so a failure returns 500 instead of leaking the export, and (b) propagate its `Result` (return 500 on `Err`) so status-inconsistency can never silently ship the payload.

## Evidence
- `backend/servers/api-server/src/routes/gdpr.rs:198-201` — `let _ = state.data_export_repo.mark_downloaded(export_request.id).await;` — full `let _ =` await, no `?`, no error branch. The audit-log call above it (`:179-198`) only logs an error and moves on, which is fine for an audit trail but not for the one-shot status transition.
- `backend/crates/db/src/repositories/data_export.rs:271-281` — `get_by_token` returns rows only when `status = 'ready'`; nothing else gates re-use.
- `backend/crates/db/src/repositories/data_export.rs:283-299` — `mark_downloaded` is `UPDATE data_export_requests SET status = 'downloaded', downloaded_at = NOW(), download_count = download_count + 1 WHERE id = $1` (returns `Result<(), SqlxError>`). A failed UPDATE = row keeps `status='ready'` = token still resolvable via `get_by_token`.
- Ordering in `download_export`: `mark_downloaded` at :198-201 fires **before** `collect_user_data` at :209-213 — currently structured so a mark-fail returns nothing observable to the caller but the caller still gets the payload one line later. If the payload assembly also fails after mark-fail, the client sees 500 but the token is still valid for the next replay.
- Similar handler at `gdpr.rs:215` — `Ok(Json(serde_json::to_value(user_data).unwrap_or_default()))` uses `unwrap_or_default()` on `serde_json`, which pairs with this same "mark first, ship anyway" pattern — see the sibling backlog row `code-review-api-core-data-export-null-on-serialize-fail` (out of scope here, but the same handler needs the same discipline).

## Files
- `backend/servers/api-server/src/routes/gdpr.rs`
- `backend/crates/db/src/repositories/data_export.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [x] C2 — Seed data
- [x] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

Mode: cloud-ok

## Repro steps
1. In a test that has DB access, insert a `data_export_requests` row with `status='ready'`, `download_token=<uuid>`, `expires_at=NOW()+interval '1 hour'`.
2. Force `mark_downloaded` to fail (e.g. cast the pool to a wrapper that returns `Err(SqlxError::PoolClosed)` for that one query, or wrap the repository behind a mock; simplest for a real integration test is to inject a `sqlx::Pool` that has been closed after the token row was seeded — the subsequent UPDATE will fail).
3. Call `GET /api/v1/gdpr/exports/download/<token>` with a valid session.
4. Expected: response is `500 Internal Server Error` and the row stays `status='ready'` (so a caller might retry, but that is a bug for a follow-up fix — for the token reuse gate we accept the "still ready" branch by returning 500 and NOT leaking the payload).
5. Actual today: the handler streams the export body with `200 OK`, the row keeps `status='ready'` because `mark_downloaded` errored, and calling `GET /download/<token>` a second time returns the payload again — unbounded replay window until `expires_at`.
6. The IG3 test (see *Test plan*) is a smaller unit-level version of this that stubs the repository and asserts the handler does not stream the body when `mark_downloaded` fails.

## Suggested approach
1. **Reorder + propagate.** In `download_export` (`gdpr.rs:~198-215`), replace the `let _ = mark_downloaded(...).await;` with `state.data_export_repo.mark_downloaded(export_request.id).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;` and move it **before** `collect_user_data`. This is the smallest correctness fix — a failed mark now short-circuits the response and the token remains `ready` for exactly one more attempt (bounded by the existing `expires_at`).
2. **Prefer a CAS-style guard** at the repository layer: change `mark_downloaded` to `UPDATE data_export_requests SET status='downloaded', downloaded_at=NOW(), download_count=download_count+1 WHERE id = $1 AND status = 'ready'` and return `Result<bool, SqlxError>` — true when a row transitioned, false when someone else already downloaded it. The handler then treats `Ok(false)` as `410 Gone` (already downloaded). This closes the race between two concurrent GET requests using the same token, which the reorder alone doesn't cover.
3. **Update the handler wiring** at `gdpr.rs:~198-215`: use `Ok(transitioned) = mark_downloaded(id).await?; if !transitioned { return Err((StatusCode::GONE, "Export already downloaded".into())); }` before the `collect_user_data` call.
4. **Do NOT change `get_by_token`** — the existing `status = 'ready'` filter is exactly what makes the CAS above meaningful; changing it would defeat the guard.
5. **Add a regression test** — see *Test plan* — that stubs the repository so `mark_downloaded` returns `Err(...)` and asserts the handler returns 500 and never invokes `collect_user_data` (assert via a mock counter). Add a second test asserting Ok(false) → 410.

## Alternatives considered
- **Wrap `mark_downloaded` + `collect_user_data` in a DB transaction** — rejected because `collect_user_data` currently reads from several tables (audit_logs, messages, …) and adding a transactional envelope would either lock those tables for the duration of the export streaming (potentially seconds), or force us to buffer everything into memory. The CAS-on-status pattern gives the same one-shot guarantee with none of the lock scope.
- **Delete the row on download** — rejected because the audit trail intentionally keeps `data_export_requests` for compliance reporting (`expire_old_requests` at `data_export.rs:~301` transitions ready/downloaded → expired but does not delete). A CAS on `status` preserves that history.

## Root-cause trace
1. Symptom: a caller who obtained a GDPR download token can re-issue `GET /api/v1/gdpr/exports/download/<token>` after a transient DB failure and receive the personal-data payload again, without any new authorization step, until `expires_at`.
2. ← `get_by_token` at `backend/crates/db/src/repositories/data_export.rs:271-281` returns the same row on the second call because its status is still `'ready'`.
3. ← `mark_downloaded` failed (transient PG error) but its `Result` was discarded by `let _ =` at `backend/servers/api-server/src/routes/gdpr.rs:198-201`; the handler streamed the body regardless.
4. ← The handler was written to mark-and-forget on the assumption that DB errors on the mark step are noise. That assumption converts a status transition into a best-effort log line, breaking the one-shot invariant the token contract depends on.
5. Origin: initial `download_export` implementation — no CAS guard, no error propagation. There is no regression test that exercises the "mark_downloaded fails" branch, so the drift has been latent since the file was first added.

## Test plan
- [ ] `backend/servers/api-server/tests/gdpr_download_token_replay_test.rs` (new) — spin up a `TestApp` with a stub `DataExportRepo` (or use `sqlx::test` with a helper that force-errors `mark_downloaded`), assert that a token whose `mark_downloaded` returns `Err(...)` yields `500 Internal Server Error` from `GET /api/v1/gdpr/exports/download/<token>` and does **not** invoke `collect_user_data` (verify by asserting the mock's call count is zero).
- [ ] Second case in the same file: after a successful download the same token now returns `410 Gone` from `GET /download/<token>` (once the CAS guard is in place).
- [ ] IG3 assertion — before the fix, run the first test against the current handler and assert it FAILS (returns 200 with a body); after the fix, assert it passes.
- [ ] Run: `cd backend && cargo test -p api-server --test gdpr_download_token_replay_test`.

## Out of scope
- The audit-log call at `gdpr.rs:179-198` — best-effort logging is intentional for the audit trail and not the token-lifecycle concern here.
- `serde_json::to_value(user_data).unwrap_or_default()` at `gdpr.rs:215` — separate backlog row (`code-review-api-core-data-export-null-on-serialize-fail`) — do not conflate with this PR.
- The `LIMIT 1000` truncation in `data_export.rs:587-597` / `:692-704` — separate backlog row (`code-review-api-core-data-export-limit1000-truncation`), Article 15(3) completeness, distinct concern.
- S3 streaming migration hinted at in the source comment `// In a real implementation, this would stream the file from S3` — out of scope; the fix here is purely lifecycle correctness.

## After-merge
- Move this file to `plans/_archive/code-review-api-core-data-export-download-token-reuse.md`
- Mark the matching `backlog.json` row as `status: "done"`
