# code-review-api-handlers-share-pw-no-throttle

**Vector:** security
**Score:** 2
**Source:** Phase 1.5 rotating expert review 2026-08-13 (api-handlers segment — signals/2026-08-13-api-handlers-tier1d-2.json)
**Confidence:** high

## Hypothesis
The public `POST /api/v1/documents/shared/{token}/access` handler (`access_protected_share`) verifies a caller-supplied share password against `verify_share_password_for_share` with no throttle: no per-token counter, no per-IP counter, no exponential backoff. The only rate limiter wired in `api-server` is the per-tenant `TenantRateLimiterSet` applied through `host_tenant_middleware`, and the handler's own docstring notes "this public endpoint has no caller org context" — meaning the tenant limiter cannot key these unauthenticated requests. An attacker with a valid share token can brute-force the share password at request speed. Smallest fix: reuse the existing `rate_limit_allowed` / `InProcessRateLimiter` helper (already used by `forgot-password`, `resend-verification`, and `mfa`) at the top of `access_protected_share`, keyed on the share token (primary) with a secondary IP bucket, returning HTTP 429 with a generic body on limit-exceeded.

## Evidence
- `backend/servers/api-server/src/routes/documents/shares.rs:574` — `access_protected_share` signature: `State<AppState>`, `ConnectInfo<SocketAddr>`, `Path<String>`, `Json<AccessShareRequest>`; no rate-limit call before the password verification at line 618.
- `backend/servers/api-server/src/routes/documents/shares.rs:27` — `pub fn public_router()` mounts the endpoint at `POST /shared/{token}/access`; `backend/servers/api-server/src/lib.rs:165` merges `routes::documents::public_router()` into the app without any additional layer.
- Grep `rate.?limit|throttle|attempt|lockout` across `backend/servers/api-server/src/routes/documents/` returns **0 hits** — the entire documents subtree has no throttle machinery.
- The reusable helper already exists: `backend/servers/api-server/src/routes/rate_limit.rs:43` (`rate_limit_allowed<K: Eq + Hash>`), used by `routes/auth/mod.rs:60–95` (email-keyed) and `routes/mfa.rs:32–90` (Uuid-keyed). A share-token-keyed `InProcessRateLimiter<String>` is a drop-in third caller.
- Handler comment at `shares.rs:582–587` explicitly states "This public endpoint has no caller org context, so the validated token is the authorization grant" — confirming the tenant-scoped limiter path cannot cover this surface.

## Files
- `backend/servers/api-server/src/routes/documents/shares.rs`
- `backend/servers/api-server/src/routes/rate_limit.rs`
- `backend/servers/api-server/tests/suites/document_share_access_tests.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [x] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**
- If C4 or C5 is ticked → `local` (implementer must run on the user's Mac)
- Otherwise → `cloud-ok` (can run as a claude.ai routine via the `ppt-bridge` MCP endpoint)

Mode: cloud-ok

## Repro steps
1. Create a password-protected share (any tenant): `curl -X POST -H "$AUTH_HEADER" -d '{"password":"CorrectHorse!"}' /api/v1/documents/{id}/shares` where `$AUTH_HEADER` is a valid session token header — the response body includes `token`.
2. In a shell loop, POST `/api/v1/documents/shared/{token}/access` with `{"password":"wrong-attempt-N"}` 200 times in 60 seconds from a single client.
3. Actual today: every call returns `401 INVALID_PASSWORD` immediately, at full server throughput — brute-force is bounded only by network round-trip. No abuse signal is surfaced.
4. Expected after fix: after N=5 wrong attempts inside the rolling window, further calls to the same `{token}` return `429 RATE_LIMITED` with a generic body until the window rolls over. The correct password (once discovered) is NOT accepted while the token is in penalty (denial-of-guess). A `tracing::warn!` with `token_hash=<short-hash>` fires on the first 429.

## Suggested approach
1. In `backend/servers/api-server/src/routes/documents/shares.rs`, add two `LazyLock<InProcessRateLimiter<String>>` module statics next to the existing constants — one keyed on the share token, one keyed on the client-IP string — mirroring the split in `routes/auth/mod.rs:60–63`. Constants: `SHARE_PW_MAX_ATTEMPTS: u32 = 5`, `SHARE_PW_WINDOW: Duration = Duration::from_secs(900)`, `SHARE_PW_SWEEP_THRESHOLD: usize = 1024`.
2. Add a small helper `share_password_rate_allowed(token: &str, ip: &str) -> bool` that calls `rate_limit_allowed` twice (once per limiter) and returns `false` if *either* bucket is exhausted. Use the same `email_rate_key`-style trimmed-lowercase normalization on the IP string; the token is opaque and must be checked verbatim.
3. At the top of `access_protected_share` (`shares.rs:574`), immediately after deriving `ip_address` at line 580 and before the token lookup at ~line 586, gate with the new helper. On limit-exceeded return `Err((StatusCode::TOO_MANY_REQUESTS, Json(ErrorResponse::new("RATE_LIMITED", "Too many attempts. Please try again later."))))`. Do NOT branch on whether the token exists — checking the token first would give an attacker a fast enumeration oracle (invalid tokens 404 with no counter increment). Increment the counter *before* the DB lookup.
4. Only increment the counter on a **failed** verify (`unwrap_or(false)`) — a successful open should not consume the budget. Structurally: gate first (`false` → 429), then look up, then verify, then on `verify == false` call a `record_share_password_failure(token, ip)` helper that increments both limiters and re-issues 401. On `verify == true` skip the increment.
5. Emit `tracing::warn!(token_hash = %truncate_hash(&token), ip = %ip_address, "share password rate limit hit")` on the 429 path so ops can alert on it — do NOT log the full token.
6. Add regression tests in `backend/servers/api-server/tests/suites/document_share_access_tests.rs` (the existing share-access suite): (a) 6th wrong-password attempt on the same token inside the window returns 429; (b) correct password fed as the 6th attempt still returns 429 (denial-of-guess); (c) IP bucket applies across distinct tokens from the same IP; (d) after the window elapses, a fresh correct-password attempt returns 200.
7. Run `cd backend && SQLX_OFFLINE=true cargo test -p api-server document_share_access` and `cargo clippy -p api-server -- -D warnings`.

## Alternatives considered
- **Move the throttle into a tower layer scoped to `public_router()` instead of the handler.** — rejected because the layer would have to key on the URL-path segment `{token}` (extractable, but painful to compose with a shared limiter), and a per-handler helper keeps the pattern identical to the three existing callers (`auth::forgot_password`, `auth::resend_verification`, `mfa::verify`) — easier to review, one code path to tune.
- **Skip in-app throttling and rely on Cloudflare / nginx rate rules.** — rejected because edge rules can't bucket by `{token}` (only by IP), so a modest botnet still brute-forces a single share; dev/staging also don't sit behind the same edge and would remain vulnerable. The in-process helper closes the gap in all environments and composes with edge limits when they exist.

## Root-cause trace
1. Symptom: attacker with a share token guesses the share password at server-response speed; no abuse signal surfaces at the API surface.
2. ← `access_protected_share` at `backend/servers/api-server/src/routes/documents/shares.rs:574` verifies the password with `verify_share_password_for_share` (`shares.rs:618`) and returns 401 on failure with no counter increment.
3. ← The `public_router` at `shares.rs:27` is merged in `backend/servers/api-server/src/lib.rs:165` with no throttle layer; the app-global `host_tenant_middleware` limiter (`backend/crates/api-core/src/middleware/host_tenant.rs:159`) keys on tenant, which does not exist for this surface.
4. Origin: PAP-21 / #754 introduced the share-token public surface with the "validated token is the authorization grant" contract but did not pair it with brute-force protection — the abuse-control gap shipped alongside the feature. The `rate_limit_allowed` helper (`routes/rate_limit.rs`) was extracted later (PR against `auth/`), so at introduction time no reusable primitive existed; today it does.

## Test plan
- [ ] `tests/suites/document_share_access_tests.rs::access_protected_share_rate_limited_per_token` — 6th wrong password inside 15m window ⇒ 429 with `RATE_LIMITED` body.
- [ ] `tests/suites/document_share_access_tests.rs::access_protected_share_denies_correct_password_in_penalty` — 5 wrongs + 1 correct inside window ⇒ correct returns 429, not 200.
- [ ] `tests/suites/document_share_access_tests.rs::access_protected_share_rate_limited_per_ip` — 6 wrongs across 6 distinct tokens from same IP ⇒ 6th returns 429.
- [ ] `tests/suites/document_share_access_tests.rs::access_protected_share_window_rollover` — advance time (or use a short test-window override) past 15m ⇒ next correct attempt returns 200.
- [ ] Regression: existing shares happy-path tests still pass (they operate under-limit).
- [ ] Command: `cd backend && SQLX_OFFLINE=true cargo test -p api-server document_share_access`

## Out of scope
- CAPTCHA / proof-of-work on the share access endpoint — separate hardening.
- Redis-backed cross-instance limiter (already a known follow-up in `rate_limit.rs` module docs) — this plan uses the same in-process helper as the existing three callers.
- Auditing the parallel `routes/signatures::public_sign_router` surface for the same gap — flagged in the signal evidence; open a fresh backlog row instead of expanding this PR.
- Reworking the audit-log IP capture (recorded as the direct proxy peer, not the client IP) — tracked separately as `code-review-api-handlers-share-log-proxy-ip`.

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-share-pw-no-throttle.md`
- Mark the matching `backlog.json` row as `status: "done"`
