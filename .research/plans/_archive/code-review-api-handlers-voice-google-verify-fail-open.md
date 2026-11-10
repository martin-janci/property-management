# code-review-api-handlers-voice-google-verify-fail-open

**Vector:** security
**Score:** 2
**Source:** Phase 1.5 dev-review 2026-08-15 (segment=api-handlers, churn-aligned voice_webhooks.rs +72 lines after PR #2748)
**Confidence:** high

## Hypothesis
`verify_google_request` at `voice_webhooks.rs:1087` is fail-open: every branch except two format-errors returns `Ok(())`, and the "verified" branch is only a substring match of the project id inside the base64-decoded JWT payload. Signature, `iss`, `aud`, `exp`, and `nbf` are never checked; the doc comment even lists the missing JWKS verify steps. Additionally, `BASE64` is the STANDARD (padded) alphabet, so legitimate URL-safe unpadded JWT payloads silently fail to decode and the caller still passes. The only real gate on `POST /google` is the OAuth access-token check in `authenticate_voice_user`; the "signature verification" is defense-in-depth theatre and needs either a real implementation or explicit removal so the return value stops implying a verification that isn't happening.

## Evidence
- `backend/servers/api-server/src/routes/voice_webhooks.rs:1087` — `fn verify_google_request(...) -> Result<(), String>` returns `Ok(())` for: no project id configured, missing Authorization header, JWT-shaped bearer whose base64 payload merely `contains(&expected_project)`, AND whose payload fails to base64-decode.
- Doc comment above the function acknowledges the missing steps: *"In production, you would: 1. Decode the JWT header ... 2. Fetch Google's public keys ... 3. Verify the signature ..."*.
- Base64 alphabet mismatch: `use base64::engine::general_purpose::STANDARD as BASE64;` at the file's top — JWT payloads are URL-safe unpadded (`URL_SAFE_NO_PAD`), so any real Google-signed JWT with `-`/`_`/no-`=` in the payload silently `Err`s the decode and falls through to `Ok(())`.
- Caller path: `google_webhook` handler → `verify_google_request` → returns success even when nothing was verified. Only `authenticate_voice_user` (OAuth access-token) actually gates access.
- Sibling contrast: `verify_hmac_signature` (same file, used for Alexa on PR #2748) fails closed via `std::env::var(...).map_err(...)?` — the pattern is already established in-tree.

## Files
- `backend/servers/api-server/src/routes/voice_webhooks.rs:1087`

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
1. Point a test against the `google_webhook` handler with `GOOGLE_ACTIONS_PROJECT_ID=foo` set.
2. Send a request with a bearer header whose token is `AAA.<base64-of-'evil-project foo evil'>.CCC` — a JWT-shaped string whose middle segment base64-decodes to a payload merely containing the substring `foo` (not actually signed by Google, no valid `iss`/`aud`/`exp`).
3. Expected: `401` — signature fails to verify. Actual: `verify_google_request` returns `Ok(())` and the handler proceeds. Repeat with a URL-safe unpadded JWT that fails to decode under STANDARD alphabet → also passes.

## Suggested approach
1. Decide the target posture with the tech lead: (A) full JWT verify via `jsonwebtoken` crate with JWKS fetch + RS256/ES256 + `iss`/`aud`/`exp`/`nbf`, or (B) remove `verify_google_request` and rely explicitly on `authenticate_voice_user`'s OAuth access-token gate, adding a comment naming the gate.
2. If (A): add `jsonwebtoken` to `backend/servers/api-server/Cargo.toml` (workspace already uses it for the auth service). Fetch JWKS from `https://www.googleapis.com/oauth2/v3/certs` behind a `OnceLock<reqwest::Client>` with a 5s timeout and a keys cache with 1h TTL. Verify with `DecodingKey::from_rsa_components` + `Validation` set for `RS256`, expected `iss=https://accounts.google.com`, `aud=<GOOGLE_ACTIONS_PROJECT_ID>`, non-zero leeway on `exp/nbf`. Also switch the base64 decode of `parts[1]` to `URL_SAFE_NO_PAD` so the payload actually decodes for real JWTs.
3. If (B): delete `verify_google_request` and its callsites; rename the remaining check to `authenticate_google_bearer` (or similar) in `authenticate_voice_user` to make the gate explicit; add a `// SECURITY: this endpoint is gated solely by the OAuth access-token check — no JWT signature verify` comment.
4. Add regression tests in `backend/servers/api-server/tests/suites/voice_webhooks_tests.rs`:
   - Forged unsigned bearer whose payload merely contains the project id → asserts rejection (401 for A; still rejected via missing OAuth token for B).
   - URL-safe unpadded JWT with valid signature (path A only) → asserts 200.
   - Missing `GOOGLE_ACTIONS_PROJECT_ID` env → asserts fail-closed (currently returns Ok — that's the escape hatch we're removing).
5. Run `cargo test -p api-server voice_webhooks`; the forged-bearer test must fail on current `dev` (IG3).
6. Update the utoipa doc for `POST /api/v1/webhooks/voice/google` to accurately describe how requests are authenticated after the change.

## Alternatives considered
- **Keep the substring match, but tighten to a full-word match** — rejected because the whole primitive is wrong (no signature verification); a "better" substring check is still trivially forgeable and still asserts a verification that hasn't happened.
- **Fetch JWKS on every request (no cache)** — rejected because it turns every webhook call into a round-trip to Google, adds a latency spike per handler invocation, and creates a Google-availability dependency for a hot path; the OnceLock + 1h TTL is the standard shape.

## Root-cause trace
1. Symptom: a forged bearer whose base64-decoded payload merely contains the configured project id is accepted by `verify_google_request`, which returns `Ok(())` even though no signature was verified.
2. ← `verify_google_request` (`voice_webhooks.rs:1087`) treats a `payload_str.contains(&expected_project)` match as verification and falls through to `Ok(())` for every non-format-error path.
3. ← `google_webhook` handler proceeds because its only remaining gate is `authenticate_voice_user`'s OAuth access-token check; the "signature verify" step contributes no security beyond that.
4. Origin: the handler was landed as a placeholder ("In production, you would...") and never followed up with the real JWT verify implementation; `git blame verify_google_request` should pin the introducing commit — the doc comment is the smoking gun.

## Test plan
- [ ] `backend/servers/api-server/tests/suites/voice_webhooks_tests.rs::google_verify_rejects_forged_bearer` — asserts a bearer whose payload merely contains the project id is rejected.
- [ ] `voice_webhooks_tests.rs::google_verify_rejects_url_safe_unpadded_jwt_when_no_verify` — asserts current behavior (STANDARD alphabet mis-decodes real JWTs) is fixed regardless of chosen posture.
- [ ] `voice_webhooks_tests.rs::google_verify_fails_closed_when_project_id_unset` — asserts missing env fails closed.
- [ ] `cargo test -p api-server voice_webhooks`

## Out of scope
- Overhauling the Alexa verify path (already hardened in PR #2748).
- Changing the OAuth-refresh endpoint's auth (tracked in `code-review-api-handlers-voice-oauth-refresh-unauthenticated`).
- Introducing a new JWT-verify wrapper crate — reuse `jsonwebtoken` if already in workspace.

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-voice-google-verify-fail-open.md`
- Mark the matching `backlog.json` row as `status: "done"`
