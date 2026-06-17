# security-voice-webhooks-hmac-bypass

**Vector:** security
**Score:** 3
**Source:** rotating-expert-review 2026-06-17 api-handlers
**Confidence:** medium

## Hypothesis
`voice_webhooks.rs:verify_hmac_signature` (line 948) compares the incoming base64 HMAC tag with the expected tag using raw `==` (line 959), and falls back to the literal string `"default_secret"` when `VOICE_WEBHOOK_SECRET` is unset (line 950). The `==` operator on strings short-circuits at the first byte mismatch, leaking a timing oracle that lets an attacker forge the tag byte-by-byte. The default-secret fallback turns the signature check into a no-op against a public, well-known key in any deployment that forgets to set the env var. Combined, the voice webhook signature gate is bypassable in practice. Switch to constant-time compare via `Mac::verify_slice` (matching `portal_webhooks.rs:112`, `integrations/sync.rs:172,187,203`) and fail-closed on missing secret (matching `install.rs:952`'s "refusing to store without encryption" pattern).

## Evidence
- `backend/servers/api-server/src/routes/voice_webhooks.rs:948-959` — `verify_hmac_signature` body. Line 950: `std::env::var("VOICE_WEBHOOK_SECRET").unwrap_or_else(|_| "default_secret".to_string())`. Line 957: `let expected = BASE64.encode(mac.finalize().into_bytes())`. Line 959: `Ok(signature == expected)`.
- `backend/servers/api-server/src/routes/portal_webhooks.rs:112` — `mac.verify_slice(&expected)` constant-time path used in this codebase.
- `backend/servers/api-server/src/routes/integrations/sync.rs:172,187,203` — three sibling webhook signature checks all use `mac.verify_slice(&signature_bytes).is_ok()`.
- `backend/servers/api-server/src/routes/signatures.rs:666` — alternative canonical pattern: `provided_b.len() == expected_b.len() && bool::from(provided_b.ct_eq(expected_b))` (subtle crate, ConstantTimeEq).
- `backend/servers/api-server/src/routes/integrations/install.rs:952` — fail-closed precedent: "Refusing to store … without encryption". Voice webhook should refuse to start (or refuse to verify) without the secret.

## Files
- `backend/servers/api-server/src/routes/voice_webhooks.rs`

## Dependencies
<!-- none -->

## Required capabilities
- [x] C1 — Systematic debugging (security class)
- [x] C6 — Verification before completion

Mode: cloud-ok

## Repro steps
1. Confirm the timing oracle with a manual sanity check (not a CI test — just to prove the issue): run the api-server with `VOICE_WEBHOOK_SECRET=test`. POST `/api/v1/webhooks/voice/verify` with body `{}` and `X-Signature: AAAA`. Measure response time. Vary the first byte across all 256 values; the response time differs (microseconds in release, but distinguishable). Constant-time compare flattens the curve.
2. Confirm the default-secret fallback: `unset VOICE_WEBHOOK_SECRET` and start the server. POST the same endpoint with a body and a signature computed with HMAC-SHA256 over the same body keyed on the literal string `"default_secret"`. Server accepts it.
3. Both are vulnerability proofs the fix must close: the constant-time compare flattens (1), and fail-closed on missing secret rejects (2).

## Suggested approach
1. **Replace the `==` compare with `Mac::verify_slice`.** In `voice_webhooks.rs:948-959`, after computing `mac`, decode the incoming `signature` from base64 to bytes and call `mac.verify_slice(&sig_bytes).is_ok()`. Mirror `portal_webhooks.rs:112` exactly. Drop the `BASE64.encode(...)` step on the expected side — `verify_slice` compares raw bytes, no encoding needed.
2. **Fail-closed on missing secret.** Change line 950 from `unwrap_or_else(|_| "default_secret".to_string())` to either:
   - `?` propagation with a top-of-function `let secret = std::env::var("VOICE_WEBHOOK_SECRET").map_err(|_| "VOICE_WEBHOOK_SECRET not set")?;`, returning `Err` from `verify_hmac_signature`; **or**
   - check the env var at handler-extractor time / app-state init, refusing to mount the route when unset (cleaner, mirrors `install.rs:952`).
   Pick (a) for minimal-diff PR (matches the caller's existing `match` arm at line 712). Add a startup-time `tracing::error!` log if the env var is missing so ops sees the misconfiguration in dev/staging.
3. **Add a unit test** at `voice_webhooks.rs` (in-file `#[cfg(test)] mod tests`) that:
   - Asserts `verify_hmac_signature` returns `Err` when `VOICE_WEBHOOK_SECRET` is unset (use `serial_test::serial` + `std::env::set_var`/`remove_var` if there's no DI seam — same pattern as existing env-var tests in the crate).
   - Asserts `verify_hmac_signature` returns `Ok(false)` (or matches the new contract) on a wrong-signature body, and `Ok(true)` on the right one.
4. **Grep regression guard.** Add `backend/servers/api-server/tests/security_webhook_signature_compare_guard.rs` that fails if anyone reintroduces `signature ==` or `tag ==` patterns near webhook verify functions — same shape as the existing rls-baseline scanners.
5. **Document the env var** in `backend/servers/api-server/README.md` (and/or `.env.example`) so the fail-closed behavior doesn't break local dev silently.

## Alternatives considered
- **Use `subtle::ConstantTimeEq::ct_eq` directly on the base64 strings** — rejected because comparing strings of variable length still leaks length, and the canonical Rust HMAC API (`Mac::verify_slice`) already does the right thing without the extra crate-import dance. `signatures.rs:666` is acceptable for a string compare but `verify_slice` is preferred for HMAC.
- **Keep the default secret but make it environment-conditional (only in dev)** — rejected because `verify_hmac_signature` is called at the same code path in all environments; dev-only default would either require a config flag (more state) or get committed and shipped (same bug). Fail-closed everywhere is the no-surprises path; dev sets the env var locally.

## Root-cause trace
1. Symptom: forged voice webhook payloads accepted by `/api/v1/webhooks/voice/verify` either via timing-oracle attack or via the well-known default secret.
2. ← `voice_webhooks.rs:959` raw `==` on base64 tag — short-circuits.
3. ← `voice_webhooks.rs:950` `unwrap_or_else(|_| "default_secret".to_string())` — turns a configuration error into a known-key signature.
4. Origin: `voice_webhooks.rs` was authored without referencing the in-repo prior art (`portal_webhooks.rs:112`, `integrations/sync.rs:172`). Likely an integration that was pasted from a stack-overflow snippet without the security review pass the other webhook integrations got. `git blame` `voice_webhooks.rs:948-959` to confirm.

## Test plan
- [ ] `backend/servers/api-server/src/routes/voice_webhooks.rs` in-file unit tests for `verify_hmac_signature` — Err on missing secret, Ok(false) on wrong sig, Ok(true) on right sig.
- [ ] `backend/servers/api-server/tests/security_webhook_signature_compare_guard.rs` — grep-based regression guard against `signature ==` near webhook verify fns.
- [ ] `cargo test -p api-server voice_webhooks` — exercises both new test paths.
- [ ] `cargo clippy -p api-server --all-targets -- -D warnings` — verify the rewrite compiles clean.

## Out of scope
- Migrating other webhook handlers that already use `verify_slice` (portal, airbnb, docusign, adobe, hellosign — confirmed in evidence). They're not the bug class.
- Restructuring the voice webhook route to fail-mount at app-state init time (cleaner but bigger PR). Track as a follow-up if (1) verify_hmac_signature lands clean.
- Rotating any keys leaked by past default-secret deployments — ops decision, not a code change.

## After-merge
- Move this file to `plans/_archive/security-voice-webhooks-hmac-bypass.md`
- Mark the matching `backlog.json` rows (`code-review-api-handlers-voice-hmac-timing` AND `code-review-api-handlers-voice-default-secret`) as `status: "done"` (both findings are resolved by this single PR — both share the same file and the same fix shape).
