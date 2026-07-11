# bug-revoke-all-sessions-cookie-blindness

**Vector:** bug
**Score:** 3
**Source:** hotspot in `backend/servers/api-server/src/routes/auth.rs` (Phase 1.5 code-review 2026-07-09)
**Confidence:** high

## Hypothesis
After the P0-12 cookie migration, ppt-web sends the refresh token only via the `HttpOnly` `refresh_token` cookie — no `X-Refresh-Token` header. `revoke_all_sessions` (`auth.rs:2013`) still reads only the header, so cookie-based callers arrive with `current_session_id = None` and `revoke_all_user_tokens(user_id, None)` revokes *every* session, including the caller's live one. The user asks to "sign out other devices" and is themselves signed out. `list_sessions` (`auth.rs:1821`) has the identical regression: it can never mark any session `isCurrent = true` for cookie clients. Smallest fix — extend both handlers with the same cookie-first / header-fallback lookup that `refresh_token` and `logout` already use via `parse_refresh_cookie` (auth.rs:1077).

## Evidence
- `backend/servers/api-server/src/routes/auth.rs:2013` — `if let Some(refresh_token) = headers.get("X-Refresh-Token").and_then(|h| h.to_str().ok())` is the only source; `parse_refresh_cookie(&headers)` is never called.
- `backend/servers/api-server/src/routes/auth.rs:2031` — result path calls `revoke_all_user_tokens(user_id, current_session_id)` where a `None` value causes the caller's session to be revoked.
- `backend/servers/api-server/src/routes/auth.rs:1077` — `refresh_token` handler already demonstrates the correct pattern: `let cookie_token = parse_refresh_cookie(&headers); let token_str = cookie_token.as_deref().unwrap_or(req.refresh_token.as_str());`.
- `backend/servers/api-server/src/routes/auth.rs:1821` — `list_sessions` has the same header-only branch, so `isCurrent` is always false for cookie clients.

## Files
- `backend/servers/api-server/src/routes/auth.rs:2013`
- `backend/servers/api-server/src/routes/auth.rs:1821`
- `backend/servers/api-server/tests/auth_tests.rs`

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
- If C4 or C5 is ticked → `local` (implementer must run on the user's Mac)
- Otherwise → `cloud-ok` (can run as a claude.ai routine via the `ppt-bridge` MCP endpoint)

Mode: cloud-ok

## Repro steps
1. Log a user in via ppt-web (cookie-based, HttpOnly `refresh_token` set; no localStorage token).
2. Open a second browser (or incognito) and log the same user in — two live sessions now exist for that user.
3. Back in the first browser, hit `POST /api/v1/auth/sessions/revoke-all` with just the access-token bearer header (no `X-Refresh-Token`).
4. Expected: exactly 1 session revoked, the current browser stays logged in. Actual: 2 sessions revoked, next request from either browser returns 401 `REFRESH_TOKEN_REVOKED`. Similarly `GET /api/v1/auth/sessions` returns `isCurrent: false` for every row.

## Suggested approach
1. Add a private helper `resolve_current_session_id(state: &AppState, headers: &HeaderMap) -> Option<Uuid>` above `revoke_all_sessions` that prefers `parse_refresh_cookie(headers)`, falls back to `headers.get("X-Refresh-Token")`, hashes the token, and returns `session_repo.find_by_token_hash(&hash).await.ok().flatten().map(|s| s.id)`.
2. Replace the inline block at `auth.rs:2011–2026` with `let current_session_id = resolve_current_session_id(&state, &headers).await;` — behavior for header-only callers is unchanged (the fallback still runs).
3. Do the same substitution inside `list_sessions` at `auth.rs:1821` so `isCurrent` is computed against the actual current session (cookie or header).
4. Add regression tests in `tests/auth_tests.rs`: (a) with cookie only, `revoke_all_sessions` leaves the caller's session alive; (b) with header only, existing behavior preserved; (c) with neither, still revokes all (documented shape); (d) `list_sessions` returns `isCurrent = true` for the cookie caller.
5. Run `cargo fmt`, `cargo clippy -p api-server -- -D warnings`, `cargo test -p api-server --test session_management_tests`.
6. No migration, no public API change, no OpenAPI regen needed — only the current-session detection is fixed.

## Alternatives considered
- **Change `revoke_all_user_tokens(user_id, None)` to be a no-op / error out** — rejected because the `None` branch is deliberate (used by session-management admin flows and by tests where no specific session is being preserved). Fixing the *caller* keeps the repository contract intact and doesn't ripple.
- **Deprecate `X-Refresh-Token` and require the cookie** — rejected because localStorage-based clients (mobile RN, older ppt-web builds still in the field) still rely on the header path; this is a fix, not a migration.

## Root-cause trace
1. Symptom: user hits "Sign out other devices" in ppt-web, then their own tab returns 401 on the next authenticated request.
2. ← `revoke_all_user_tokens(user_id, current_session_id)` at `backend/servers/api-server/src/routes/auth.rs:2031` receives `current_session_id = None`.
3. ← `current_session_id` at `backend/servers/api-server/src/routes/auth.rs:2012–2026` is derived exclusively from `headers.get("X-Refresh-Token")`, which is absent on cookie-based clients after P0-12.
4. Origin: P0-12 cookie migration (search: `parse_refresh_cookie` first landed in the tree). `refresh_token` (line 1077) and `logout` (~line 1385) were updated to prefer the cookie; `revoke_all_sessions` and `list_sessions` were missed.

## Test plan
- [ ] `tests/auth_tests.rs::revoke_all_sessions_preserves_cookie_caller` — cookie-only request, one other session, assert exactly 1 revoked and caller's session still valid on subsequent `/me`.
- [ ] `tests/auth_tests.rs::list_sessions_marks_cookie_caller_current` — cookie-only request, assert exactly one row has `isCurrent = true` and it matches the caller's session.
- [ ] Regression: existing header-based tests must still pass (`cargo test -p api-server --test session_management_tests`).
- [ ] `cargo clippy -p api-server -- -D warnings` → exit 0.
- [ ] Command: `cd backend && SQLX_OFFLINE=true cargo test -p api-server --test session_management_tests`

## Out of scope
- Rotating the refresh cookie value on revoke-all (session-fixation hardening) — separate hardening plan.
- Adding `X-Refresh-Token` deprecation warnings — that belongs to the P0-12 follow-up.
- Rate limiting on `revoke_all_sessions` — separate concern.

## After-merge
- Move this file to `plans/_archive/bug-revoke-all-sessions-cookie-blindness.md`
- Mark the matching `backlog.json` row as `status: "done"`
