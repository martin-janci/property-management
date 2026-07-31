# security-voice-webhooks-signature-hardening

**Vector:** security
**Score:** 7
**Source:** hotspot in backend/servers/api-server/src/routes/voice_webhooks.rs
**Confidence:** high

## Hypothesis

Two independent signature-verification paths in `voice_webhooks.rs` are effectively fail-open. `verify_alexa_signature` (`voice_webhooks.rs:750-784`) extracts the `Signature` header into `_signature` and never uses it — only the cert-URL format and the body timestamp are validated. `verify_hmac_signature` (`voice_webhooks.rs:948-960`) falls back to a hardcoded `"default_secret"` when `VOICE_WEBHOOK_SECRET` is unset (every other webhook family in this repo — portal, stripe, airbnb — fails closed on missing secret; see `backend/CLAUDE.md`) and uses `signature == expected` (plain `String` equality, not `subtle::ConstantTimeEq`). Combined with the separate auth bypass (`plans/security-voice-webhooks-auth-bypass.md`), any anonymous caller can craft a request that reaches tenant data. The smallest change: make both paths fail closed on missing secret, real-verify Alexa signatures via x509 chain validation, and use constant-time comparison for the MAC.

## Evidence

- `backend/servers/api-server/src/routes/voice_webhooks.rs:750-784` — `verify_alexa_signature` binds `_signature` (unused) and returns Ok after only `validate_alexa_cert_url` + `validate_alexa_timestamp`. In-code comment lines 768-776 explicitly document the missing steps 3 (cert-chain fetch) & 4 (signature-verify).
- `backend/servers/api-server/src/routes/voice_webhooks.rs:948-960` — `verify_hmac_signature` uses `std::env::var("VOICE_WEBHOOK_SECRET").unwrap_or_else(|_| "default_secret".to_string())`; line 959 does `signature == expected` (non-constant-time).
- `backend/CLAUDE.md` — the ENV-var table documents `PORTAL_WEBHOOK_SECRET`, `AIRBNB_WEBHOOK_SECRET`, `STRIPE_WEBHOOK_SECRET`, and the `REALITY_PORTAL_WEBHOOK_SECRET` family all "fail closed" on missing secret; voice is the outlier.
- Hardened webhook pattern to copy: `backend/servers/api-server/src/routes/portal_webhooks.rs::verify_timestamped_portal_webhook` and `backend/servers/api-server/src/routes/layout/webhook.rs::sign_timestamped_payload` (both use HMAC-SHA256 over `"{timestamp}.{body}"` with a ±300s replay window).
- Regression-test evidence from PR #2604 (merged 2026-07-31): the unit test `alexa_signature_accepts_valid_url_and_timestamp` at `voice_webhooks.rs:1336` literally passes `signature: "sig"` and expects `Ok` — pinning the bypass into the test suite. Any real fix must delete/replace that test.

## Files

- `backend/servers/api-server/src/routes/voice_webhooks.rs:750`
- `backend/servers/api-server/src/routes/voice_webhooks.rs:948`
- `backend/servers/api-server/src/routes/portal_webhooks.rs`
- `backend/servers/api-server/src/routes/layout/webhook.rs`
- `backend/Cargo.toml`

## Dependencies

## Required capabilities

- [x] C1 — Systematic debugging (security fix; signature verification is easy to get subtly wrong)
- [ ] C2 — Seed data (unit tests only — no DB fixtures needed for signature-verify paths)
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception

Mode: cloud-ok

## Repro steps

1. Unset `VOICE_WEBHOOK_SECRET` (or start api-server with it unset).
2. POST `/api/v1/webhooks/voice/google` with an authorization header (Bearer-style token) whose HMAC is computed with the literal secret `"default_secret"`.
3. Expected: 500 `CONFIG_ERROR` (fail-closed, matching portal/stripe/airbnb).
4. Actual (dev): the request is accepted as authentic.
5. Separately, for Alexa: POST `/api/v1/webhooks/voice/alexa` with `SignatureCertChainUrl: https://s3.amazonaws.com/echo.api/x`, `Signature: sig` (literal three-letter string), and a body timestamp inside 150s. Expected: 401 `INVALID_SIGNATURE`. Actual: 200 OK.

## Suggested approach

1. In `verify_hmac_signature` (`voice_webhooks.rs:948`): replace `unwrap_or_else(|_| "default_secret".to_string())` with `map_err(|_| "VOICE_WEBHOOK_SECRET not configured".to_string())?`. Return `Err` so the caller emits 500 `CONFIG_ERROR` (mirroring `portal_webhooks.rs` behavior). Replace `signature == expected` with `subtle::ConstantTimeEq::ct_eq` (already a transitive dep of `hmac` in the workspace — verify in `backend/Cargo.toml` / `Cargo.lock`) or `constant_time_eq::constant_time_eq`.
2. In `verify_alexa_signature` (`voice_webhooks.rs:750`): drop the `_` prefix on `signature`, fetch the cert chain via `reqwest::Client` (with a small LRU cache keyed on `cert_url`, TTL 10 min — Amazon rotates infrequently), validate the chain up to the Amazon SigningKey root using `x509-parser` + `rustls-webpki` (both already in the workspace tree — grep `x509-parser` / `webpki` in `Cargo.lock` first; add if absent), then verify the signature over the raw body bytes using the cert's public key. If any step fails, return `Err`.
3. Fix the `/verify` helper (`voice_webhooks.rs:698`) so `valid` reflects `verify_alexa_signature(...).is_ok()` for `alexa` and `verify_hmac_signature(...).is_ok()` for `google`.
4. Delete or rewrite the two PR #2604 tests that pinned the broken behavior: `alexa_signature_accepts_valid_url_and_timestamp` (line 1336) and any `hmac_verify_*` test that assumes `default_secret` as the fallback.
5. Add positive + negative unit tests: (a) HMAC unset → `Err`, (b) HMAC set + valid signature → `Ok`, (c) HMAC set + invalid signature → `Err`, (d) Alexa forged signature over a legit cert chain → `Err`, (e) Alexa mismatched cert chain (self-signed) → `Err`.
6. Update `backend/CLAUDE.md`'s ENV-var table to list `VOICE_WEBHOOK_SECRET` alongside the other webhook secrets ("fails closed when unset").

## Alternatives considered

- **Delete the voice-webhook endpoints entirely** — rejected because there is an evident OAuth linking flow (`oauth_token_exchange`, `oauth_token_refresh` in the same file) and infrastructure to build on; sunsetting the surface after real customers have linked skills would break flows without a migration path.
- **Move signature verification into a shared `axum` middleware and gate the whole `voice_webhook_router()` on it** — rejected for this plan because the HMAC vs Alexa cert-chain paths are asymmetric (different key material, different header schema); a middleware layer would need to branch on route anyway. Leave as a follow-up refactor once both paths are correct in isolation.

## Root-cause trace

1. Symptom: forged Alexa deliveries and Google Actions calls with a source-code-known HMAC secret are accepted as authentic; downstream handlers then act on tenant data.
2. ← `verify_alexa_signature` (`voice_webhooks.rs:750`) validates only URL format + timestamp; the cryptographic signature is deliberately unread (`_signature` prefix).
3. ← `verify_hmac_signature` (`voice_webhooks.rs:948`) falls back to a hardcoded secret; there is no fail-closed path when the operator forgets to configure `VOICE_WEBHOOK_SECRET`.
4. Origin: original file authorship — the in-code comments admit "For now, we validate URL format and timestamp which catches most issues" (Alexa, line 775) and no fail-closed handling for HMAC. PR #2604 (merged 2026-07-31) added tests that codified this behavior, making a future fix a test-refactor + code-fix.

## Test plan

- [ ] Extend the unit-test block at the bottom of `backend/servers/api-server/src/routes/voice_webhooks.rs` with fail-closed + constant-time cases described above (must fail on `main`).
- [ ] Add an integration test at `backend/servers/api-server/tests/suites/voice_webhooks_signature_tests.rs` covering: HMAC missing-secret returns 500, forged Alexa signature returns 401, matched signature returns 200.
- [ ] `cargo test -p api-server voice_webhooks` locally before pushing.

## Out of scope

- The `authenticate_voice_user` cross-tenant bypass — covered by `plans/security-voice-webhooks-auth-bypass.md` (must land first or in parallel; both are required to close the exploit chain).
- Rotating the workspace-transitive `subtle` / `constant_time_eq` dep on major-version bumps — dep-update noise.
- Alexa cert-chain caching beyond a simple LRU (a proper cache-eviction policy is a follow-up if this becomes a hot path).
- Ridding the file of remaining `unwrap_or_default()` calls in unrelated response builders.

## After-merge

- Move this file to `plans/_archive/security-voice-webhooks-signature-hardening.md`
- Mark backlog rows `security-voice-webhook-alexa-signature-not-verified` and `code-review-api-handlers-voice-webhooks-hmac-default` as `status: "done"`
- Operator: ensure `VOICE_WEBHOOK_SECRET` is set in all production environments before deploying (fail-closed will otherwise 500 all Google Actions requests); coordinate with Alexa Skill config for cert-chain URL if it changes.
