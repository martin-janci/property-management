# security-voice-webhooks-auth-bypass

**Vector:** security
**Score:** 3
**Source:** hotspot in backend/servers/api-server/src/routes/voice_webhooks.rs
**Confidence:** high

## Hypothesis

`authenticate_voice_user` at `voice_webhooks.rs:963-1008` binds the OAuth access token to `_access_token` (unused) and returns whichever tenant's `voice_assistant_devices` row was most recently active for the platform. Both `alexa_webhook` (line 127) and `google_actions_webhook` (line 243) act on the returned `device.id`, so every authenticated-looking voice webhook — with any non-empty bearer/access token string — executes voice commands (balance checks, fault reports) against a foreign tenant's data. This is a P0 cross-tenant impersonation that predates the tests added by PR #2604 (which pinned the current broken behavior). The smallest change that resolves it: wire real token validation (look the device up by the presented token's device_id / hashed token column), and return 401 when the token doesn't match a device.

## Evidence

- `backend/servers/api-server/src/routes/voice_webhooks.rs:963-1008` — the SQL is `SELECT * FROM voice_assistant_devices WHERE platform = $1 AND is_active = TRUE ORDER BY last_used_at DESC NULLS LAST LIMIT 1`; the in-code comment on line 972 admits "for demo, just find any active device for the platform".
- `backend/servers/api-server/src/routes/voice_webhooks.rs:127` — `alexa_webhook` calls `authenticate_voice_user(rls.conn(), token, voice_platform::ALEXA)` and uses the returned `device.id` to run commands via `VoiceCommandProcessor::process_command`.
- `backend/servers/api-server/src/routes/voice_webhooks.rs:243` — `google_actions_webhook` mirrors the same call site for Google Assistant.
- The regression tests in PR #2604 (merged 2026-07-31) cover the branch logic (`intent → command`, response builders, cert-URL/timestamp validation) but never assert cross-tenant isolation — so they cannot catch this bypass.
- Related surface: the `/verify` helper at `voice_webhooks.rs:698` reports auth status as `valid: !request.signature.is_empty()` — a self-described "simplified check for demo" that leaks the same design choice.

## Files

- `backend/servers/api-server/src/routes/voice_webhooks.rs:963`
- `backend/servers/api-server/src/routes/voice_webhooks.rs:127`
- `backend/servers/api-server/src/routes/voice_webhooks.rs:243`
- `backend/crates/db/src/repositories`
- `backend/servers/api-server/src/routes/voice_webhooks.rs:698`

## Dependencies

## Required capabilities

- [x] C1 — Systematic debugging (security fix — tenant boundary regression risk)
- [x] C2 — Seed data (need multi-tenant device rows to write the failing-then-passing test)
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception

Mode: cloud-ok

## Repro steps

1. Seed org A with a `voice_assistant_devices` row (`platform='alexa'`, `is_active=true`, `access_token='TOKEN_A'`, `user_id` in org A).
2. Seed org B with a `voice_assistant_devices` row for the same platform but active more recently than org A's row (`is_active=true`, `access_token='TOKEN_B'`, `last_used_at = now`).
3. POST `/api/v1/webhooks/voice/alexa` with an Alexa `IntentRequest` body whose `session.user.access_token = 'TOKEN_A'` and a `SignatureCertChainUrl` under `s3.amazonaws.com/echo.api/…` (any string), a `Signature` header (any non-empty value), and a body-timestamp inside the ±150s window.
4. Expected: 401 Unauthorized OR the command is executed against org A's device.
5. Actual (dev): 200 OK — the command runs against org B's device because `authenticate_voice_user` returns the most-recently-active device regardless of the presented token.

## Suggested approach

1. Add a `access_token_hash TEXT UNIQUE` column (or repurpose the existing `access_token` column if it is already stored as a bcrypt/argon2 hash — inspect `backend/crates/db/src/repositories/voice_assistant_devices.rs` and any migration under `backend/migrations/` that created the table before choosing). Migration goes in a new `backend/migrations/NNNNN_voice_device_token_hash.sql` if the column doesn't exist.
2. In `authenticate_voice_user` (`voice_webhooks.rs:963`), remove the `_access_token` underscore, add a repository method (e.g. `VoiceAssistantDeviceRepository::find_active_by_token_hash(platform, token_hash) -> Option<VoiceAssistantDevice>`), and use it — dropping the `ORDER BY last_used_at` fallback entirely.
3. Return `401 DEVICE_NOT_LINKED` (existing error variant, line 1000-1006) when the repository returns `None`.
4. Update `/verify` (`voice_webhooks.rs:698`) so `valid` reflects a real token match, not `!signature.is_empty()`.
5. Update the PR #2604 tests that inadvertently pinned the bypass — rename or delete any test whose fixture leans on "single-device-per-platform" as an implicit auth guarantee.
6. Add the regression test from *Repro steps* — a `#[sqlx::test]` under `backend/servers/api-server/tests/suites/` that seeds two tenants and asserts the wrong-token path returns 401 (must fail on `main`).
7. Rotate any real production tokens after merge (operator step, called out in *After-merge*).

## Alternatives considered

- **Sunset the endpoint by returning 501 until real OAuth linking ships** — rejected because there are 6 mounted endpoints (Alexa webhook, Google webhook, OAuth token exchange, OAuth token refresh, /verify, health check) and the OAuth infrastructure appears to exist (`oauth_token_exchange`, `oauth_token_refresh` handlers). The single-line demo shortcut is the surgical fix; deleting the surface would drop working OAuth token flows.
- **Add a per-tenant path prefix (e.g. `/api/v1/webhooks/voice/{org_id}/alexa`) and require Alexa/Google skill config to encode the tenant** — rejected because it doesn't fix the underlying token-is-ignored bug (a caller who guesses another org's `org_id` still succeeds) and demands third-party skill-config changes for every tenant onboarding.

## Root-cause trace

1. Symptom: any bearer/access token succeeds at Alexa/Google voice webhooks and executes commands against a foreign tenant's data.
2. ← `authenticate_voice_user` at `voice_webhooks.rs:963-1008` binds the token to `_access_token` (unused) and issues a token-blind SQL query.
3. ← `alexa_webhook` (`voice_webhooks.rs:127`) and `google_actions_webhook` (`voice_webhooks.rs:243`) trust the returned `device.id` as the authenticated user boundary.
4. Origin: pre-existing since the file was first added (predates the git window this routine observes). The in-code comment "for demo, just find any active device for the platform" (line 978) confirms it was a placeholder that was never replaced. PR #2604 (merged 2026-07-31) added tests around the surface but did not touch this function.

## Test plan

- [ ] `backend/servers/api-server/tests/suites/voice_webhooks_auth_tests.rs` — a new `#[sqlx::test]` seeding two tenants with distinct active devices, asserting the wrong-token POST returns 401 (must fail on `main`).
- [ ] Update or replace any assertion in `backend/servers/api-server/src/routes/voice_webhooks.rs` unit tests that reads the current single-device fixture as authoritative.
- [ ] `cargo test -p api-server voice_webhooks` locally before pushing.

## Out of scope

- Full x509 signature-chain validation for Alexa (covered separately by `plans/security-voice-webhooks-signature-hardening.md`).
- HMAC secret hardening for the Google Actions path (also in the signature-hardening plan).
- The `/verify` endpoint's other simplifications beyond the auth check (documented as "for demo" in the file — separate cleanup).
- Any Voice-Assistant device linking UX changes (separate feature).

## After-merge

- Move this file to `plans/_archive/security-voice-webhooks-auth-bypass.md`
- Mark the matching `backlog.json` row (`code-review-api-handlers-voice-webhooks-auth-bypass`) as `status: "done"`
- Operator: rotate any live voice-assistant OAuth tokens; audit `voice_assistant_devices.last_used_at` for suspicious cross-tenant activity in the window this bypass existed.
