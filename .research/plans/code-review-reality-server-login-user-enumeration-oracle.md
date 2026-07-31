# code-review-reality-server-login-user-enumeration-oracle

**Vector:** security
**Score:** 2
**Source:** review-2026-07-31 (reality-server segment, pm-backend expert)
**Confidence:** high

## Hypothesis
`reality-server`'s `login()` at `handlers/users/mod.rs:251` returns three distinguishable outcomes for the public, unauthenticated `POST /api/v1/users/login`, giving an attacker two account-enumeration oracles: (a) unknown email → immediate `"Invalid email or password"` with NO argon2 verify; (b) known email but no `password_hash` (SSO-only account) → the literal string `"Account uses SSO login"` is returned verbatim in the 401 body, disclosing that the email exists AND is SSO-bound; (c) known email + password → argon2 verify (slow) then a generic error. The distinct SSO string identifies SSO accounts directly, and the argon2/no-argon2 timing gap between (a)/(b) and (c) leaks password-account existence. This contradicts the same handler's own documented anti-enumeration convention (`mod.rs:22-24`, which collapses "user not found" for password reset). Promoted under the **security fast-track** (vector=security, confidence=high, score=2).

## Evidence
- `backend/servers/reality-server/src/handlers/users/mod.rs:251` — `login()` returns `Result<PortalUser, &'static str>`.
- `backend/servers/reality-server/src/handlers/users/mod.rs:255` — `Ok(None) => return Err("Invalid email or password")` for unknown email, no argon2.
- `backend/servers/reality-server/src/handlers/users/mod.rs:263` — `.ok_or("Account uses SSO login")?` — the enumeration oracle.
- `backend/servers/reality-server/src/routes/users.rs:210,240` — router forwards the raw `&'static str` verbatim as the 401 body (`Err((StatusCode::UNAUTHORIZED, message.to_string()))`).
- `backend/servers/reality-server/src/handlers/users/mod.rs:22` — comment: `"We deliberately collapse the 'user not found' case ... to avoid account-existence enumeration"` — the pattern to mirror.

## Files
- `backend/servers/reality-server/src/handlers/users/mod.rs:251`
- `backend/servers/reality-server/src/routes/users.rs:210`

## Dependencies
<!-- none -->

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Seed the DB with two portal users — one password-only (`user_pw@x.test`, `hunter2`) and one SSO-only (`user_sso@x.test`, no `password_hash`).
2. `curl -s -w '\n%{time_total}\n' -X POST http://localhost:8081/api/v1/users/login -H 'Content-Type: application/json' -d '{"email":"unknown@x.test","password":"x"}'` → observe body + timing.
3. Repeat with `user_sso@x.test` → observe body reads `"Account uses SSO login"` (oracle) and timing is comparable to (2).
4. Repeat with `user_pw@x.test` + wrong password → observe body reads `"Invalid email or password"` but timing is materially longer (argon2 cost).
5. Expected: identical body + identical timing across all three. Actual: two distinguishable oracles (body + timing).

## Suggested approach
1. In `handlers/users/mod.rs:263`, change the SSO branch to return `Err("Invalid email or password")` — collapse it to the generic string. Do NOT log-side leak the SSO nature to public callers (internal logs are fine).
2. In `handlers/users/mod.rs:255`, run a fixed-cost dummy argon2 verify in the unknown-email branch against a constant, precomputed hash held in a `once_cell::sync::Lazy` — this makes timing uniform between (a) and (c). Do the same in the SSO branch so (b) also spends argon2 time.
3. Extract the constant dummy hash into a `DUMMY_ARGON2_HASH` static at the top of `mod.rs` with a comment naming its purpose (timing-side-channel hardening).
4. If the internal `UserService::login` needs to distinguish SSO-vs-password for the caller (audit logging), split the error return: a public `LoginError::InvalidCredentials` (mapped to `"Invalid email or password"` in the router) and an internal `LoginError::SsoOnly` (logged, never sent). The router at `routes/users.rs:210,240` maps both to the same 401 body.

## Alternatives considered
- **Add per-account login rate limit and skip the timing fix** — rejected because rate-limit is orthogonal (it's already a separately tracked finding for the same route), and the enumeration is a *per-request* info leak that survives any rate limit ≥1. The oracle needs closing at the response layer.
- **Return `429`/`403` instead of `401` for SSO accounts** — rejected because *any* differential response teaches the attacker; the fix has to be zero-information — same body, same timing. Anything else is a downgrade.

## Root-cause trace
1. Symptom: `curl POST /api/v1/users/login` on a known SSO email returns the string `"Account uses SSO login"` — attacker learns the email exists and is SSO-bound.
2. ← `backend/servers/reality-server/src/handlers/users/mod.rs:263` — `.ok_or("Account uses SSO login")?` is the leaking string literal.
3. ← `backend/servers/reality-server/src/routes/users.rs:240` — the router forwards the `&'static str` verbatim as the 401 body without collapsing to a generic message.
4. Origin: the login handler was added before the anti-enumeration convention was formalised for password-reset (see `mod.rs:22-24` comment) — the login flow was never refactored to match.

## Test plan
- [ ] Unit test in `backend/servers/reality-server/src/handlers/users/mod.rs` (or `tests/login_enumeration_tests.rs`) that seeds an SSO-only user, calls `service.login("that_email", "wrong")`, and asserts the error string equals `"Invalid email or password"` (not `"Account uses SSO login"`).
- [ ] Integration test in `backend/servers/reality-server/tests/users_login_tests.rs` (create if absent) that POSTs `/api/v1/users/login` for (a) unknown email, (b) SSO-only email, (c) password email + wrong password and asserts all three bodies are byte-identical.
- [ ] Timing assertion: same integration test measures `std::time::Instant::elapsed` for (a) vs (c) and asserts `abs(ta - tc) < 15ms` on a warmed argon2 (the dummy-hash fix should make them near-identical).
- [ ] `cargo test -p reality-server login_enumeration_tests`.

## Out of scope
- Do NOT add or change rate limiting on `/api/v1/users/login` — tracked separately.
- Do NOT touch the password-reset flow — it already applies the pattern this plan ports over.
- Do NOT log the `"Account uses SSO login"` reason to public error tracking; server-side audit logs are fine.

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-login-user-enumeration-oracle.md`
- Mark the matching `backlog.json` row as `status: "done"`
