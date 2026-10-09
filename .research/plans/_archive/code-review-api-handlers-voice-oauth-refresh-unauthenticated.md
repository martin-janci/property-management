# code-review-api-handlers-voice-oauth-refresh-unauthenticated

**Vector:** security
**Score:** 2
**Source:** Phase 1.5 dev-review 2026-08-15 (segment=api-handlers, churn-aligned voice_webhooks.rs +72 lines after PR #2748)
**Confidence:** high

## Hypothesis
`POST /api/v1/webhooks/voice/oauth/refresh` accepts an unauthenticated `{device_id: Uuid}`, then unconditionally calls the upstream OAuth manager and overwrites the linked user's stored `access_token_encrypted`/`access_token_hash`. Any caller that learns a device_id can rotate that user's OAuth token (integration DoS + amplification against Amazon/Google OAuth endpoints). Sibling `oauth_token_exchange` already takes an `AuthUser` extractor — the refresh handler must gate on the same principal (owner or platform admin), or on a shared webhook secret / HMAC signature over the body. Smallest safe change: attach `AuthUser` and reject when `auth.user_id` doesn't own the device row.

## Evidence
- `backend/servers/api-server/src/routes/voice_webhooks.rs:535` — `async fn oauth_token_refresh(State(state): State<AppState>, Json(request): Json<VoiceTokenRefreshRequest>) -> Result<...>` — no auth extractor.
- `backend/servers/api-server/src/routes/voice_webhooks.rs:69` — `.route("/oauth/refresh", post(oauth_token_refresh))` — mounted under `/api/v1/webhooks/voice`, reachable from mod.rs:126 `pub mod voice_webhooks`.
- `backend/crates/db/src/models/llm_document.rs:627` — `VoiceTokenRefreshRequest { device_id: Uuid }` — no signature, no bearer, no principal.
- Contrast: `oauth_token_exchange` in the same file takes `auth: api_core::AuthUser`, so the callsite pattern is already in-tree.
- Downstream write path: `oauth_manager.refresh_token(...)` rotates `access_token_encrypted` + `access_token_hash` on the linked-device row.

## Files
- `backend/servers/api-server/src/routes/voice_webhooks.rs:535`
- `backend/servers/api-server/src/routes/voice_webhooks.rs:69`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception

**Execution mode:** `Mode: cloud-ok`

## Repro steps
1. Boot api-server against a DB that has at least one linked voice device with a live OAuth session.
2. As an unauthenticated caller (no bearer, no cookie), `POST /api/v1/webhooks/voice/oauth/refresh` with body `{"device_id":"<uuid-of-known-device>"}`.
3. Expected: `401 Unauthorized`. Actual: `200 OK` — upstream Amazon/Google refresh call fires and the device's `access_token_hash` on `voice_devices` rotates. Reproduce a second time to observe the rotation stat go up while the caller stays anonymous.

## Suggested approach
1. Add `auth: api_core::AuthUser` extractor to `oauth_token_refresh` at `voice_webhooks.rs:535` (same shape as the sibling `oauth_token_exchange`).
2. Before calling `oauth_manager.refresh_token`, load the device row and require either `auth.user_id == device.user_id` OR `auth.role == PlatformAdmin`; otherwise return `AppError::Forbidden` and log the mismatch with the caller's principal.
3. Wrap the whole handler in the existing per-user rate limit used elsewhere in this module (`with_rate_limit(auth.user_id, ...)` if present, else a simple `tower_governor` layer on this route) so authenticated abuse is still capped.
4. Add an integration test in `backend/servers/api-server/tests/suites/voice_webhooks_tests.rs` (create the suite file if it doesn't exist) that:
   - Sends the request with no auth → asserts 401.
   - Sends it as a user who doesn't own the device → asserts 403.
   - Sends it as the owner → asserts 200 + `access_token_hash` rotates.
5. Run `cargo test -p api-server voice_webhooks` and confirm all three assertions pass; the "no auth" case must currently FAIL on `dev` (that's IG3).
6. Update the OpenAPI utoipa annotation for the endpoint to document the new authentication requirement.

## Alternatives considered
- **Shared-secret HMAC over the body** — rejected because the endpoint already lives in the "authenticated PM caller" surface (sibling `oauth_token_exchange` uses `AuthUser`); mixing signature-auth into an otherwise-bearer route doubles the auth surface for no benefit and duplicates key management.
- **Rate-limit only (no auth change)** — rejected because it does not close the vulnerability: an attacker still gets one free token rotation per device per rate window, and rotating any linked user's token is enough to break the integration.

## Root-cause trace
1. Symptom: unauthenticated POST to `/api/v1/webhooks/voice/oauth/refresh` returns 200 and rotates a linked user's OAuth token.
2. ← `oauth_token_refresh` handler (`voice_webhooks.rs:535`) takes only `State` + `Json<VoiceTokenRefreshRequest>` — no auth extractor.
3. ← Router wiring (`voice_webhooks.rs:69`) mounts the handler without any middleware that inserts an auth check, and `VoiceTokenRefreshRequest` (`db/models/llm_document.rs:627`) carries no principal.
4. Origin: the OAuth-refresh endpoint was added alongside the exchange endpoint but shipped without an auth extractor — most likely modelled on the *webhook* surface (which uses HMAC) rather than the *bearer* surface. Blame `git log -L :oauth_token_refresh:backend/servers/api-server/src/routes/voice_webhooks.rs` for the introducing commit.

## Test plan
- [ ] `backend/servers/api-server/tests/suites/voice_webhooks_tests.rs::oauth_refresh_rejects_unauthenticated` — asserts 401 with no bearer.
- [ ] `voice_webhooks_tests.rs::oauth_refresh_rejects_non_owner` — asserts 403 for wrong user.
- [ ] `voice_webhooks_tests.rs::oauth_refresh_owner_rotates_token` — asserts 200 + `access_token_hash` changes for the device row.
- [ ] `cargo test -p api-server voice_webhooks`

## Out of scope
- Rewriting the Amazon/Google OAuth refresh flow.
- Migrating other unauthenticated voice webhook handlers (`alexa_webhook`, `google_webhook`) — those have HMAC and are tracked separately.
- Rate-limiter framework choice (use whatever is already in-tree; do not introduce a new dep).

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-voice-oauth-refresh-unauthenticated.md`
- Mark the matching `backlog.json` row as `status: "done"`
