# code-review-reality-server-login-account-enumeration-oracle

**Vector:** security
**Score:** 2
**Source:** rotating-expert-review 2026-09-04 (reality-server segment)
**Confidence:** high

## Hypothesis
`reality-server /api/v1/users/login` returns three response bodies whose text (and timing) reveal account state to any anonymous caller: an unknown email returns `Invalid email or password` after a single sub-millisecond DB lookup (no argon2 verify); a known-but-SSO-only email (`password_hash IS NULL`) returns `Account uses SSO login` — again with no hashing — and a known password-based email with the wrong password runs a ~50-100 ms argon2 verify before returning `Invalid email or password`. Because the route hands the internal error string straight through as the HTTP 401 body, an attacker can (a) enumerate SSO-only accounts by the distinct response text and (b) enumerate password-based accounts by the response-time gap. This is the classic account-enumeration antipattern the api-server login was previously flagged for on a separate branch (`code-review-api-handlers-auth-account-enum-suspended`); the reality-server twin was never hardened. Security fast-track applies: `vector=security` + `confidence=high` + `score=2` → promotion allowed per routine Phase 3. Fix: run `verify_password` against a fixed startup-generated dummy hash on both the unknown-email and SSO-only branches, and collapse every failed-auth arm into one static `Invalid email or password` 401 body — the SSO-only case is logged internally (via `common::email_log_hash`) but never leaked to the client.

## Evidence
- `backend/servers/reality-server/src/handlers/users/mod.rs:258` — `login()`'s early `.password_hash.as_ref().ok_or("Account uses SSO login")?` returns before `verify_password` runs; the unknown-email branch (`user_repo.find_by_email(...) → None`) returns `Invalid email or password` also without invoking argon2.
- `backend/servers/reality-server/src/routes/users.rs:206` — the login route maps `Err(message)` to `Err((StatusCode::UNAUTHORIZED, message.to_string()))`, propagating the internal string verbatim as the HTTP body.
- Distinct from `code-review-reality-server-users-login-no-rate-limit` (brute-force protection / attempt counting); this finding is specifically about *response distinguishability* + *timing side channel* — orthogonal defenses.
- Related prior art: `code-review-api-handlers-auth-account-enum-suspended` on the api-server login (recorded suspended in backlog); the reality-server login shares the same shape and needs the same hardening.

## Files
- `backend/servers/reality-server/src/handlers/users/mod.rs`
- `backend/servers/reality-server/src/routes/users.rs`

## Dependencies
_(none)_

## Required capabilities
- [x] C1 — Systematic debugging (security / auth-critical)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception (security-critical change to auth path)

**Execution mode (auto-derived):** pure Rust backend edit, verifiable via `cargo test -p reality-server` in the cloud runner.

Mode: cloud-ok

## Repro steps
1. Seed three users in the reality-server DB: `alice@example.com` with a bcrypt/argon2 password_hash, `bob@example.com` with `password_hash IS NULL` (SSO-only), and no user for `nobody@example.com`.
2. Run the reality-server locally (`cargo run -p reality-server`) with logging at INFO.
3. For each of the three emails, POST `/api/v1/users/login` with `password=WrongPass!` — capture (body, latency).
4. Expected after fix: all three responses have identical body `Invalid email or password` and latencies within a narrow argon2-dominated band (~50-100 ms).
5. Actual today: `nobody@example.com` returns `Invalid email or password` in <1 ms; `bob@example.com` returns `Account uses SSO login` in <1 ms; `alice@example.com` returns `Invalid email or password` in ~50-100 ms.

## Suggested approach
1. In `backend/servers/reality-server/src/handlers/users/mod.rs`, add a `once_cell::sync::Lazy<PasswordHash>` (or `OnceLock`) holding a fixed dummy argon2 hash minted at first access; document that it is a *constant-time decoy*, not a real credential.
2. Rewrite `login()` so control flow is: `let user_opt = user_repo.find_by_email(email).await?;` → then always call `Self::verify_password(candidate, user_opt.and_then(|u| u.password_hash).as_deref().unwrap_or(&DUMMY_HASH))`. Regardless of which branch mismatches, return the same typed `LoginError::InvalidCredentials`.
3. In the SSO-only branch, keep the internal audit log — `tracing::info!(user_hint = %common::email_log_hash(email), "login attempt against SSO-only account")` — but do NOT surface that distinction to the caller.
4. Add a small enum `LoginError { InvalidCredentials, RateLimited, ... }` (or extend the existing error type) so `routes/users.rs::login` maps every variant of failed auth to a single static body `(StatusCode::UNAUTHORIZED, "Invalid email or password")`. Never `.to_string()` an internal `Err(&'static str)` through to the HTTP body from this handler.
5. Verify the `verify_password` call is the same cost on the dummy-hash path as on a real-hash path (argon2 parameters must match); check by running the timing test in step 3 of Repro before/after.
6. Do NOT remove the existing SSO-account audit trail — moving it from response-body-leak to internal-log is the whole point.
7. Update any docs mentioning the `Account uses SSO login` error text so the ppt-web / mobile-native clients don't rely on it as a UX cue (they shouldn't, but grep to confirm).

## Alternatives considered
- **Return HTTP 429 immediately on any failed login after N attempts (rate-limit only)** — rejected as insufficient: even one failed attempt already leaks the SSO/exists/timing distinctions today, and the fast-track requirement is that response *shape* be indistinguishable, not just that brute-force be slow. (Rate-limiting is tracked separately in `code-review-reality-server-users-login-no-rate-limit` and stays that item's concern.)
- **Return a slower fixed sleep on the unknown / SSO branches (`tokio::time::sleep(Duration::from_millis(75))`)** — rejected because sleep is trivially detectable by a client that measures CPU vs wall-clock, and it doesn't defend against a co-located attacker running side-channel timing; running the real argon2 primitive is the honest fix, not adding a spurious delay.

## Root-cause trace
1. Symptom: an anonymous caller can distinguish (unknown email) vs (SSO-only email) vs (password-based email with wrong password) by response body and by response latency.
2. ← `backend/servers/reality-server/src/handlers/users/mod.rs:258-277` — `login()` returns different `&'static str` messages per branch and short-circuits before `verify_password` runs on branches (a) and (b).
3. ← `backend/servers/reality-server/src/routes/users.rs:206-242` — route maps the internal `Err(message)` straight into `Err((StatusCode::UNAUTHORIZED, message.to_string()))`, propagating the leak verbatim.
4. Origin: reality-server's login handler predates the api-server hardening captured in `code-review-api-handlers-auth-account-enum-suspended`; the two logins were never unified and reality-server never got the same fix.

## Test plan
- [ ] `backend/servers/reality-server/tests/users_login_account_enum_tests.rs` — fixture with 3 users (unknown / SSO-only / password-based); assert (a) all three POST responses have identical body bytes; (b) all three call `verify_password` exactly once (spy on the seam via a repo trait mock or a test-only counter); (c) the three response latencies are within a narrow band (assert `max - min < 25 ms` on CI hardware).
- [ ] Internal-audit regression: assert `tracing::info!` is emitted with `user_hint` (an `email_log_hash`) on the SSO-only-account attempt, and that the message is NOT the raw email.
- [ ] Green-path regression: a valid `alice@example.com + correctpass` still returns 200 with the session envelope — no behaviour change on success.
- [ ] Command: `cargo test -p reality-server --test users_login_account_enum_tests` and `cargo test -p reality-server` for the full suite.

## Out of scope
- Rate-limiting or attempt-counting on the login endpoint — that belongs to `code-review-reality-server-users-login-no-rate-limit`.
- Unifying the reality-server login with the api-server login into a shared crate — a defensible refactor but a widening this PR does not need.
- Client-side UX changes (the ppt-web / mobile clients continue to render whatever body the server returns; the point is that body is now static).

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-login-account-enumeration-oracle.md`
- Mark the matching `backlog.json` row as `status: "done"`
