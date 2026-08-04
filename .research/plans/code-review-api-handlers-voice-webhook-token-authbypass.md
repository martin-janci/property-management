# code-review-api-handlers-voice-webhook-token-authbypass

**Vector:** security
**Score:** 3
**Source:** Issue #2658 (tier1d-dispatcher-generator, api-handlers segment, 2026-08-04)
**Confidence:** high

## Hypothesis

`authenticate_voice_user` in `backend/servers/api-server/src/routes/voice_webhooks.rs` accepts an `_access_token: &str` parameter but never validates it — the parameter is underscore-prefixed and unused. The function returns whichever `voice_assistant_devices` row was most recently used on the given platform, regardless of who the caller is. Both live handlers `alexa_webhook` (line 127) and `google_actions_webhook` (line 243) call this helper and then execute the incoming voice command in the returned device's owning-user context, which is broken authentication / horizontal privilege escalation on a mounted route (`/api/v1/webhooks/voice/*` per `lib.rs:339-340`). The load-bearing fix is to validate the OAuth access token, extract the linked `user_id`, and scope the device lookup to `AND user_id = $1`; fail closed (`401 UNAUTHORIZED`) on invalid/absent token.

## Evidence

- `backend/servers/api-server/src/routes/voice_webhooks.rs:963-999` — `authenticate_voice_user()` signature `_access_token: &str` (underscore-prefixed → ignored) followed by the platform-only SELECT `SELECT * FROM voice_assistant_devices WHERE platform = $1 AND is_active = TRUE ORDER BY last_used_at DESC NULLS LAST LIMIT 1`
- `backend/servers/api-server/src/routes/voice_webhooks.rs:127` — `authenticate_voice_user(rls.conn(), token, voice_platform::ALEXA)` inside `alexa_webhook`; the returned device's owner drives the subsequent voice-command dispatch
- `backend/servers/api-server/src/routes/voice_webhooks.rs:243` — same call inside `google_actions_webhook`
- `backend/servers/api-server/src/lib.rs:340` — `routes::voice_webhooks::voice_webhook_router()` is mounted at `/api/v1/webhooks/voice`, so both live handlers are reachable in production
- GitHub issue #2658 — filed 2026-08-04 (labels: security, backend, follow-up), owner pm-security/pm-backend

## Files

- `backend/servers/api-server/src/routes/voice_webhooks.rs:963`
- `backend/servers/api-server/src/routes/voice_webhooks.rs:127`
- `backend/servers/api-server/src/routes/voice_webhooks.rs:243`

## Dependencies

<!-- No hard task dependencies. Sibling backlog rows (`code-review-api-handlers-voice-webhook-default-secret`, `code-review-api-handlers-voice-webhook-timing-cmp`) touch the same file and may be merged in either order; they are not prerequisites. -->

## Required capabilities

- [x] C1 — Systematic debugging (auth path involves OAuth session lookup)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps

1. Boot `api-server` with `VOICE_WEBHOOK_SECRET` set to any value.
2. Insert two `voice_assistant_devices` rows for the `alexa` platform belonging to distinct users A and B, both with `is_active=TRUE`. Update B's `last_used_at` to `now()` so B is the most recent.
3. Craft an Alexa Skills Kit request whose `session.user.access_token` is any non-empty string (a random 32-char UUID) — it does not need to be a valid OAuth token for either A or B.
4. POST it to `/api/v1/webhooks/voice/alexa` and observe the response. Expected: `401 UNAUTHORIZED` because the token is not tied to any user. Actual: the request is authenticated as B (the most-recently-used device), and the voice-command dispatch reads/writes B's tenant data on behalf of the anonymous caller.

## Suggested approach

1. In `voice_webhooks.rs`, drop the underscore from `_access_token` and change the helper's signature to take the token as `&str` (currently already `&str` — the change is semantic).
2. Add an OAuth-token validation step at the top of `authenticate_voice_user`: look up the token in the existing OAuth-linking table (`voice_oauth_tokens` or equivalent — grep `oauth_token_exchange` in the same file, line ~68, for the persistence path). If not found or expired, return `(StatusCode::UNAUTHORIZED, Json(ErrorResponse::new("INVALID_TOKEN", "Voice access token is invalid or expired")))`.
3. Extract the linked `user_id` from the validated token row and change the device SELECT to `WHERE platform = $1 AND is_active = TRUE AND user_id = $2 ORDER BY last_used_at DESC NULLS LAST LIMIT 1`. On zero rows, return `(StatusCode::UNAUTHORIZED, Json(ErrorResponse::new("DEVICE_NOT_LINKED", "No active voice device for this user")))`.
4. Callers at line 127 and 243 already pass `token` — no signature change on their side.
5. Update the module docstring at the top of `voice_webhooks.rs` to remove the "simplified platform lookup" caveat around this helper.
6. Add integration test coverage per the *Test plan* section.
7. `cd backend && cargo fmt -p api-server && cargo clippy -p api-server -- -D warnings && cargo test -p api-server routes::voice_webhooks`.

## Alternatives considered

- **Delete both Alexa/Google handlers** — rejected because voice-webhook receiver is Story 93.3 shipped surface area; removing it would regress product functionality just to close the auth hole.
- **Gate the router behind a feature flag** — rejected because the caller-facing endpoints are already mounted (reachable in prod), so the hole is exploitable today; a flag defers the fix without closing it.

## Root-cause trace

1. Symptom: unauthenticated Alexa/Google caller executes voice commands (check-balance, contact-manager, report-fault) as the most-recently-active device's owner
2. ← `alexa_webhook`/`google_actions_webhook` (voice_webhooks.rs:127, 243) trust the device returned by `authenticate_voice_user` and use its owner as the request principal
3. ← `authenticate_voice_user` (voice_webhooks.rs:963-999) never validates the token; the SQL selects by `platform` only, with `ORDER BY last_used_at DESC LIMIT 1` returning an unrelated user's device
4. Origin: initial voice-webhook implementation (Story 93.3). The in-source comments explicitly flag it as placeholder ("for now, find any active device", "in production, you would validate the token").

## Test plan

- [ ] `backend/servers/api-server/tests/voice_webhooks_auth.rs` (new) — integration test that seeds two devices for the same platform under distinct users, calls the helper with a bogus token, and asserts `401 UNAUTHORIZED` (before the fix this fails: the call currently returns the most-recent device instead)
- [ ] Extend `voice_webhooks.rs`'s in-file test module (`#[cfg(test)] mod tests`, starting near line 1163) with a unit case that instantiates the helper against a mocked `PgConnection` (or feature-gated in-memory stub) and confirms token-absent → error, valid-token-for-user-B → device belonging to B, valid-token-for-user-A → device belonging to A
- [ ] `cd backend && cargo test -p api-server routes::voice_webhooks::tests::test_authenticate_voice_user_rejects_unlinked_token`

## Out of scope

- The two sibling voice-webhook hardening items (`code-review-api-handlers-voice-webhook-default-secret`, `code-review-api-handlers-voice-webhook-timing-cmp`) — separate backlog rows; do not fold in.
- `verify_alexa_signature` / `verify_google_request` are also stubbed; leave them for a follow-up (they are the *signature*-side complement to this fix, not the auth-side).
- OAuth token-exchange endpoints (`/oauth/exchange`, `/oauth/refresh`) — this plan validates existing tokens; it does not restructure the token-issuance flow.

## After-merge

- Move this file to `plans/_archive/code-review-api-handlers-voice-webhook-token-authbypass.md`
- Mark the matching `backlog.json` row as `status: "done"`
