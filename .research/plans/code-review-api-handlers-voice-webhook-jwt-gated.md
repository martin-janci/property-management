# code-review-api-handlers-voice-webhook-jwt-gated

**Vector:** bug
**Score:** 3
**Source:** hotspot in backend/servers/api-server/src/routes/voice_webhooks.rs
**Confidence:** high

## Hypothesis
Both public voice webhooks — `POST /api/v1/webhooks/voice/alexa` and `/google` — take `RlsConnection` as an Axum extractor argument. Extractors run before the handler body, and `RlsConnection` transitively requires a Property Management JWT in an `Authorization` header plus a resolved tenant. Amazon and Google send neither, so every genuine inbound delivery is rejected with a plain-text `401 Missing Authorization header` before any signature verification, device authentication, or command processing runs. The smallest change that resolves it is to drop the `RlsConnection` extractor from these two handlers and acquire an un-principaled connection inside the body — binding RLS context from the device the platform signature resolves — exactly as `oauth_token_refresh` already does in the same file.

## Evidence
- `backend/servers/api-server/src/routes/voice_webhooks.rs:168` — `mut rls: RlsConnection,` in the `alexa_webhook` argument list; `:307` — the same in `google_actions_webhook`.
- `backend/servers/api-server/src/lib.rs:339` — the router is nested at `/api/v1/webhooks/voice` with no auth-exempting layer, and `voice_webhooks.rs:130` `voice_webhook_router()` applies no layer either.
- `backend/crates/api-core/src/extractors/rls_connection.rs:232` — the extractor's first step is `ValidatedTenantExtractor::from_request_parts(parts, state).await?`, which at `backend/crates/api-core/src/extractors/tenant.rs:137` calls `AuthUser::from_request_parts`, which at `backend/crates/api-core/src/extractors/auth.rs:272` rejects a missing `Authorization` header with `401`.
- `backend/servers/api-server/src/routes/voice_webhooks.rs:697` — `oauth_token_refresh` in the same module already uses the correct pattern: it takes `State(state)` and acquires the connection itself.
- No integration test in `backend/servers/api-server/tests/` ever POSTs to `/api/v1/webhooks/voice/alexa` or `/google` — the ~60 unit tests in the module call helper functions directly, which is why the gate was never observed.

## Files
- `backend/servers/api-server/src/routes/voice_webhooks.rs`
- `backend/servers/api-server/src/lib.rs`
- `backend/crates/api-core/src/extractors/rls_connection.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [x] C7 — Code-review reception (tick if you expect controversy)

Mode: cloud-ok

## Repro steps
1. Build the api-server router as the integration-test harness does (see `backend/servers/api-server/tests/suites/voice_oauth_exchange_auth_tests.rs` for the existing setup).
2. Send `POST /api/v1/webhooks/voice/alexa` with a JSON Alexa `LaunchRequest` body and **no** `Authorization` header and **no** `X-Tenant-ID` header — the exact shape Amazon delivers.
3. Expected: the handler body runs and rejects on signature verification, returning a JSON `ErrorResponse` with `code: "INVALID_SIGNATURE"`. Actual: a plain-text `401 Missing Authorization header` produced by the extractor, meaning `verify_alexa_signature` never executed.

## Suggested approach
1. In `voice_webhooks.rs:166`, change `alexa_webhook`'s signature to take `State(state): State<AppState>`, `headers: HeaderMap`, `body: Bytes` — dropping the `mut rls: RlsConnection` argument.
2. Run `verify_alexa_signature(&headers, &body).await` and the body parse first, while holding no database connection at all.
3. Acquire a public (un-principaled) connection with the same `RlsPool` pattern used at `voice_webhooks.rs:697-704`, use it for `authenticate_voice_user(...)` to resolve the owning device, then re-bind RLS context to that device's `organization_id` / `user_id` with `acquire_with_rls(...)` as at `voice_webhooks.rs:843-855` before invoking `VoiceCommandProcessor`.
4. Restructure the fallible flow as `let result = async { … }.await;` followed by a single unconditional `release().await` and `return result`, so no exit path leaves the connection to `Drop`.
5. Apply the same three changes to `google_actions_webhook` at `voice_webhooks.rs:305`, whose `verify_google_request` check has the identical ordering problem.
6. Leave the `/oauth/exchange`, `/oauth/refresh`, `/verify` and `/alexa/health` routes untouched — they are separately reachable and out of scope here.
7. Add the integration test from *Test plan* and confirm it fails against the current code before the change.

## Alternatives considered
- **Keep `RlsConnection` and add an auth-bypass layer on the voice router** — rejected because it would punch a hole in the tenant-validation extractor for a whole route subtree, and the codebase's established posture for unauthenticated inbound receivers (`backend/servers/api-server/src/routes/integrations/webhook.rs`) is to take no request principal at all and derive RLS context from the server-resolved owner.
- **Have Amazon/Google send a service JWT so the extractor passes** — rejected because neither platform supports injecting a custom bearer token on skill/action delivery; the platforms authenticate with their own signature schemes, which this module already implements.

## Root-cause trace
1. Symptom: every real Alexa/Google delivery to `/api/v1/webhooks/voice/{alexa,google}` returns plain-text `401 Missing Authorization header`; the module's signature verification and command processing never execute.
2. ← Immediate cause at `backend/crates/api-core/src/extractors/auth.rs:272` — `AuthUser::from_request_parts` rejects the request because no `Authorization` header is present.
3. ← Upstream cause at `backend/crates/api-core/src/extractors/tenant.rs:137` and `backend/crates/api-core/src/extractors/rls_connection.rs:232` — `RlsConnection` runs `ValidatedTenantExtractor`, which runs `AuthUser`, before any handler code.
4. Origin: the `mut rls: RlsConnection` argument on `alexa_webhook` (`backend/servers/api-server/src/routes/voice_webhooks.rs:168`) and `google_actions_webhook` (`:307`) — an authenticated-endpoint idiom applied to two endpoints whose callers are external platforms that cannot authenticate that way.

## Test plan
- [ ] New `backend/servers/api-server/tests/suites/voice_alexa_webhook_route_tests.rs`, registered as a `mod` in `backend/servers/api-server/tests/suite_8.rs` alongside the existing `voice_oauth_*_auth_tests` modules: POST an Alexa body to `/api/v1/webhooks/voice/alexa` with no `Authorization` and no `X-Tenant-ID`, assert the response body is JSON carrying `INVALID_SIGNATURE` rather than the extractor's plain-text `Missing Authorization header`.
- [ ] Same-shape case for `POST /api/v1/webhooks/voice/google`, asserting the handler's `INVALID_REQUEST` JSON rather than the extractor rejection.
- [ ] `cd backend && cargo test -p api-server --test suite_8 voice_alexa_webhook`

## Out of scope
- Changing the signature-verification algorithms themselves (`verify_alexa_signature`, `verify_google_request`) — they are already hardened and this plan only makes them reachable.
- The `/oauth/exchange` and `/oauth/refresh` error-message leak, tracked separately as `code-review-api-handlers-voice-oauth-error-leak`.
- Any change to `VoiceCommandProcessor` behaviour once it is finally reached.

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-voice-webhook-jwt-gated.md`
- Mark the matching `backlog.json` row as `status: "done"`
