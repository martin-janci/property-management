# code-review-api-handlers-voice-webhook-default-secret

**Vector:** security
**Score:** 3
**Source:** Issue #2658 (tier1d-dispatcher-generator, api-handlers segment, 2026-08-04)
**Confidence:** high

## Hypothesis

`verify_hmac_signature` in `backend/servers/api-server/src/routes/voice_webhooks.rs:948-960` reads `VOICE_WEBHOOK_SECRET` from the environment and falls back to the hardcoded, source-visible constant `"default_secret"` via `unwrap_or_else(|_| "default_secret".to_string())`. When the env var is unset, inbound HMAC-SHA256 signatures on `POST /api/v1/webhooks/voice/verify` are validated against that public constant, so anyone reading the repo can forge a passing signature. Every sibling receiver in this backend (`DOCUSIGN_WEBHOOK_SECRET`, `ADOBE_SIGN_WEBHOOK_SECRET`, `HELLOSIGN_WEBHOOK_SECRET` in `routes/integrations/webhook.rs:777`/`808`/`837`, plus `PORTAL_WEBHOOK_SECRET`/`AIRBNB_WEBHOOK_SECRET`/`STRIPE_WEBHOOK_SECRET` per `backend/CLAUDE.md`) fails closed with a `500 CONFIG_ERROR` when its secret is empty. Fix: fail closed on the same shape for voice.

## Evidence

- `backend/servers/api-server/src/routes/voice_webhooks.rs:948-960` — `let secret = std::env::var("VOICE_WEBHOOK_SECRET").unwrap_or_else(|_| "default_secret".to_string());` followed by HMAC-SHA256 mac construction and `Ok(signature == expected)`
- `backend/servers/api-server/src/routes/voice_webhooks.rs:712` — call site inside `verify_webhook_signature`, reachable via `POST /api/v1/webhooks/voice/verify` (`voice_webhook_router()` in the same file at line 60-)
- `backend/servers/api-server/src/routes/integrations/webhook.rs:777-787` — canonical fail-closed template: `let secret = std::env::var("DOCUSIGN_WEBHOOK_SECRET").unwrap_or_else(|_| String::new()); if secret.is_empty() { return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new("CONFIG_ERROR", "Webhook verification not configured")))); }`
- `backend/CLAUDE.md` — env-var table explicitly documents fail-closed behavior for the other webhook secrets; `VOICE_WEBHOOK_SECRET` is not documented there and diverges from the pattern
- GitHub issue #2658 (finding #2) — filed 2026-08-04

## Files

- `backend/servers/api-server/src/routes/voice_webhooks.rs:948`
- `backend/servers/api-server/src/routes/voice_webhooks.rs:712`

## Dependencies

<!-- No hard task dependencies. Sibling row `code-review-api-handlers-voice-webhook-token-authbypass` touches the same file but a different function (`authenticate_voice_user`), and the two changes are orthogonal. -->

## Required capabilities

- [ ] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps

1. Ensure `VOICE_WEBHOOK_SECRET` is **unset** in the shell that will spawn `api-server` (`unset VOICE_WEBHOOK_SECRET`).
2. Boot `cargo run -p api-server`.
3. Compute the HMAC-SHA256 of an arbitrary body `b` under the literal key `default_secret` (base64-encode): `echo -n "$b" | openssl dgst -sha256 -hmac default_secret -binary | base64`.
4. `POST /api/v1/webhooks/voice/verify` with `{"platform":"google","body":"<b>","signature":"<computed>"}`. Expected: `500 CONFIG_ERROR` (secret not configured). Actual: `200 {"valid": true, "platform": "google"}` — a forged signature passes verification.

## Suggested approach

1. In `voice_webhooks.rs:948`, replace the `unwrap_or_else` with the fail-closed pattern from `routes/integrations/webhook.rs:777-787`. Change the helper's return contract from `Result<bool, String>` to lift the empty-secret case into a distinct error surface — either an enum variant (`enum VoiceWebhookVerifyError { NotConfigured, InvalidKey(String), … }`) or a boxed error carrying a stable code that the caller at line 712 maps to `500 CONFIG_ERROR`.
2. At the call site (line 712 inside `verify_webhook_signature`), match the new error and return a `WebhookVerificationResult { valid: false, platform, error: Some("Voice webhook verification not configured".into()) }` — but for the *HTTP* response, prefer `500 CONFIG_ERROR` if the caller is the mounted `/verify` endpoint (mirroring DocuSign). Check the endpoint's response type at `voice_webhooks.rs:71` and decide where the 500 branch lives without leaking internal detail.
3. Add the `VOICE_WEBHOOK_SECRET` row to `backend/CLAUDE.md`'s env-var table with the same wording as `PORTAL_WEBHOOK_SECRET`: `HMAC secret for the voice-webhook /verify endpoint; fails closed (500 CONFIG_ERROR) when unset.`
4. Grep the repo for the literal `"default_secret"` and delete any residue: `grep -rn '"default_secret"' backend/` → expect 0 hits after the fix.
5. `cd backend && cargo fmt -p api-server && cargo clippy -p api-server -- -D warnings && cargo test -p api-server routes::voice_webhooks`.

## Alternatives considered

- **Ship a randomly-generated default in-process** — rejected because a pod-local random secret means each pod verifies against a different key, so a horizontally-scaled deployment silently accepts and rejects the same signature depending on which pod handles the request. Fail-closed is the operational-parity choice.
- **Fail closed only when `default_secret` is the literal value at compile time** — rejected because it depends on a build-time constant matching a runtime env-var, which is fragile (secret rotation, staging vs. prod). The env-var-unset check at request time is what the sibling receivers do.

## Root-cause trace

N/A — security doesn't need backward tracing beyond the evidence above.

## Test plan

- [ ] `backend/servers/api-server/tests/voice_webhooks_default_secret.rs` (new) — integration test that unsets `VOICE_WEBHOOK_SECRET` at test entry, boots the router, `POST`s a signature computed under `default_secret`, and asserts `500 CONFIG_ERROR` (before the fix this fails: the request returns `200 {"valid": true}`)
- [ ] Extend the existing in-file test module at `voice_webhooks.rs:1388-1401` — the current tests assert HMAC good/bad against a set secret; add a case that clears `VOICE_WEBHOOK_SECRET` and asserts the helper returns the new NotConfigured error variant
- [ ] `cd backend && cargo test -p api-server routes::voice_webhooks::tests::test_verify_hmac_fails_closed_when_secret_unset`

## Out of scope

- The other two voice-webhook findings (`token-authbypass`, `timing-cmp`) are separate backlog rows and separate plans (one already promoted; timing-cmp waits for the next routine run).
- Rotating or re-issuing an existing prod `VOICE_WEBHOOK_SECRET` — an operational change, not a code change.
- Adding replay protection (`X-Webhook-Timestamp` window) to the voice endpoint — separate hardening, follow the `PORTAL_WEBHOOK_SECRET` pattern in a follow-up plan.

## After-merge

- Move this file to `plans/_archive/code-review-api-handlers-voice-webhook-default-secret.md`
- Mark the matching `backlog.json` row as `status: "done"`
