# code-review-reality-server-imports-raw-err-leak

**Vector:** security
**Score:** 2
**Source:** rotating-expert-review reality-server 2026-09-03
**Confidence:** high

## Hypothesis
Six `map_err` sites in `reality-server` route handlers (five in `routes/imports.rs`, one in `routes/agencies.rs`) return `format!("Failed to ...: {}", other)` as the 500 body, echoing raw `sqlx::Error` text (constraint names, column names, DSN detail) to authenticated realtor clients. `crate::util::errors::db_error` exists specifically to scrub that text and was already applied across `listings.rs` / `realtors.rs` / `layout.rs` under the earlier `code-review-reality-server-db-error-leak-to-client` fix, but the sweep never reached `imports.rs` + `agencies.rs`. Route every offending `map_err` through `db_error` so the 500 body becomes the generic `"Internal server error"`.

## Evidence
- `backend/servers/reality-server/src/routes/imports.rs:223,259,294,417,453` — five sites of the leak pattern (`format!("Failed to update import job: {}", other)`, etc.), each returned directly as the 500 String body
- `backend/servers/reality-server/src/routes/agencies.rs:280` — same shape on the agencies `update_agency` route
- `backend/servers/reality-server/src/util/errors.rs` — the `db_error` helper the rest of the crate uses; its doc-comment states `"Reality-server is internet-facing. Raw sqlx::Error ... can leak column names, constraint names"`
- Prior fix for the identical pattern in the same crate: `code-review-reality-server-db-error-leak-to-client` (backlog id, dropped after listings.rs/realtors.rs/layout.rs sweep) — this is the residual set that sweep missed

## Files
- `backend/servers/reality-server/src/routes/imports.rs`
- `backend/servers/reality-server/src/routes/agencies.rs`
- `backend/servers/reality-server/src/util/errors.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. In `backend/servers/reality-server`, cause `update_agency` (`agencies.rs:280`) to hit any non-`RowNotFound` `sqlx::Error` — for example, run the reality-server integration harness against a schema where the `agencies.slug` unique index has been renamed, then `PATCH /api/v1/agencies/<id>` from an authenticated realtor.
2. Expected: 500 body is a scrubbed `"Internal server error"`.
   Actual (today): 500 body is `Failed to update agency: error returned from database: duplicate key value violates unique constraint "agencies_slug_key" ...` — the constraint name (and any column / DSN detail sqlx surfaces) reaches the client.
3. Same reproduction with any of the five `imports.rs` sites — e.g. `POST /api/v1/imports/<id>/start` hitting a rewritten `imports.status` check constraint yields raw driver text in the response body.

## Suggested approach
1. Read `backend/servers/reality-server/src/util/errors.rs` to confirm the current `db_error` signature — it's a `fn(context: &str, err: sqlx::Error) -> (StatusCode, String)` that returns `"Internal server error"` for every variant.
2. In `imports.rs`, replace each of the five leaking `map_err(|other| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to ...: {}", other)))` calls with `map_err(|e| crate::util::errors::db_error("<matching context>", e))`, preserving any surrounding `match` on `sqlx::Error::RowNotFound` / typed variants that intentionally choose a non-500 status.
3. Same substitution at `agencies.rs:280`.
4. Add one test module in each touched route file (or extend the existing one) that asserts, given a `sqlx::Error::Protocol("column \"foo\" not found ...")`-shape driver error, the 500 body equals `"Internal server error"` and contains none of the driver text — mirroring the pattern from `backend/servers/api-server/src/routes/compliance.rs::tests` added in PR #2925.
5. Grep the two files for any remaining `format!(.*\{.*e[.:]?)` on a 500 branch — the fix should be file-local, no other routes affected.
6. Run `cargo test -p reality-server --tests` in `backend/`; run `cargo clippy -p reality-server -- -D warnings`.

## Alternatives considered
- **Add a middleware that scrubs error bodies globally** — rejected because it would also strip legitimate typed error text (e.g. the `SavedSearchError::LimitReached { max }` string from PR #2922) and would hide, not fix, the class of bug where a handler forgot to route through `db_error`.
- **Change `db_error` to also log-and-swallow specific `sqlx::Error` variants (e.g. `RowNotFound`)** — rejected because those variants deserve typed 404 mappings at the call site (as `SavedSearchError::NotFound` already does), not a hidden reinterpretation inside the sanitizer.

## Root-cause trace
1. Symptom: 500 responses from `/api/v1/imports/*` and `PATCH /api/v1/agencies/{id}` echo raw sqlx driver text back to authenticated realtor clients.
2. ← Handler `map_err` interpolates `sqlx::Error` into the response body at `imports.rs:223,259,294,417,453` and `agencies.rs:280`.
3. ← The earlier crate-wide sweep (`code-review-reality-server-db-error-leak-to-client`, promoted 2026-08-02 via security fast-track) covered `listings.rs` / `realtors.rs` / `layout.rs` but not these two files — residual sites in the same class.
4. Origin: the offending `format!` blocks predate the `util::errors::db_error` sanitizer and were never rewritten when that helper landed; each individual handler was written before the crate-wide "route through `db_error`" convention was established.

## Test plan
- [ ] Unit test in `routes/imports.rs::tests` asserting a `sqlx::Error::Protocol(...)` bubbled through the `map_err` boundary yields `StatusCode::INTERNAL_SERVER_ERROR` and body `"Internal server error"` (body must not contain `"column"`, `"constraint"`, `"host="`, or the raw `sqlx::Error::to_string()` output)
- [ ] Same-shape unit test in `routes/agencies.rs::tests` for the `update_agency` path
- [ ] Regression grep-guard test (same style as PR #2925's added `no_handler_bubbles_raw_db_error_to_client`) — `include_str!("imports.rs")` walks every non-comment line and asserts none contains `map_err(...) && e.to_string() && INTERNAL_SERVER_ERROR` together; same for `agencies.rs`
- [ ] `cd backend && cargo test -p reality-server` — expect green after fix

## Out of scope
- Structural refactor of `imports.rs` / `agencies.rs` beyond the six `map_err` sites — the fix is one edit per site.
- Behavior of `crate::util::errors::db_error` itself — it already returns the generic scrubbed body and is treated as authoritative.
- Any other `reality-server` route file — a repo-wide sweep already ran on 2026-08-02; only these two files are the residual.
- Adding new typed error enums (`ImportError`, `AgencyError`) — a follow-up refactor, not required to close the leak.

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-imports-raw-err-leak.md`
- Mark the matching `backlog.json` row as `status: "done"`
