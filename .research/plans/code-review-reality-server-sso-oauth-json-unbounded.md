# code-review-reality-server-sso-oauth-json-unbounded

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review 2026-11-09 reality-server (dispatcher Tier-1d-equivalent in Phase 1.5)
**Confidence:** high

## Hypothesis
`reality-server/src/routes/sso.rs:763` and `:780` deserialise the PM OAuth token-exchange and user-info responses with a bare `response.json().await?`, with no response-body cap and no structured validation. A compromised, misconfigured, or slow PM OAuth endpoint can make reality-server (a) consume unbounded memory while serde tries to buffer a malicious response, or (b) hand a partially-populated `TokenResponse` / `SsoUserInfo` to `claim_or_create_portal_user_from_pm`, mutating portal user records with incomplete data. The team just landed the same fix pattern on the reality-web side in PR #3350 (`parseListingSummary` + bounded `bytes().await?` + `safeJson`): mirror that shape here with a `Content-Length`-aware reader capped to a small size (e.g. 1 MiB) and then `serde_json::from_slice::<T>`.

## Evidence
- `backend/servers/reality-server/src/routes/sso.rs:763` — `Ok(response.json().await?)` in `exchange_code_for_tokens` (verified by Read 2026-11-09)
- `backend/servers/reality-server/src/routes/sso.rs:780` — same pattern in `get_user_info` returning `SsoUserInfo`
- PR #3350 (merged 2026-11-07) — reality-web shared-comparison body now parsed via `parseListingSummary(listingSummarySchema)` as the just-landed reference pattern for unchecked JSON cast at a trust boundary
- `backend/servers/reality-server/src/routes/sso.rs:758-761` — status is checked but `.text()` / `.json()` consume the body with no size limit
- Related finding: `code-review-mobile-native-kmp-http-client-no-timeout` (open backlog row) — complementary surface where unbounded upstream responses hang clients

## Files
- `backend/servers/reality-server/src/routes/sso.rs:763`
- `backend/servers/reality-server/src/routes/sso.rs:780`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**
- No C4 or C5 ticked → `cloud-ok`

Mode: cloud-ok

## Repro steps
1. Start a mock upstream that returns a 200 with a 100 MiB response body on the PM `/token` endpoint (or a slow-drip body that never completes).
2. Point `pm_token_url` at the mock and drive a reality-server `/api/v1/portal/auth/sso/callback` flow with a fresh code.
3. Expected (after fix): reality-server rejects the response with a `SsoError::BadUpstream` (or equivalent) and returns 502 within the cap; memory usage stays bounded.
4. Actual (today): reality-server allocates megabytes trying to buffer the body into serde; the request hangs (no `HttpTimeout`-style ceiling), and under memory pressure the process can OOM.

## Suggested approach
1. Add a small helper in `reality-server/src/routes/sso.rs` (or `util/http.rs`): `async fn read_bounded_json<T: DeserializeOwned>(resp: Response, max_bytes: usize) -> Result<T, anyhow::Error>` that checks `Content-Length` (reject > `max_bytes`), falls back to `bytes().await?` with a `.take(max_bytes + 1)` guard, then `serde_json::from_slice`.
2. Replace `Ok(response.json().await?)` at `sso.rs:763` with `read_bounded_json::<TokenResponse>(response, 1024 * 1024).await`.
3. Same at `sso.rs:780` for `get_user_info` → `SsoUserInfo`.
4. Add explicit `anyhow!("Upstream response exceeded {max_bytes} bytes")` error → maps to the existing `SsoError` 502 path.
5. Audit `backend/servers/reality-server/src/routes/sso.rs:788-805` (`introspect_pm_token`) — same pattern may lurk inside `introspect_pm_token_inner`.
6. Write a unit test that hands the helper a fake `Response` with an oversized body and asserts the bounded error is returned without full buffering.
7. Add a sqlx-free integration test driving the SSO callback against a wiremock upstream that serves 2 MiB of junk.

## Alternatives considered
- **Install `HttpTimeout` on `pm_oauth_client`** — rejected because a request-level timeout bounds the duration but NOT the memory footprint of an attacker-controlled response body; a fast malicious upstream can still send 100 MiB inside the timeout.
- **Wrap the whole SSO callback in a tower `Limit` layer** — rejected because it would throttle legitimate concurrent callbacks without addressing the per-response memory cap; the issue is response-body size, not request rate.

## Root-cause trace
1. Symptom: reality-server allocates unbounded memory (or hands a partial struct downstream) on a misbehaving PM OAuth response.
2. ← `sso.rs:763` `response.json().await?` — serde fully buffers the body with no cap.
3. ← `sso.rs:755` `pm_oauth_client.post(...).send().await?` — reqwest client has no decoded-body-size limit.
4. ← `state.rs` (or wherever the client is built) — `pm_oauth_client` constructed as a plain `reqwest::Client` without a response-body limit extension.
5. Origin: introduced when the PM OAuth client was first added; predates the reality-portal hardening sweep that produced PR #3350 for reality-web.

## Test plan
- [ ] `backend/servers/reality-server/src/routes/sso_tests.rs` (new) — unit test `read_bounded_json_rejects_oversized_body`: feeds a 2 MiB body, asserts err.
- [ ] `backend/servers/reality-server/tests/` — integration test `sso_callback_rejects_oversized_token_response`: wiremock PM upstream serves 2 MiB JSON; reality-server returns 502 within 1s, memory stable.
- [ ] `cargo test -p reality-server sso`
- [ ] `cargo clippy -p reality-server -- -D warnings`

## Out of scope
- Introducing full request/body timeouts on the PM OAuth client (follow-up plan: `code-review-reality-server-pm-oauth-timeouts`).
- Hardening the mobile-native KMP side (tracked by `code-review-mobile-native-kmp-sso-error-body-unguarded` and `-http-client-no-timeout`).
- Changing the PM OAuth protocol itself.

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-sso-oauth-json-unbounded.md`
- Mark the matching `backlog.json` row as `status: "done"`
