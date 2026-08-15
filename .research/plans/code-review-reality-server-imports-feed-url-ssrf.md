# code-review-reality-server-imports-feed-url-ssrf

**Vector:** bug
**Score:** 3
**Source:** Tier1d review 2026-08-13 (reality-server)
**Confidence:** medium

## Hypothesis
`reality-server` exposes two parallel feed-import surfaces. `agency_imports.rs` calls `crate::util::url_validator::validate_fetch_url()` on the caller-supplied URL at submission time, but the sibling `imports.rs` handlers (`create_feed`, `update_feed`, `sync_feed`) persist and later fetch `feed_url` with no SSRF guard. A portal-agency user can register `feed_url=http://169.254.169.254/…` (or any RFC1918/loopback host) and have the async import worker fetch it — SSRF against cloud metadata / internal services. Smallest fix: call the existing `validate_fetch_url()` at the same three entry points in `imports.rs` and reject invalid URLs before persistence, matching the pattern already used one file over.

## Evidence
- `backend/servers/reality-server/src/routes/imports.rs:339` `create_feed`, `:399` `update_feed`, `:436` `sync_feed` — accept `feed_url` from `CreateFeedSubscription` / `UpdateFeedSubscription` with no validator call.
- `backend/servers/reality-server/src/routes/agency_imports.rs:238` `test_connection`, `:290` `run_import` — explicitly call `validate_fetch_url(url)` with the audit comment `SECURITY (H3, round-9 audit): reject SSRF-prone feed URLs at submission time`.
- `grep -rn 'validate_fetch_url' backend/servers/reality-server/src/` — only `agency_imports.rs` hits; `imports.rs` never references it.
- Both surfaces converge on the same async import/sync worker that opens an outbound HTTP request against the persisted `feed_url`.

## Files
- `backend/servers/reality-server/src/routes/imports.rs:339`
- `backend/servers/reality-server/src/routes/imports.rs:399`
- `backend/servers/reality-server/src/routes/imports.rs:436`
- `backend/servers/reality-server/src/routes/agency_imports.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**

Mode: cloud-ok

## Repro steps
1. Start reality-server locally against a seeded DB with a portal-agency account.
2. `POST /api/v1/imports/feeds` with body `{"feed_url":"http://169.254.169.254/latest/meta-data/", …}` — the request should be **rejected** at submission time with `400`, but today it returns `201` and the URL is persisted.
3. Trigger `POST /api/v1/imports/feeds/{id}/sync` — the sync worker performs an outbound GET to `169.254.169.254`. Expected: refused before persistence; actual: fetched.

## Suggested approach
1. In `backend/servers/reality-server/src/routes/imports.rs`, import `crate::util::url_validator::validate_fetch_url` at the top of the module.
2. In `create_feed` (`:339`), after extracting `payload.feed_url`, call `validate_fetch_url(&payload.feed_url)?` before any DB write. Map error to `AppError::BadRequest("invalid feed_url: …")`.
3. In `update_feed` (`:399`), gate the same way when `payload.feed_url` is `Some(_)`.
4. In `sync_feed` (`:436`), re-validate the persisted `feed_url` before dispatching the fetch — belt-and-braces in case a row predates the validator.
5. Extract shared error mapping into a small helper if the three call sites duplicate too much.
6. Confirm the existing `agency_imports.rs` pattern (`:238`, `:290`) is untouched — this plan aligns `imports.rs` with the sibling, it does not rewrite the shared validator.
7. Add an audit comment mirroring the `SECURITY (H3, round-9 audit)` line so future greps find both surfaces.

## Alternatives considered
- **Validate inside `url_validator::validate_fetch_url()`'s caller only in the worker (defer to sync time)** — rejected because a persisted invalid `feed_url` still consumes rows, still enables blind-SSRF probes via the sync trigger, and diverges from `agency_imports.rs` which validates at submission.
- **Introduce a new middleware for all outbound-URL handlers** — rejected because scope creep for a two-line fix that just needs to call the existing helper; a middleware refactor should be its own plan.

## Root-cause trace
1. Symptom: `POST /api/v1/imports/feeds` accepts a private-IP `feed_url` and the worker fetches it.
2. ← immediate cause at `backend/servers/reality-server/src/routes/imports.rs:339` — no `validate_fetch_url` call on `payload.feed_url`.
3. ← upstream cause: `imports.rs` and `agency_imports.rs` implement the same feature twice; the H3 audit fix landed in only the second file.
4. Origin: the split between the two import surfaces predates the H3 SSRF audit; the audit patch touched `agency_imports.rs` only.

## Test plan
- [ ] Integration test in `backend/servers/reality-server/tests/` (or the crate's existing `routes/imports` test module) that POSTs a `feed_url=http://127.0.0.1/foo` and asserts `400`.
- [ ] Regression test: `feed_url=http://169.254.169.254/latest/meta-data/` → `400`, no DB row.
- [ ] Positive test: a valid public HTTPS URL still round-trips create → update → sync.
- [ ] `cd backend && cargo test -p reality-server routes::imports`

## Out of scope
- Refactoring `imports.rs` and `agency_imports.rs` into a shared module (tracked separately if the duplication becomes worse).
- Widening `validate_fetch_url`'s allow-list or denylist — the existing helper is the contract.
- Rewriting the sync worker's HTTP client.

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-imports-feed-url-ssrf.md`
- Mark the matching `backlog.json` row as `status: "done"`
