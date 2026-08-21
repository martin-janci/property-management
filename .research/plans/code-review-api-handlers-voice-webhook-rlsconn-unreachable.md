# code-review-api-handlers-voice-webhook-rlsconn-unreachable

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review [api-handlers] 2026-08-21 — `.research/signals/2026-08-21-api-handlers-tier1d.json`
**Confidence:** high

## Hypothesis
The two primary voice webhook handlers — `alexa_webhook` at `backend/servers/api-server/src/routes/voice_webhooks.rs:165` and `google_actions_webhook` at `:304` — take `mut rls: RlsConnection` as a handler argument. `RlsConnection::from_request_parts` (backend/crates/api-core/src/extractors/rls_connection.rs:229-232) first runs `ValidatedTenantExtractor`, which in turn requires a valid PM `AuthUser` JWT (backend/crates/api-core/src/extractors/tenant.rs:135-137). Real Alexa / Google Assistant requests authenticate with a platform SIGNATURE (Alexa cert-chain / Google JWT-in-header), never with a PM Bearer token, so the extractor rejects every genuine platform request with `401` before the signature verification and `VoiceCommandProcessor` ever run. The Alexa and Google voice endpoints are unreachable in production; the entire hardened signature surface is dead code. Sibling `oauth_token_refresh` at `:694-707` and `portal_webhooks.rs:318-321` already show the correct pattern: signature-authenticated webhooks take only `State`/`HeaderMap`/`Bytes` and acquire an `RlsPool::acquire_public()` connection internally after signature verification. Apply that pattern to both voice handlers.

## Evidence
- `backend/servers/api-server/src/routes/voice_webhooks.rs:165-170` — `alexa_webhook(State, HeaderMap, mut rls: RlsConnection, body: Bytes)`; extractor runs before the handler body.
- `backend/servers/api-server/src/routes/voice_webhooks.rs:304-311` — `google_actions_webhook` — same signature shape, same reachability trap.
- `backend/crates/api-core/src/extractors/rls_connection.rs:229-232` — `RlsConnection::from_request_parts` calls `ValidatedTenantExtractor::from_request_parts` first; `backend/crates/api-core/src/extractors/tenant.rs:135-137` — that extractor requires `AuthUser` (PM JWT Bearer) and returns 401 otherwise.
- `backend/servers/api-server/src/routes/voice_webhooks.rs:694-724` — `oauth_token_refresh` shows the correct pattern: `RlsPool::new(state.db.clone()).acquire_public()` after signature verification, with `authenticate_voice_user`-style ownership check as the authz gate.
- `backend/servers/api-server/src/routes/voice_webhooks.rs:1826-1838` — the unit-test module explicitly leaves the four DB-holding handlers to an integration harness that never runs, which is why the reachability regression is invisible to `cargo test`.

## Files
- `backend/servers/api-server/src/routes/voice_webhooks.rs`
- `backend/crates/api-core/src/extractors/rls_connection.rs`
- `backend/crates/api-core/src/extractors/tenant.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

Mode: cloud-ok

## Repro steps
1. `cargo test -p api-server --test suites` — confirm no existing test hits `POST /api/v1/webhooks/voice/alexa` or `/api/v1/webhooks/voice/google` end-to-end (the handler-entrypoint gap the finding calls out).
2. Add a route-integration test using `axum::Router` (mirroring `portal_webhooks_signature_tests.rs`): construct a request with a valid Alexa signature and no PM JWT bearer header, dispatch through `voice_webhook_router()`.
3. Expected on `dev`: response `401` (extractor rejects before `verify_alexa_signature` runs). Actual after fix: `200` (signature verified, `authenticate_voice_user` gate applied, `VoiceCommandProcessor` runs).

## Suggested approach
1. `alexa_webhook` (`voice_webhooks.rs:165`): drop the `mut rls: RlsConnection` parameter; keep `State`, `HeaderMap`, `Bytes`. After `verify_alexa_signature` succeeds and the request is parsed, acquire the connection the same way `oauth_token_refresh` does at `:694-707`: `let rls_pool = RlsPool::new(state.db.clone()); let mut lookup = rls_pool.acquire_public().await?;` and pass `lookup.conn()` (or `&mut **lookup.conn()`) to `authenticate_voice_user` and `VoiceCommandProcessor::process_command`. Preserve the existing "no access-token → link account" branch by returning `build_alexa_link_account_response()` before acquiring the connection.
2. `google_actions_webhook` (`voice_webhooks.rs:304`): apply the identical refactor. Call `verify_google_request` first, then `acquire_public`, then `authenticate_voice_user`.
3. If a handler holds `lookup` across `await`s that mutate the same connection (e.g. `authenticate_voice_user` writes `last_seen_at`), match the `oauth_token_refresh` `drop(lookup)` pattern between the read-phase and the write-phase to avoid holding two mutable borrows.
4. Add an integration test file `backend/servers/api-server/tests/suites/voice_webhooks_reachability_tests.rs`: two tests — one per handler — build the request through `voice_webhook_router()` with a synthetic-but-verifiable signature setup (either a signing helper that mirrors production, or a `#[cfg(test)]` bypass gated so the test tickles the full pipeline). Assert `200 OK` and that the response body is an `AlexaSkillResponse` / Google fulfillment envelope, not a JSON error.
5. Leave `authenticate_voice_user` (`voice_webhooks.rs:1574-1681`) intact — it remains the authz gate; only the extractor scaffolding changes.
6. Run `cargo fmt --all && cargo clippy -p api-server --all-targets -- -D warnings && cargo test -p api-server --test suites voice_webhooks_reachability_tests -- --nocapture`. Confirm the two new tests fail on `dev` (baseline capture) and pass on the branch.

## Alternatives considered
- **Introduce a synthetic tenant-injecting middleware in `voice_webhook_router()`** — rejected because it fabricates a `principal.org_id` that has no real meaning for platform requests, and every downstream repo call would still run under `SET LOCAL app.org_id = <fake>`, which either leaks across tenants or matches nothing. Signature-then-`acquire_public()` is what other signature-authenticated webhooks already do and is the pattern the finding's CORROBORATION cites.
- **Remove the routes entirely and expose only the OAuth exchange/refresh** — rejected because Alexa / Google Assistant fulfilment traffic terminates at these very endpoints; deleting them removes the whole voice product surface, not just the reachability bug.

## Root-cause trace
1. Symptom: every genuine Alexa / Google Assistant POST returns `401 UNAUTHORIZED` before signature verification runs.
2. ← `alexa_webhook`/`google_actions_webhook` handler signature includes `mut rls: RlsConnection` at `voice_webhooks.rs:168` / `:307`.
3. ← `RlsConnection::from_request_parts` invokes `ValidatedTenantExtractor::from_request_parts` at `rls_connection.rs:229-232`.
4. ← `ValidatedTenantExtractor` demands `AuthUser` (PM JWT) at `tenant.rs:135-137`, which platform requests never carry — the whole request is short-circuited to `401` before the handler body runs.
5. Origin: initial voice-webhook wiring pre-dated the reachability regression — the extractor was borrowed from the JWT-authenticated `oauth_token_exchange` handler at `:400` and applied to the signature-authenticated fulfilment handlers without noticing that the two auth models are incompatible.

## Test plan
- [ ] `backend/servers/api-server/tests/suites/voice_webhooks_reachability_tests.rs::alexa_webhook_reaches_handler_with_valid_signature` — fails on `dev`, passes on the branch.
- [ ] `backend/servers/api-server/tests/suites/voice_webhooks_reachability_tests.rs::google_webhook_reaches_handler_with_valid_signature` — fails on `dev`, passes on the branch.
- [ ] `cargo test -p api-server --test suites voice_webhooks_reachability_tests` — both green.
- [ ] Existing `voice_webhooks` tests in `voice_webhooks.rs:1826+` remain green (no regression in the sub-helpers the reachability tests don't cover).
- [ ] `cargo clippy -p api-server --all-targets -- -D warnings` clean.

## Out of scope
- The related `voice-webhook-rls-release-inconsistent` finding (score 1 open item) — same file, different bug class; leave for a follow-up so this PR stays reviewable.
- Any change to `authenticate_voice_user` or to the OAuth exchange/refresh endpoints (already handled correctly).
- Adding automated signature-generation helpers beyond what the two reachability tests need.

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-voice-webhook-rlsconn-unreachable.md`
- Mark the matching `backlog.json` row as `status: "done"`
