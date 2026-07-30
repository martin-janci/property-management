# code-review-reality-server-users-login-no-rate-limit

**Vector:** security
**Score:** 3
**Source:** tier-1d dev-review 2026-07-30 (reality-server slice 2)
**Confidence:** medium

## Hypothesis
Reality-server's three public unauthenticated auth endpoints — `POST /api/v1/users/login`, `POST /api/v1/users/register`, and `POST /api/v1/users/password-reset` — accept unlimited requests per IP and per account with no throttle, backoff, lockout, captcha, or middleware guard. Crate-wide grep for `RateLimit|Governor|tower_governor|throttle|captcha|login_attempt|lockout` finds zero handler-level or middleware protection (the only matches are unrelated: `InquiryResult::RateLimited` enum, `ErrorCategory::RateLimit` label, saved-search-alert throttles). Because reality-server is internet-facing, `login` is an open credential-stuffing / password-brute-force surface, `register` is an account-spam vector, and `password-reset` is a mail-flood / DB-flood surface (each hit issues a token). The smallest change is to reuse api-server's existing `routes::rate_limit` sliding-window limiter (or lift it into a shared crate) and add per-IP + per-account throttles in front of the three handlers, returning the same generic bodies so response-shape enumeration stays blind.

## Evidence
- `backend/servers/reality-server/src/routes/users.rs:42-52` — `Router::new()` registers `/register` (:44), `/login` (:45), `/password-reset` (:46), `/password-reset/confirm` (:47) with **no** rate-limit middleware layered.
- `backend/servers/reality-server/src/routes/users.rs:204-214` — `login()` handler calls `handler.login()` directly with no failed-attempt tracking / lockout / per-account counter.
- `backend/servers/reality-server/src/handlers/users/mod.rs:251-269` — `login()` does a plain `find_user_by_email` + `verify_password` with no failed-attempt tracking or lockout.
- `backend/servers/api-server/src/routes/rate_limit.rs:1-30` — an in-process sliding-window limiter already exists in api-server (extracted from `mfa` and `caddy_ask` per the module-doc); documented as the "single implementation" the auth surface should reuse.
- Distinct from the merged `security-forgot-password-no-rate-limit` (api-server `auth.rs::forgot_password` / `resend-verification`) — that plan hardened api-server endpoints; reality-server has its own separate auth surface that never got the same treatment.

## Files
- `backend/servers/reality-server/src/routes/users.rs`
- `backend/servers/reality-server/src/handlers/users/mod.rs`
- `backend/servers/api-server/src/routes/rate_limit.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode:** `Mode: cloud-ok`

## Repro steps
1. Start reality-server (`cd backend && cargo run -p reality-server`).
2. `for i in $(seq 1 100); do curl -s -o /dev/null -w '%{http_code}\n' -X POST http://localhost:8081/api/v1/users/login -H 'Content-Type: application/json' -d '{"email":"target@example.com","password":"guess-'$i'"}'; done`
3. Expected (after fix): once `max_attempts` is exceeded for the per-IP or per-email bucket, further requests return `429 Too Many Requests` (or a generic 401 with a `Retry-After` header) — subsequent guesses are refused before DB verify runs.
4. Actual (today): all 100 requests reach the password-verify path; response codes are 401 / 200 depending on the guess. No throttle at any layer.
5. Repeat with `/register` (account-spam) and `/password-reset` (mail-flood): same story — every request writes to DB.

## Suggested approach
1. Decide on placement: (a) lift `backend/servers/api-server/src/routes/rate_limit.rs` into a shared crate (e.g. `backend/crates/api-core` where both servers already depend on shared middleware), or (b) copy the module into `backend/servers/reality-server/src/routes/rate_limit.rs` as a temporary duplicate with a `TODO(#<issue>): promote to shared crate` comment. Prefer (a) if diff stays bounded; (b) is acceptable if it doesn't and gets a follow-up issue.
2. Wire two `InProcessRateLimiter` instances into reality-server's `AppState` (see `backend/servers/reality-server/src/state.rs`) — one keyed by client IP (extracted from `X-Forwarded-For` / `X-Real-IP` / socket peer address), one keyed by lowercased email.
3. At the top of `login()`, `register()`, and `request_password_reset()` in `handlers/users/mod.rs` (or in a thin `.layer(...)`-style middleware in `routes/users.rs`), call `rate_limit_allowed(&limiter_ip, ip_key)` and `rate_limit_allowed(&limiter_email, email_key)` before the DB lookup. On block, return the same generic response shape the handler already returns (200 with the "if account exists" body for password-reset; 401 for login; 400 for register) to preserve enumeration-blindness. Add a `Retry-After` header on 429 for well-behaved clients.
4. Windows: start conservative — 5 requests / hour / email, 20 / hour / IP for login and password-reset; 10 registrations / hour / IP.
5. Emit `tracing::warn!` on block with `key`, `endpoint`, `rate_limit_key_type` so ops has a signal.
6. Add tests (see *Test plan*).
7. Run `cargo test -p reality-server`, `cargo clippy -p reality-server --all-targets -- -D warnings`.

## Alternatives considered
- **Redis-backed distributed limiter (via `sessions` Redis already in the stack)** — rejected as the shipped V1: introduces a Redis round-trip on every auth request and the follow-up is already tracked in `routes/rate_limit.rs`'s module doc ("A Redis-backed counter would hold the limit across instances — tracked as a follow-up"). Ship the in-process limiter first; promote to Redis in a follow-up when horizontal scaling matters.
- **Tower middleware (`tower_governor`)** — rejected because the api-server already ships a reviewed in-process limiter with the exact semantics needed (per-key sliding window with amortised cleanup); adding a second, differently-configured throttle library creates two failure modes to reason about.

## Root-cause trace
1. Symptom: unlimited login attempts succeed against `POST /api/v1/users/login` — no throttle response.
2. ← `backend/servers/reality-server/src/routes/users.rs:45` — the `.route("/login", post(login))` layer chain has no rate-limit middleware.
3. ← `backend/servers/reality-server/src/handlers/users/mod.rs:251-269` — the handler calls `verify_password` immediately with no failed-attempt tracking.
4. ← Reality-server was spun off from api-server without inheriting the auth-surface rate-limit hardening that later landed in api-server (`routes/rate_limit.rs`, `routes/auth.rs::forgot_password`).
5. Origin: reality-server auth-router creation commit (predates `routes/rate_limit.rs` extraction).

## Test plan
- [ ] Add integration tests in `backend/servers/reality-server/tests/suites/users_authz_tests.rs` (or a new `users_rate_limit_tests.rs` suite) covering:
  - Login: N+1 attempts from the same IP against different emails → last one returns 429 (or generic 401 with `Retry-After`).
  - Login: N+1 attempts against the same email from different IPs → last one blocked on the per-email bucket.
  - Password-reset: N+1 requests for the same email → last one returns generic 200 body but does NOT persist a new token (verify via repo call count / DB row count).
  - Register: N+1 registrations from the same IP → last one returns 400 with no DB write.
  - Positive: a single well-behaved request in each shape still succeeds after the block window rolls over (mocked clock or `Duration::from_millis(1)` window).
- [ ] Command: `cd backend && cargo test -p reality-server users_rate_limit`.
- [ ] Regression: `cd backend && cargo test -p reality-server` (full crate); `cargo test -p api-server routes::rate_limit` (verify shared module still passes if lifted).

## Out of scope
- Redis-backed distributed rate limiting — separate follow-up.
- Captcha / MFA / passkey enrolment on register or password-reset.
- Rate-limiting the authenticated `/me`, `/logout` handlers — different threat profile, different limits.
- Applying the same treatment to api-server routes that already have their own limits.

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-users-login-no-rate-limit.md`
- Mark the matching `backlog.json` row as `status: "done"`
