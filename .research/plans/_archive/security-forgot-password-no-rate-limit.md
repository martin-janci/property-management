# security-forgot-password-no-rate-limit

**Vector:** security
**Score:** 2
**Source:** hotspot in `backend/servers/api-server/src/routes/auth.rs` (Phase 1.5 code-review 2026-07-09)
**Confidence:** high

## Hypothesis
`/api/v1/auth/forgot-password` and `/api/v1/auth/resend-verification` generate + persist a token and enqueue an email on every request, with **no** per-email, per-IP, or global throttle. `check_rate_limit` is called only from `/login` (`auth.rs:555`). An attacker who knows a target's email can mailbomb their inbox (spam / phishing amplification), burn the org's email-service quota, and — because each hit invalidates prior reset tokens (`auth.rs:1477`, `invalidate_user_tokens`) — repeatedly break the target's own in-flight password resets. The intentional generic response ("If an account exists…") also removes the natural feedback signal that would surface abuse in logs. Smallest change — reuse the existing `check_rate_limit` machinery (bucket by lowercased email + client IP) at the top of both handlers with a conservative window (e.g. 5 requests / hour / email, 20 / hour / IP), returning the same generic 200 body so enumeration remains blind.

## Evidence
- `backend/servers/api-server/src/routes/auth.rs:1461` — `forgot_password` handler; no `check_rate_limit` invocation between the extractor and the DB write.
- `backend/servers/api-server/src/routes/auth.rs:555` — `check_rate_limit` is called only from `/login`; grep the file for its symbol confirms one call site.
- `backend/servers/api-server/src/routes/auth.rs:1477` — every hit calls `password_reset_repo.invalidate_user_tokens(user.id)` before minting a fresh token → serial hits from an attacker keep clobbering the legitimate user's active reset link.
- `backend/servers/api-server/src/routes/auth.rs:1466` — response body is always the generic "If an account exists…" line, so there is no visible signal to detect flooding at the API surface; abuse only shows up in the email provider's dashboard.
- `resend-verification` handler (same file, near `forgot_password`) follows the same shape: enqueues an email on every hit with no throttle.

## Files
- `backend/servers/api-server/src/routes/auth.rs:1461`
- `backend/servers/api-server/src/routes/auth.rs`
- `backend/servers/api-server/tests/auth_tests.rs`

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
1. Pick any target email `victim@example.com` known to exist in the DB.
2. In a shell loop, POST `/api/v1/auth/forgot-password` with `{ "email": "victim@example.com" }` 60 times in 60 seconds from the same IP (or from a rotating pool — the endpoint doesn't care).
3. Expected after fix: after ~5 requests within the window, the endpoint still returns the generic 200 body but no additional emails are enqueued (silent rate limit — enumeration-blind).
4. Actual today: 60 password-reset emails land in `victim@example.com`'s inbox, and any reset link the victim clicks from mail #3 was already invalidated by mail #4.

## Suggested approach
1. Read `check_rate_limit` at `auth.rs:555` and lift the current bucket key from `"login:{email}"` into a small helper `check_email_rate_limit(kind: &str, email: &str, headers: &HeaderMap, cfg: RateLimitConfig)` that composes an email bucket AND an IP bucket (use `X-Forwarded-For` first, `RemoteAddr` fallback — the same pattern the login limiter uses).
2. At the top of `forgot_password` (before the DB lookup at `auth.rs:1472`), call `check_email_rate_limit("forgot_password", &lowercased_email, &headers, cfg)`. On limit-exceeded, return the same generic 200 body — do NOT return 429; that would leak abuse feedback and reveal existence. Emit a `tracing::warn!` with a structured bucket-key so ops can alert on it.
3. Do the same at the top of `resend_verification`.
4. Add config values `AUTH_FORGOT_PW_MAX_PER_HOUR_EMAIL` (default 5), `AUTH_FORGOT_PW_MAX_PER_HOUR_IP` (default 20), `AUTH_RESEND_VERIFY_MAX_PER_HOUR_EMAIL` (default 5) with the same `dotenvy` pattern already used for `JWT_SECRET`.
5. Add regression tests in `tests/auth_tests.rs`: (a) 6th `forgot-password` for the same email inside 1h returns 200 but the outbound-email mock records only 5 sends; (b) the *legitimate user's* in-flight token remains valid after the 6th attempt (i.e. we skip `invalidate_user_tokens` when rate-limited); (c) IP bucket applies across distinct emails.
6. Run `cargo fmt`, `cargo clippy -p api-server -- -D warnings`, and the new test target.

## Alternatives considered
- **Return HTTP 429 with a "too many requests" body** — rejected because it re-introduces the enumeration signal the generic response deliberately hides. An attacker probing email addresses would see 429 on real emails and 200 on fake ones after enough traffic, defeating the anti-enumeration property.
- **Add a Cloudflare / edge rate limit instead of in-app** — rejected because edge limits can't bucket by lowercased-and-normalised email (only by IP), so single-address mailbombing from a botnet still slips through; also, dev/staging don't sit behind the same edge and would still be vulnerable.

## Root-cause trace
1. Symptom: attacker floods `victim@example.com` with password-reset mails; victim's own reset attempt from the first legitimate mail fails with `INVALID_TOKEN`.
2. ← `forgot_password` at `backend/servers/api-server/src/routes/auth.rs:1461` writes a fresh reset token on every request and calls `invalidate_user_tokens` at `auth.rs:1477`.
3. ← There is no `check_rate_limit` call in either handler (grep-verified — only `/login` at `auth.rs:555` calls it).
4. Origin: the anti-enumeration response ("always return generic 200") shipped without a matching abuse-control (`check_rate_limit` predates it in the codebase and was never wired to reset flows). No specific PR to point at — it's an omission at introduction time of the flows.

## Test plan
- [ ] `tests/auth_tests.rs::forgot_password_rate_limited_per_email` — 6th same-email request inside 1h ⇒ 200 generic body AND email mock shows exactly 5 sends AND victim's original token still validates.
- [ ] `tests/auth_tests.rs::forgot_password_rate_limited_per_ip` — 21 distinct emails from same IP inside 1h ⇒ email #21 skipped (mock shows 20 sends).
- [ ] `tests/auth_tests.rs::resend_verification_rate_limited_per_email` — mirror for the resend flow.
- [ ] Regression: existing `auth/forgot-password` tests still pass (they operate under-limit).
- [ ] Command: `cd backend && SQLX_OFFLINE=true cargo test -p api-server --test rate_limit_tests`

## Out of scope
- CAPTCHA / proof-of-work for `/forgot-password` — separate hardening; the request here is throttle-only.
- Rate limiting other unauthenticated endpoints (`/register`, `/verify-email`) — evaluate separately; this plan is scoped to the two token-minting flows identified.
- Metric export for the abuse buckets — surface via `tracing::warn!` first, dashboard follow-up.

## After-merge
- Move this file to `plans/_archive/security-forgot-password-no-rate-limit.md`
- Mark the matching `backlog.json` row as `status: "done"`
