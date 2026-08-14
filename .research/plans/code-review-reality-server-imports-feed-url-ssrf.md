# code-review-reality-server-imports-feed-url-ssrf

**Vector:** security
**Score:** 3
**Source:** Tier1d review 2026-08-13 (reality-server)
**Confidence:** medium

## Hypothesis
`reality-server`'s `/api/v1/imports/feeds*` create/update/sync handlers persist and later fetch a caller-supplied `feed_url` with **no** SSRF guard, while the sibling `agency_imports` handlers correctly call `crate::util::url_validator::validate_fetch_url` (with an explicit "reject SSRF-prone feed URLs at submission time" comment). A portal-agency user can register `feed_url=http://169.254.169.254/...` or `http://localhost:...` and drive the import worker to fetch cloud-metadata or internal-service endpoints. The smallest correct change is to call the existing `validate_fetch_url()` guard in `create_feed` and `update_feed` at submission time (mirror `agency_imports.rs`), plus re-validate at fetch time in the sync worker as belt-and-suspenders against DNS-rebind / TOCTOU.

## Evidence
- `backend/servers/reality-server/src/routes/imports.rs:339` `create_feed` — accepts `Json<CreateFeedSubscription>` and persists `feed_url` unchecked
- `backend/servers/reality-server/src/routes/imports.rs:399` `update_feed` — accepts `Json<UpdateFeedSubscription>` and persists `feed_url` unchecked
- `backend/servers/reality-server/src/routes/imports.rs:436` `sync_feed` — triggers a fetch of the persisted `feed_url` unchecked
- `backend/servers/reality-server/src/routes/agency_imports.rs:238,290` — sibling handlers correctly call `validate_fetch_url(url)` at submission time
- `backend/servers/reality-server/src/util/url_validator.rs:116` `pub fn validate_fetch_url(raw: &str) -> Result<Url, UrlValidationError>` is the shared guard

## Files
- `backend/servers/reality-server/src/routes/imports.rs:339`
- `backend/servers/reality-server/src/routes/imports.rs:399`
- `backend/servers/reality-server/src/routes/imports.rs:436`
- `backend/servers/reality-server/src/util/url_validator.rs:116`
- `backend/servers/reality-server/src/routes/agency_imports.rs:238`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):**
- No C4 or C5 → `cloud-ok`

Mode: cloud-ok

## Repro steps
1. As an authenticated portal-agency user, POST `/api/v1/imports/feeds` with body `{ "feed_url": "http://169.254.169.254/latest/meta-data/", "kind": "generic-xml", ... }`. Endpoint returns 201 (feed persisted).
2. POST `/api/v1/imports/feeds/<id>/sync`. Import worker fetches the metadata endpoint (SSRF). Expected: submission is rejected 400 with `UrlValidationError`; sync worker refuses to fetch a private/reserved-range host.

## Suggested approach
1. Add `use crate::util::url_validator::validate_fetch_url;` to `imports.rs` header (mirror `agency_imports.rs:7`).
2. In `create_feed` (`imports.rs:339`), immediately after `Json(data)` extraction, call `validate_fetch_url(&data.feed_url).map_err(|e| ... 400 ...)` before any repository call.
3. In `update_feed` (`imports.rs:399`), if the incoming `UpdateFeedSubscription` carries `feed_url: Some(u)`, call `validate_fetch_url(u)` on that value before persisting; unchanged case (`None`) needs no check.
4. In `sync_feed` (`imports.rs:436`) or the shared fetch helper that ultimately runs the HTTP GET, re-run `validate_fetch_url(&loaded.feed_url)` right before the reqwest call (defense-in-depth against DNS rebinding + TOCTOU between submission and sync).
5. Add tests in the existing imports handler tests (or the util tests colocated with `agency_imports`), asserting create/update/sync reject the four canonical SSRF cases (`http://127.0.0.1`, `http://10.0.0.1`, `http://169.254.169.254`, `http://[::1]/`) and accept a public `https://example.com/feed.xml`.
6. `just verify` — expect green.

## Alternatives considered
- **Gate the entire `imports` router behind a middleware that re-validates every JSON body field named `feed_url`** — rejected because it is more invasive than the point-fix, requires reflection-style validation over deserialized bodies, and does not compose with per-handler error mapping.
- **Only validate at fetch time in the sync worker** — rejected because a persisted attacker URL then sits latent in the DB, and every re-sync/schedule is a fresh SSRF attempt; validation at submission time is the primary layer, sync-time is defence-in-depth.

## Root-cause trace
1. Symptom: portal-agency user registers `feed_url=http://169.254.169.254/...`, import worker fetches it → SSRF.
2. ← Fetch happens at `imports.rs:436` `sync_feed` (or the shared fetch helper it calls) with no URL guard.
3. ← Persistence happens at `imports.rs:339` `create_feed` / :399 `update_feed`, which accept the raw `feed_url` from the request body with no `validate_fetch_url` call.
4. ← Sibling `agency_imports.rs` was hardened during round-9 audit ("SECURITY (H3, round-9 audit)"); `imports.rs` is an unhardened parallel of the same feature that missed the sweep.
5. Origin: pre-round-9 code; the shared `util/url_validator.rs::validate_fetch_url` was added but only wired into `agency_imports.rs`.

## Test plan
- [ ] `cargo test -p reality-server routes::imports` — new cases assert `create_feed`, `update_feed`, `sync_feed` reject `http://127.0.0.1`, `http://10.0.0.1`, `http://169.254.169.254`, `http://[::1]/`; and accept a public `https://` URL.

## Out of scope
- Reworking the render-URL validator drift (`validate_image_or_link_url` duplicated in `reports.rs` + `agency_branding.rs`) — separate backlog row `code-review-reality-server-dup-url-validators`; do NOT bundle.
- Introducing a shared `PageLimit`-style newtype for other unclamped inputs — separate refactor.
- Retroactively scanning already-persisted `feed_url` values in production data.

## After-merge
- Close the backlog row `code-review-reality-server-imports-feed-url-ssrf`.
- Grep the codebase for other handlers that accept a `_url` body field and forward it to a fetcher without the shared guard; file follow-up backlog rows for each hit.
- Consider tightening `validate_fetch_url` to also refuse `169.254.169.254` explicitly (the current CIDR block already covers it, but an explicit test guards against future changes).
