# code-review-api-handlers-share-pw-no-throttle

**Vector:** security
**Score:** 2
**Source:** Tier1d review 2026-08-15 (api-handlers) — file:line `backend/servers/api-server/src/routes/documents/shares.rs:574`
**Confidence:** high

## Hypothesis
`POST /api/v1/documents/shared/{token}/access` verifies a caller-supplied share
password with zero brute-force protection: no failed-attempt counter, no
per-token/per-IP lockout, no throttle. An attacker holding a share token can
brute-force the share password at request speed. The tenant-keyed limiter
covering the rest of the API cannot key this endpoint because it is unauthenticated
and outside any org context. Adding a per-token (and per-IP fallback)
sliding-window limiter on this exact handler — using the same in-process
primitive already shared by `auth/mod.rs`, `mfa.rs`, and `voice_webhooks.rs` —
closes the gap with a small, review-friendly change.

## Evidence
- `backend/servers/api-server/src/routes/documents/shares.rs:574` — `access_protected_share` handler (public, mounted at `POST /api/v1/documents/shared/{token}/access` via `shares::public_router`).
- `backend/servers/api-server/src/routes/documents/shares.rs:618` — `verify_share_password_for_share(share.id, &req.password)` — the password check with no attempt counter before or after.
- `grep -rEn 'rate.?limit|throttle|attempt|lockout' backend/servers/api-server/src/routes/documents/` returns 0 hits (verified 2026-08-15).
- The only rate limiter live in `api-server` today is `state.tenant_rate_limiters: TenantRateLimiterSet`, keyed via `host_tenant_middleware` on **org** context. Handler comment at shares.rs:582 explicitly acknowledges "This public endpoint has no caller org context" — so tenant limiter cannot cover it.
- The reusable primitive already exists at `backend/servers/api-server/src/routes/rate_limit.rs` (`InProcessRateLimiter<K>` + `rate_limit_allowed`) and is in use by `routes::auth::mod` (email-keyed) and `routes::mfa` / `routes::voice_webhooks` (Uuid-keyed).

## Files
- `backend/servers/api-server/src/routes/documents/shares.rs`
- `backend/servers/api-server/src/routes/rate_limit.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (security-relevant bug, cross-file wiring)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception (security-labelled)

Mode: cloud-ok

## Repro steps
1. Create a password-protected share (via `POST /api/v1/documents/{doc}/shares` with a `password`); grab the returned `token`.
2. In a loop from an anonymous client: `curl -s -o /dev/null -w '%{http_code}\n' -X POST -H 'Content-Type: application/json' -d '{"password":"guess-N"}' http://localhost:8080/api/v1/documents/shared/<token>/access`.
3. **Expected after fix:** after N failed attempts within the window (proposal: 10 attempts per 15 min per token, mirrored per source-IP), the handler returns `429 TOO_MANY_REQUESTS` before ever calling `verify_share_password_for_share`.
4. **Actual on `main`:** every request runs the bcrypt/Argon verify path with no counter — brute-force is bounded only by network throughput.

## Suggested approach
1. Reuse `crate::routes::rate_limit::{rate_limit_allowed, InProcessRateLimiter}` (exact same primitive as `routes::mfa`).
2. In `backend/servers/api-server/src/routes/documents/shares.rs`, add two `LazyLock<InProcessRateLimiter<_>>` statics at module scope: one keyed by `share_token: String` (primary), one keyed by `source_ip: IpAddr` (fallback for token enumeration).
3. In `access_protected_share` (shares.rs:574), gate the entire body on both `rate_limit_allowed(&SHARE_ACCESS_TOKEN_LIMITER, token.clone(), 10, Duration::from_secs(900), 1024)` AND `rate_limit_allowed(&SHARE_ACCESS_IP_LIMITER, addr.ip(), 30, Duration::from_secs(900), 1024)`. When either returns `false`, return `(StatusCode::TOO_MANY_REQUESTS, Json(ErrorResponse::new("RATE_LIMITED", "Too many attempts; try again later")))` **before** any DB lookup.
4. Increment the counters on every incoming request (successful or not) — a successful password unlock still counts against the token limit so a compromise cannot be laundered by racing legitimate accesses.
5. Update the `#[utoipa::path(…)]` annotation on `access_protected_share` to list `(status = 429, description = "Too many attempts")` in `responses`.
6. Mirror the same limiter on the sibling `access_public_share` path at shares.rs:437 if it also runs unauthenticated (verify during implementation; if it already uses a password-less flow, only the token-scoped limiter applies).
7. Add regression tests to `backend/servers/api-server/tests/suites/` — a new file `share_password_throttle_tests.rs`: seed a password-protected share, hammer with wrong passwords, assert the 11th within the window returns 429 and the 12th also returns 429 without touching the password verifier (assert via monotonic counter on a stub verifier, or via a query on `share_access_log` counts).

## Alternatives considered
- **Redis-backed counter (shared across instances)** — rejected because the module's own docs (`routes/rate_limit.rs:19`) already call out Redis as a known follow-up, but the current in-process primitive is what every other auth-surface route uses; matching them keeps the review surface small. The token brute-force cap is orders of magnitude below what would justify introducing a Redis dependency on this path first.
- **Rely on Cloudflare / nginx-level rate limits at the edge** — rejected because we don't own the proxy layer uniformly across environments (mefistos vs Hetzner), and application-layer defence-in-depth is standard for password-verify endpoints. The existing `auth/forgot-password` limiter proves the repo convention is to enforce this in Rust.

## Root-cause trace
1. Symptom: unauthenticated attacker guessing share passwords is never throttled — every request runs the crypto verify.
2. ← `access_protected_share` at `backend/servers/api-server/src/routes/documents/shares.rs:574` — no attempt counter around `verify_share_password_for_share`.
3. ← Handler comment shares.rs:582 correctly identifies that the endpoint has no org context; the code stopped there without adding a token-scoped counter to compensate for the tenant limiter's inapplicability.
4. Origin: share endpoints predate the `routes::rate_limit` extraction (which was done for `mfa` + `auth`); the primitive was never back-ported to `documents/shares.rs`.

## Test plan
- [ ] New file `backend/servers/api-server/tests/suites/share_password_throttle_tests.rs` — must be `#[mod path]`'d into the appropriate test-shard aggregator (see `tests/suite_*.rs`).
- [ ] Case A: 10 wrong-password POSTs to `/api/v1/documents/shared/{token}/access` within window all return 401. The 11th returns 429. Assert the 429 response body carries `"code":"RATE_LIMITED"`.
- [ ] Case B: after 15 minutes (simulate by mutating `SHARE_ACCESS_TOKEN_LIMITER` window in a helper — mirror the pattern in `routes::mfa` tests), the counter resets and a correct password unlocks again.
- [ ] Case C: IP-limit fallback — from the same source IP, hammer 30 requests across 3 different valid tokens; the 31st (any token) returns 429 even if it is the first request for that specific token.
- [ ] Local command: `cargo test -p api-server share_password_throttle`

## Out of scope
- Redis-backed cross-instance limiter (tracked as the pre-existing follow-up in `routes/rate_limit.rs:19`).
- Reviewing the sibling `signatures::public_sign_router` for the same gap — that is a distinct handler and deserves its own signal/plan (surface as a new backlog row if identified during implementation).
- Changing the underlying password-hash / verify algorithm.
- Making the response body reveal the remaining-attempts counter (avoid leaking the limiter's window to attackers).

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-share-pw-no-throttle.md`
- Mark the matching `backlog.json` row (`id: code-review-api-handlers-share-pw-no-throttle`) as `status: "done"`
